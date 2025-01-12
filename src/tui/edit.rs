use colored::Colorize;
use std::path::PathBuf;

use crate::actions::{Action, DescribedAction};
use crate::configs::DeployerGlobalConfig;
use crate::entities::custom_command::{CustomCommand, specify_bash_c};
use crate::entities::programming_languages::{ProgrammingLanguage, specify_programming_languages};
use crate::entities::targets::{TargetDescription, OsVariant, OsVersionSpecification};
use crate::entities::traits::{Edit, EditExtended};
use crate::entities::variables::{Variable, VarValue};
use crate::hmap;
use crate::i18n;
use crate::pipelines::DescribedPipeline;
use crate::tui::add::{collect_af_inplacement, collect_artifact};
use crate::utils::tags_custom_type;

impl EditExtended<DeployerGlobalConfig> for Vec<DescribedAction> {
  fn edit_from_prompt(&mut self, opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = i18n::ACTION_EDIT.replace("{1}", &c.title).replace("{2}", &c.info.to_str());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::REORDER.to_string(), i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::ACTION_SELECT_TO_CHANGE, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::REORDER => self.reorder(opts)?,
          i18n::ADD => self.add_item(opts)?,
          i18n::REMOVE => self.remove_item(opts)?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_action_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self, _opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    use inquire::ReorderableList;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for selected in self.iter() {
      let key = i18n::ACTION.replace("{1}", &selected.title).replace("{2}", &selected.info.to_str());
      k.push(key.clone());
      h.insert(key, selected);
    }
    
    let reordered = ReorderableList::new(i18n::CMDS_REORDER, k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self, opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    use inquire::Select;
    
    const USE_ANOTHER: &str = i18n::ACTION_SPECIFY_ANOTHER;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for action in opts.actions_registry.values() {
      let key = i18n::ACTION.replace("{1}", &action.title).replace("{2}", &action.info.to_str());
      k.push(key.clone());
      h.insert(key, action);
    }
    
    k.push(USE_ANOTHER.to_string());
    
    let selected = Select::new(i18n::ACTION_CHOOSE_TO_ADD, k).prompt()?;
    
    if selected.as_str() == USE_ANOTHER {
      if let Ok(action) = DescribedAction::new_from_prompt(opts) {
        self.push(action);
      }
    } else {
      self.push((**h.get(&selected).ok_or(anyhow::anyhow!("Can't get specified Action!"))?).clone());
    }
    Ok(())
  }
  
  fn remove_item(&mut self, _opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = i18n::ACTION_REMOVE.replace("{1}", &c.title).replace("{2}", &c.info.to_str());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::ACTION_CHOOSE_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    
    Ok(())
  }
}

impl DescribedAction {
  pub(crate) fn edit_action_from_prompt(&mut self) -> anyhow::Result<()> {
    let mut actions = vec![];
    match &self.action {
      Action::Custom(_) | Action::Observe(_) => { actions.push(i18n::EDIT_COMMAND); },
      Action::Check(_) => { actions.extend_from_slice(&[i18n::EDIT_COMMAND, i18n::CHECK_EDIT_REGEXES]); },
      Action::PreBuild(_) | Action::Build(_) | Action::PostBuild(_) | Action::Test(_) => {
        actions.extend_from_slice(&[i18n::EDIT_COMMANDS, i18n::EDIT_PLS]);
      },
      Action::Pack(_) | Action::Deliver(_) | Action::Install(_) => {
        actions.extend_from_slice(&[i18n::EDIT_COMMANDS, i18n::EDIT_TARGETS]);
      },
      Action::ConfigureDeploy(_) | Action::Deploy(_) | Action::PostDeploy(_) => {
        actions.extend_from_slice(&[i18n::EDIT_COMMANDS, i18n::EDIT_DEPL_TOOLKIT]);
      },
      Action::Patch(_) => { actions.push(i18n::EDIT_PATCH); },
      Action::AddToStorage(_) => { actions.push(i18n::EDIT_ATS); }
      Action::Interrupt | Action::ForceArtifactsEnplace | Action::UseFromStorage(_) => {},
    }
    actions.extend_from_slice(&[
      i18n::EDIT_TITLE,
      i18n::EDIT_DESC,
      i18n::EDIT_TAGS,
    ]);
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        i18n::EDIT_TITLE => self.title = inquire::Text::new(i18n::ACTION_FULL_NAME).with_initial_value(self.title.as_str()).prompt()?,
        i18n::EDIT_DESC => self.desc = inquire::Text::new(i18n::ACTION_DESC).with_initial_value(self.desc.as_str()).prompt()?,
        i18n::EDIT_TAGS => {
          let joined = self.tags.join(", ");
          self.tags = tags_custom_type(i18n::ACTION_TAGS, if joined.is_empty() { None } else { Some(joined.as_str()) }).prompt()?
        },
        i18n::EDIT_COMMAND => {
          if let Action::Custom(cmd) = &mut self.action {
            cmd.edit_command_from_prompt()?;
          } else if let Action::Observe(o_command) = &mut self.action {
            o_command.command.edit_command_from_prompt()?;
          } else if let Action::Check(c_command) = &mut self.action {
            c_command.command.edit_command_from_prompt()?;
          }
        },
        i18n::EDIT_ATS if let Action::AddToStorage(a) = &mut self.action => a.edit_from_prompt()?,
        i18n::EDIT_COMMANDS => {
          match &mut self.action {
            Action::PreBuild(a) => a.commands.edit_from_prompt()?,
            Action::Build(a) => a.commands.edit_from_prompt()?,
            Action::PostBuild(a) => a.commands.edit_from_prompt()?,
            Action::Test(a) => a.commands.edit_from_prompt()?,
            Action::Pack(a) => a.commands.edit_from_prompt()?,
            Action::Deliver(a) => a.commands.edit_from_prompt()?,
            Action::Install(a) => a.commands.edit_from_prompt()?,
            Action::ConfigureDeploy(a) => a.commands.edit_from_prompt()?,
            Action::Deploy(a) => a.commands.edit_from_prompt()?,
            Action::PostDeploy(a) => a.commands.edit_from_prompt()?,
            Action::Check(a) => a.edit_check_from_prompt()?,
            Action::Observe(a) => a.command.edit_command_from_prompt()?,
            Action::Custom(a) => a.edit_command_from_prompt()?,
            Action::Interrupt | Action::ForceArtifactsEnplace | Action::Patch(_) | Action::UseFromStorage(_) | Action::AddToStorage(_) => {},
          }
        },
        i18n::CHECK_EDIT_REGEXES if let Action::Check(c_action) = &mut self.action => c_action.change_regexes_from_prompt()?,
        i18n::EDIT_PLS => {
          match &mut self.action {
            Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => {
              a.supported_langs = specify_programming_languages()?;
            },
            _ => {},
          }
        },
        i18n::EDIT_TARGETS => {
          match &mut self.action {
            Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => {
              a.target = Some(TargetDescription::new_from_prompt()?);
            },
            _ => {},
          }
        },
        i18n::EDIT_DEPL_TOOLKIT => {
          match &mut self.action {
            Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => {
              a.deploy_toolkit = inquire::Text::new(&format!("{} {}:", i18n::DEPL_TOOLKIT, i18n::HIT_ESC)).prompt_skippable()?;
            },
            _ => {},
          }
        },
        i18n::EDIT_PATCH if let Action::Patch(patch) = &mut self.action => { patch.edit_from_prompt()?; },
        _ => {},
      }
    }
    
