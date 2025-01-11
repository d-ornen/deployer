use colored::Colorize;
use std::path::PathBuf;

use crate::actions::{Action, DescribedAction};
use crate::configs::DeployerGlobalConfig;
use crate::entities::programming_languages::specify_programming_languages;
use crate::entities::targets::TargetDescription;
use crate::entities::traits::{Edit, EditExtended};
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