    Ok(())
  }
}

impl DescribedPipeline {
  pub(crate) fn edit_pipeline_from_prompt(&mut self, globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let actions = vec![
      i18n::EDIT_PIPELINE_ACTIONS,
      i18n::EDIT_TITLE,
      i18n::EDIT_DESC,
      i18n::EDIT_TAGS,
      i18n::EDIT_EXCL_TAG,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        i18n::EDIT_TITLE => self.title = inquire::Text::new(i18n::PIPELINE_FULL_NAME).with_initial_value(self.title.as_str()).prompt()?,
        i18n::EDIT_DESC => self.desc = inquire::Text::new(i18n::PIPELINE_DESC).with_initial_value(self.desc.as_str()).prompt()?,
        i18n::EDIT_TAGS => {
          let joined = self.tags.join(", ");
          self.tags = tags_custom_type(i18n::PIPELINE_TAGS, if joined.is_empty() { None } else { Some(joined.as_str()) }).prompt()?
        },
        i18n::EDIT_PIPELINE_ACTIONS => self.actions.edit_from_prompt(globals)?,
        i18n::EDIT_EXCL_TAG => self.exclusive_exec_tag = if self.exclusive_exec_tag.is_none() {
          inquire::Text::new(&format!("{} {}:", i18n::PIPELINE_SPECIFY_EXCL_TAG, i18n::OR_HIT_ESC)).prompt_skippable()?
        } else {
          inquire::Text::new(
            &format!("{} {}:", i18n::PIPELINE_SPECIFY_EXCL_TAG, i18n::OR_HIT_ESC)
          ).with_initial_value(self.exclusive_exec_tag.as_ref().unwrap()).prompt_skippable()?
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

impl EditExtended<DeployerGlobalConfig> for Vec<DescribedPipeline> {
  fn edit_from_prompt(&mut self, opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = i18n::PIPELINE_EDIT.replace("{1}", &c.title).replace("{2}", &c.info.to_str());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::REORDER.to_string(), i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::SELECT_PIPELINE_TO_CHANGE, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::REORDER => self.reorder(opts)?,
          i18n::ADD => self.add_item(opts)?,
          i18n::REMOVE => self.remove_item(opts)?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_pipeline_from_prompt(opts)?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self, _opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    use inquire::ReorderableList;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for selected in self.iter() {
      let key = i18n::PIPELINE.replace("{1}", &selected.title).replace("{2}", &selected.info.to_str());
      k.push(key.clone());
      h.insert(key, selected);
    }
    
    let reordered = ReorderableList::new(i18n::PIPELINE_REORDER_ACTIONS, k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self, opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    use inquire::Select;
    
    const USE_ANOTHER: &str = i18n::PIPELINE_SPECIFY_ANOTHER;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for pipeline in opts.pipelines_registry.values() {
      let key = i18n::PIPELINE.replace("{1}", &pipeline.title).replace("{2}", &pipeline.info.to_str());
      k.push(key.clone());
      h.insert(key, pipeline);
    }
    
    k.push(USE_ANOTHER.to_string());
    
    let selected = Select::new(i18n::PIPELINE_CHOOSE_TO_ADD, k).prompt()?;
    
    if selected.as_str() == USE_ANOTHER {
      if let Ok(pipeline) = DescribedPipeline::new_from_prompt(opts) {
        self.push(pipeline);
      }
    } else {
      self.push((**h.get(&selected).ok_or(anyhow::anyhow!("Can't get specified Pipeline!"))?).clone());
    }
    Ok(())
  }
  
  fn remove_item(&mut self, _opts: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = i18n::PIPELINE_REMOVE.replace("{1}", &c.title).replace("{2}", &c.info.to_str());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::PIPELINE_CHOOSE_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    
    Ok(())
  }
}

impl Edit for Vec<PathBuf> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::ENTITY, c.to_string_lossy());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(collect_artifact()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("{} `{}`", i18n::ENTITY, c.to_string_lossy());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::VALUE_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

impl EditExtended<Vec<PathBuf>> for Vec<(PathBuf, PathBuf)> {
  fn edit_from_prompt(&mut self, opts: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}` -> `{}`", i18n::INPLACEMENT, c.0.to_string_lossy(), c.1.to_string_lossy());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item(opts)?,
          i18n::REMOVE => self.remove_item(opts)?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self, _opts: &mut Vec<PathBuf>) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self, opts: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let v = opts.iter().map(|v| v.to_string_lossy()).collect::<Vec<_>>();
    self.push(collect_af_inplacement(&v)?);
    Ok(())
  }
  
  fn remove_item(&mut self, _opts: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("{} `{}` -> `{}`", i18n::INPLACEMENT, c.0.to_string_lossy(), c.1.to_string_lossy());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::REMOVE_INPLACEMENT, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

impl Edit for Vec<TargetDescription> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::EDIT_TARGET, c.to_string().green());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(
        &format!("{} {}:", i18n::SELECT_TARGET_TO_CHANGE, i18n::HIT_ESC),
        cs,
      ).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_target_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(TargetDescription::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("{} `{}`", i18n::TARGET, c.to_string().green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::SELECT_TARGET_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

impl Variable {
  pub(crate) fn edit_variable_from_prompt(&mut self) -> anyhow::Result<()> {
    let actions = vec![
      i18n::EDIT_TITLE,
      i18n::EDIT_VAR_SECRET,
      i18n::EDIT_VALUE,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        i18n::EDIT_TITLE => self.title = inquire::Text::new(i18n::VAR_TITLE).prompt()?,
        i18n::EDIT_VAR_SECRET => self.is_secret = inquire::Confirm::new(i18n::VAR_IS_SECRET).with_default(false).prompt()?,
        i18n::EDIT_VALUE => self.value = VarValue::Plain(inquire::Text::new(i18n::VAR_CONTENT).prompt()?),
        _ => {},
      }
    }
    
    Ok(())
  }
}

impl Edit for Vec<Variable> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::VAR_EDIT, c.title.green());
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::VAR_SELECT_FC, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_variable_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(Variable::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("{} `{}`", i18n::VAR, c.title.green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::VAR_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}

impl TargetDescription {
  pub(crate) fn edit_target_from_prompt(&mut self) -> anyhow::Result<()> {
    let actions = vec![
      i18n::EDIT_ARCH,
      i18n::EDIT_OS,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      use inquire::{Select, Text};
      
      match action {
        i18n::EDIT_ARCH => self.arch = Text::new(i18n::TARGET_ARCH).prompt()?,
        i18n::EDIT_OS => {
          let os = Select::new(
            i18n::TARGET_OS_SELECT,
            vec!["Android", "iOS", "Linux", "Unix-like", "Windows", "macOS", "Other"]
          ).prompt()?;
          
          self.os = match os {
            "Android" => OsVariant::Android,
            "iOS" => OsVariant::iOS,
            "Linux" => OsVariant::Linux,
            "Unix-like" => {
              let name = Text::new(i18n::TARGET_OS_UNIX_LIKE).prompt()?;
              OsVariant::UnixLike(name)
            },
            "Windows" => OsVariant::Windows,
            "macOS" => OsVariant::macOS,
            "Other" => {
              let name = Text::new(i18n::TARGET_OS_OTHER).prompt()?;
              OsVariant::Other(name)
            },
            _ => unreachable!(),
          };
          
          self.derivative = Text::new(i18n::TARGET_OS_DER).prompt()?;
          
          let version_type = Select::new(
            i18n::TARGET_OS_VER_S,
            vec![i18n::TARGET_OS_VER_NS, i18n::TARGET_OS_VER_WS, i18n::TARGET_OS_VER_SS]
          ).prompt()?;
          
          self.version = match version_type {
            i18n::TARGET_OS_VER_NS => OsVersionSpecification::No,
            i18n::TARGET_OS_VER_WS => {
              let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
              OsVersionSpecification::Weak(ver)
            },
            i18n::TARGET_OS_VER_SS => {
              let ver = Text::new(i18n::TARGET_OS_VER).prompt()?;
              OsVersionSpecification::Strong(ver)
            },
            _ => unreachable!(),
          };
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

impl CustomCommand {
  pub(crate) fn edit_command_from_prompt(&mut self) -> anyhow::Result<()> {
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::CMD_SELECT_TO_CHANGE.replace("{}", &self.bash_c.green()), i18n::HIT_ESC),
      vec![
        i18n::CMD_EDIT_SHELL,
        i18n::CMD_CHANGE_PLACEHOLDERS,
        i18n::CMD_CHANGE_FAILURE_IGNORANCE,
        i18n::CMD_CHANGE_VISIBILITY_AT_BUILD,
        i18n::CMD_CHANGE_VISIBILITY_ON_SUCC,
        i18n::CMD_CHANGE_ON_FRESH,
      ],
    ).prompt_skippable()? {
      match action {
        i18n::CMD_EDIT_SHELL => self.bash_c = specify_bash_c(Some(self.bash_c.as_str()))?,
        i18n::CMD_CHANGE_PLACEHOLDERS => {
          let placeholders = if let Some(phs) = &self.placeholders {
            let joined = phs.join(", ");
            tags_custom_type(i18n::CMD_PLACEHOLDERS, Some(joined.as_str())).prompt()?
          } else {
            tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?
          };
          self.placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
        },
        i18n::CMD_CHANGE_FAILURE_IGNORANCE => {
          self.ignore_fails = inquire::Confirm::new(i18n::CMD_IGNORE_FAILS).with_default(false).prompt()?;
        },
        i18n::CMD_CHANGE_VISIBILITY_AT_BUILD => {
          self.show_bash_c = inquire::Confirm::new(i18n::CMD_SHOW_BASH_C).with_default(true).prompt()?;
        },
        i18n::CMD_CHANGE_VISIBILITY_ON_SUCC => {
          self.show_success_output = inquire::Confirm::new(i18n::CMD_SHOW_SUCC_OUT).with_default(false).prompt()?;
        },
        i18n::CMD_CHANGE_ON_FRESH => {
          self.only_when_fresh = if inquire::Confirm::new(i18n::CMD_ONLY_WHEN_FRESH).with_default(false).prompt()? {
            Some(true)
          } else {
            None
          };
        },
        _ => {},
      }
    }
    
    Ok(())
  }
}

impl Edit for Vec<CustomCommand> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::CUSTOM_CMD_EDIT, c.bash_c.green());
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::CUSTOM_CMD_REORDER.to_string(), i18n::CUSTOM_CMD_ADD.to_string(), i18n::CUSTOM_CMD_RM.to_string()]);
      
      if let Some(action) = inquire::Select::new(
        &format!("{} {}:", i18n::CUSTOM_CMD_EDIT_PROMPT, i18n::HIT_ESC),
        cs,
      ).prompt_skippable()? {
        match action.as_str() {
          i18n::CUSTOM_CMD_REORDER => self.reorder()?,
          i18n::CUSTOM_CMD_ADD => self.add_item()?,
          i18n::CUSTOM_CMD_RM => self.remove_item()?,
          s if cmap.contains_key(s) => cmap.get_mut(s).unwrap().edit_command_from_prompt()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> {
    use inquire::ReorderableList;
    
    let mut h = hmap!();
    let mut k = vec![];
    
    for selected_command in self.iter() {
      let key = format!("`{}`", selected_command.bash_c);
      k.push(key.clone());
      h.insert(key, selected_command);
    }
    
    let reordered = ReorderableList::new(i18n::CMDS_REORDER, k).prompt()?;
    
    let mut selected_commands_ordered = vec![];
    for key in reordered {
      selected_commands_ordered.push((*h.get(&key).unwrap()).clone());
    }
    
    *self = selected_commands_ordered;
    
    Ok(())
  }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(CustomCommand::new_from_prompt()?);
    
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("`{}`", c.bash_c.green());
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::CMD_SELECT_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    
    Ok(())
  }
}

impl Edit for Vec<ProgrammingLanguage> {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let mut cmap = hmap!();
      let mut cs = vec![];
      
      self.iter_mut().for_each(|c| {
        let s = format!("{} `{}`", i18n::LANGUAGE, c);
        
        cmap.insert(s.clone(), c);
        cs.push(s);
      });
      
      cs.extend_from_slice(&[i18n::ADD.to_string(), i18n::REMOVE.to_string()]);
      
      if let Some(action) = inquire::Select::new(&format!("{} {}:", i18n::PL_ACTION_PROMPT, i18n::HIT_ESC), cs).prompt_skippable()? {
        match action.as_str() {
          i18n::ADD => self.add_item()?,
          i18n::REMOVE => self.remove_item()?,
          _ => {},
        }
      } else { break }
    }
    
    Ok(())
  }
  
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  
  fn add_item(&mut self) -> anyhow::Result<()> {
    self.push(ProgrammingLanguage::new_from_prompt()?);
    Ok(())
  }
  
  fn remove_item(&mut self) -> anyhow::Result<()> {
    let mut cmap = hmap!();
    let mut cs = vec![];
    
    self.iter().for_each(|c| {
      let s = format!("`{}`", c);
      
      cmap.insert(s.clone(), c);
      cs.push(s);
    });
    
    let selected = inquire::Select::new(i18n::PL_TO_REMOVE, cs.clone()).prompt()?;
    
    let mut commands = vec![];
    for key in cs {
      if key.as_str().eq(selected.as_str()) { continue }
      commands.push((*cmap.get(&key).unwrap()).clone());
    }
    
    *self = commands;
    Ok(())
  }
}
