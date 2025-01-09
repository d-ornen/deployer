use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::exit;

pub(crate) mod check;
pub(crate) mod buildlike;
pub(crate) mod packlike;
pub(crate) mod deploylike;
pub(crate) mod observe;
pub(crate) mod patch;
pub(crate) mod storage_add;

use crate::actions::{
  check::{CheckAction, specify_regex},
  buildlike::*,
  packlike::*,
  deploylike::*,
  observe::ObserveAction,
  patch::PatchAction,
  storage_add::AddToStorageAction,
};
use crate::cmd::{NewActionArgs, CatActionArgs};
use crate::configs::DeployerGlobalConfig;
use crate::entities::{
  auto_version::AutoVersionExtractFromRule,
  custom_command::{CustomCommand, specify_bash_c},
  info::{ActionInfo, ContentInfo, info2str, str2info},
  programming_languages::{ProgrammingLanguage, specify_programming_languages},
  targets::TargetDescription,
  traits::{Edit, EditExtended},
  variables::Variable,
};
use crate::hmap;
use crate::i18n;
use crate::rw::read_checked;
use crate::utils::tags_custom_type;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) struct DescribedAction {
  pub(crate) title: String,
  pub(crate) desc: String,
  /// Короткое имя и версия
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) info: ActionInfo,
  /// Список меток для фильтрации действий при выборе из реестра
  pub(crate) tags: Vec<String>,
  pub(crate) action: Action,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) enum Action {
  /// Действие прерывания. Используется, когда пользователю необходимо выполнить действия самостоятельно.
  Interrupt,
  
  /// Кастомные команды сборки
  Custom(CustomCommand),
  /// Команда проверки состояния (может прерывать пайплайн при проверке вывода)
  Check(CheckAction),
  
  /// Принудительно размещает доступные артефакты
  ForceArtifactsEnplace,
  
  /// Действие перед сборкой
  PreBuild(PreBuildAction),
  /// Действие сборки
  Build(BuildAction),
  /// Действие после сборки
  PostBuild(PostBuildAction),
  
  /// Тесты
  Test(TestAction),
  
  /// Упаковка артефактов
  Pack(PackAction),
  /// Доставка артефактов
  Deliver(DeliveryAction),
  /// Установка артефактов
  Install(InstallAction),
  
  /// Действие перед развёртыванием
  ConfigureDeploy(ConfigureDeployAction),
  /// Развёртывание
  Deploy(DeployAction),
  /// Действие после развёртывания
  PostDeploy(PostDeployAction),
  
  /// Действие наблюдения за состоянием
  Observe(ObserveAction),
  
  /// Действие добавления содержимого из хранилища.
  /// Это могут быть любые файлы и папки, сохраняющие структуру расположения
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  UseFromStorage(ContentInfo),
  
  /// Действие автоматического добавления артефактов в хранилище
  AddToStorage(AddToStorageAction),
  
  /// Действие применения патча
  Patch(PatchAction),
}

impl DescribedAction {
  pub(crate) fn new_from_prompt(opts: &mut DeployerGlobalConfig) -> anyhow::Result<Self> {
    use inquire::{Select, Text};
    
    let short_name = Text::new(i18n::ACTION_SHORT_NAME).prompt()?;
    let version = Text::new(i18n::ACTION_VERSION).prompt()?;
    
    let info = ActionInfo::new(short_name, version)?;
    
    let name = Text::new(i18n::ACTION_FULL_NAME).prompt()?;
    let desc = Text::new(i18n::ACTION_DESC).prompt()?;
    
    let tags: Vec<String> = tags_custom_type(i18n::ACTION_TAGS, None).prompt()?;
    
    let action_types: Vec<&str> = vec![
      "Interrupt",
      "Custom",
      "Check",
      "Force artifacts enplace",
      "Use content from storage",
      "Patch",
      "Pre-build",
      "Build",
      "Post-build",
      "Test",
      "Pack",
      "Deliver",
      "Install",
      "Configure deploy",
      "Deploy",
      "Post-deploy",
      "Observe",
      "Automatical push artifacts to the common storage",
    ];
    
    let selected_action_type = Select::new(i18n::ACTION_SELECT_TYPE, action_types).prompt()?;
    
    let action = match selected_action_type {
      "Interrupt" => Action::Interrupt,
      "Force artifacts enplace" => Action::ForceArtifactsEnplace,
      "Custom" => {
        let command = CustomCommand::new_from_prompt()?;
        Action::Custom(command)
      },
      "Check" => {
        let bash_c = specify_bash_c(None)?;
        
        let placeholders = tags_custom_type(i18n::CMD_PLACEHOLDERS, None).prompt()?;
        let placeholders = if placeholders.is_empty() { None } else { Some(placeholders) };
        
        let ignore_fails = !inquire::Confirm::new(i18n::CHECK_IGNORE_FAILS).with_default(true).prompt()?;
        
        let mut success_when_found = None;
        let mut success_when_not_found = None;
        loop {
          if inquire::Confirm::new(i18n::SPECIFY_REGEX_SUCC).with_default(true).prompt()? {
            success_when_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_SUCC)?);
          }
          
          if inquire::Confirm::new(i18n::SPECIFY_REGEX_FAIL).with_default(true).prompt()? {
            success_when_not_found = Some(specify_regex(i18n::SPECIFY_REGEX_FOR_FAIL)?);
          }
          
          if success_when_found.is_some() || success_when_not_found.is_some() { break }
          else { println!("{}", i18n::CHECK_NEED_TO_AT_LEAST); }
        }
        
        Action::Check(CheckAction {
          success_when_found,
          success_when_not_found,
          command: CustomCommand {
            bash_c,
            placeholders,
            replacements: None,
            ignore_fails,
            show_success_output: true,
            show_bash_c: false,
            only_when_fresh: None,
          },
        })
      },
      action_type @ ("Pre-build" | "Build" | "Post-build" | "Test") => {
        let supported_langs = specify_programming_languages()?;
        let commands = collect_multiple_commands()?;
        
        let action = BuildAction {
          supported_langs,
          commands,
        };
        
        match action_type {
          "Pre-build" => Action::PreBuild(action),
          "Build" => Action::Build(action),
          "Post-build" => Action::PostBuild(action),
          "Test" => Action::Test(action),
          _ => unreachable!(),
        }
      },
      action_type @ ("Pack" | "Deliver" | "Install") => {
        let target = TargetDescription::new_from_prompt()?;
        let commands = collect_multiple_commands()?;
        
        let action = PackAction {
          target: Some(target),
          commands,
        };
        
        match action_type {
          "Pack" => Action::Pack(action),
          "Deliver" => Action::Deliver(action),
          "Install" => Action::Install(action),
          _ => unreachable!(),
        }
      },
      action_type @ ("Configure deploy" | "Deploy" | "Post-deploy") => {
        let deploy_toolkit = Text::new("Enter deploy toolkit name (or hit `esc`):").prompt_skippable()?;
        let tags = tags_custom_type("Enter deploy tags:", None).prompt()?;
        let commands = collect_multiple_commands()?;
        
        let action = DeployAction {
          deploy_toolkit,
          tags,
          commands,
        };
        
        match action_type {
          "Configure deploy" => Action::ConfigureDeploy(action),
          "Deploy" => Action::Deploy(action),
          "Post-deploy" => Action::PostDeploy(action),
          _ => unreachable!(),
        }
      },
      "Observe" => {
        let tags = tags_custom_type(i18n::OBSERVE_TAGS, None).prompt()?;
        let command = CustomCommand::new_from_prompt_unspecified()?;
        
        Action::Observe(ObserveAction { tags, command })
      },
      "Patch" => Action::Patch(PatchAction::new_from_prompt()?),
      "Use content from storage" => {
        let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
        let version = Text::new(i18n::CONTENT_VER).prompt()?;
        
        let info = ContentInfo::new_for_using(short_name, version)?;
        
        Action::UseFromStorage(info)
      },
      "Automatical push artifacts to the common storage" => {
        let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
        let auto_version_rule = AutoVersionExtractFromRule::new_from_prompt()?;
        
        Action::AddToStorage(AddToStorageAction { short_name, auto_version_rule })
      }
      _ => unreachable!(),
    };
    
    let described_action = DescribedAction {
      title: name,
      desc,
      info,
      tags,
      action,
    };
    
    if
      opts.actions_registry.contains_key(&described_action.info.to_str()) &&
      !inquire::Confirm::new(&i18n::ACTION_REG_ALREADY_HAVE.replace("{}", &described_action.info.to_str())).prompt()?
    {
      exit(0);
    }
    
    opts.actions_registry.insert(described_action.info.to_str(), described_action.clone());
    
    Ok(described_action)
  }
  
  fn setup_buildlike_action(
    &self,
    action: &BuildAction,
    langs: &Vec<ProgrammingLanguage>,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<BuildAction> {
    let mut action = action.clone();
    if 
      !langs.iter().any(|l| action.supported_langs.contains(l)) && 
      !inquire::Confirm::new(
        &i18n::ACTION_COMPAT_PLS
          .replace("{1}", &self.info.to_str())
          .replace("{2}", &format!("{:?}", action.supported_langs))
          .replace("{3}", &format!("{:?}", langs))
      ).prompt()?
    {
      return Ok(BuildAction::default())
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?; }
    
    Ok(action)
  }
  
  fn setup_packlike_action(
    &self,
    action: &PackAction,
    targets: &[TargetDescription],
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<PackAction> {
    let mut action = action.clone();
    
    if
      action.target.as_ref().is_some_and(|t| !targets.contains(t)) &&
      !inquire::Confirm::new(
        &i18n::ACTION_COMPAT_TARGETS
          .replace("{1}", &self.info.to_str())
          .replace("{2}", &format!("{}", action.target.as_ref().unwrap()))
          .replace("{3}", &format!("{:?}", targets.iter().map(TargetDescription::to_string).collect::<Vec<_>>()))
      ).prompt()?
    {
      return Ok(PackAction::default())
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?; }
    Ok(action)
  }
  
  fn setup_deploylike_action(
    &self,
    action: &DeployAction,
    deploy_toolkit: &Option<String>,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<DeployAction> {
    let mut action = action.clone();
    if
      action.deploy_toolkit.as_ref().is_some_and(|l| deploy_toolkit.as_ref().is_some_and(|r| l.as_str() != r.as_str())) &&
      !inquire::Confirm::new(
        &i18n::ACTION_COMPAT_DEPL_TOOLKIT
          .replace("{1}", &self.info.to_str())
          .replace("{2}", action.deploy_toolkit.as_ref().unwrap())
          .replace("{3}", deploy_toolkit.as_ref().unwrap())
      ).prompt()?
    {
      return Ok(DeployAction::default())
    }
    
    for cmd in &mut action.commands { *cmd = cmd.prompt_setup_for_project(&self.info, variables, artifacts)?; }
    
    Ok(action)
  }
  
  fn setup_observe_action(
    &self,
    action: &ObserveAction,
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<ObserveAction> {
    let mut action = action.clone();
    
    action.command = action.command.prompt_setup_for_project(&self.info, variables, artifacts)?;
    
    Ok(action)
  }
  
  pub(crate) fn prompt_setup_for_project(
    &self,
    langs: &Vec<ProgrammingLanguage>,
    deploy_toolkit: &Option<String>,
    targets: &[TargetDescription],
    variables: &[Variable],
    artifacts: &[PathBuf],
  ) -> anyhow::Result<Self> {
    let action = match &self.action {
      Action::Custom(cmd) => Action::Custom(cmd.prompt_setup_for_project(&self.info, variables, artifacts)?),
      Action::Check(cmd) => Action::Check(cmd.prompt_setup_for_project(&self.info, variables, artifacts)?),
      Action::PreBuild(pb_action) => Action::PreBuild(self.setup_buildlike_action(pb_action, langs, variables, artifacts)?),
      Action::Build(b_action) => Action::Build(self.setup_buildlike_action(b_action, langs, variables, artifacts)?),
      Action::PostBuild(pb_action) => Action::PostBuild(self.setup_buildlike_action(pb_action, langs, variables, artifacts)?),
      Action::Test(t_action) => Action::Test(self.setup_buildlike_action(t_action, langs, variables, artifacts)?),
      Action::Pack(p_action) => Action::Pack(self.setup_packlike_action(p_action, targets, variables, artifacts)?),
      Action::Deliver(p_action) => Action::Deliver(self.setup_packlike_action(p_action, targets, variables, artifacts)?),
      Action::Install(p_action) => Action::Install(self.setup_packlike_action(p_action, targets, variables, artifacts)?),
      Action::ConfigureDeploy(cd_action) => Action::ConfigureDeploy(self.setup_deploylike_action(cd_action, deploy_toolkit, variables, artifacts)?),
      Action::Deploy(d_action) => Action::Deploy(self.setup_deploylike_action(d_action, deploy_toolkit, variables, artifacts)?),
      Action::PostDeploy(pd_action) => Action::PostDeploy(self.setup_deploylike_action(pd_action, deploy_toolkit, variables, artifacts)?),
      Action::Observe(o_action) => Action::Observe(self.setup_observe_action(o_action, variables, artifacts)?),
      Action::Interrupt | Action::ForceArtifactsEnplace | Action::Patch(_) | Action::UseFromStorage(_) | Action::AddToStorage(_) => self.action.clone(),
    };
    
    let mut described_action = self.clone();
    described_action.action = action;
    
    Ok(described_action)
  }
  
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

/// Перечисляет все доступные действия.
pub(crate) fn list_actions(
  globals: &DeployerGlobalConfig,
) {
  println!("{}", i18n::ACTIONS_AVAILABLE);
  
  let mut actions = globals.actions_registry.values().collect::<Vec<_>>();
  actions.sort_by_key(|a| a.info.to_str());
  
  for action in actions {
    let action_info = action.info.to_str();
    let action_title = format!("[{}]", action.title);
    let tags = if action.tags.is_empty() { String::new() } else { format!(" ({}: {})", i18n::TAGS, action.tags.join(", ").as_str().blue().italic()) };
    println!("• {} {}{}", action_info.blue().bold(), action_title.green().bold(), tags);
    if !action.desc.is_empty() { println!("\t> {}", action.desc.green().italic()); }
  }
}

/// Удаляет выбранное действие.
pub(crate) fn remove_action(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<()> {
  use inquire::{Select, Confirm};
  
  if globals.actions_registry.is_empty() {
    println!("{}", i18n::NO_ACTIONS);
    return Ok(())
  }
  
  let mut actions = globals.actions_registry.values().collect::<Vec<_>>();
  actions.sort_by_key(|a| a.info.to_str());
  
  let (actions, keys) = {
    let mut h = hmap!();
    let mut k = vec![];
    
    for key in globals.actions_registry.keys() {
      let action = globals.actions_registry.get(key).unwrap();
      let new_key = format!("{} - {}", action.info.to_str(), action.title);
      h.insert(new_key.clone(), action);
      k.push(new_key);
    }
    
    k.sort();
    
    (h, k)
  };
  
  let selected_action = Select::new(i18n::ACTION_REGISTRY_CHOOSE_TO_REMOVE, keys).prompt()?;
  let action = *actions.get(&selected_action).unwrap();
  
  if !Confirm::new(i18n::ARE_YOU_SURE).prompt()? { return Ok(()) }
  
  globals.actions_registry.remove(&action.info.to_str());
  
  Ok(())
}

/// Добавляет новое действие.
pub(crate) fn new_action(
  globals: &mut DeployerGlobalConfig,
  args: &NewActionArgs,
  // data_dir: &str,
) -> anyhow::Result<DescribedAction> {
  let actions = &mut globals.actions_registry;
  
  if let Some(from_file) = &args.from {
    let action = read_checked::<DescribedAction>(from_file).map_err(|e| {
      panic!("Can't read provided Action file due to: {}", e);
    }).unwrap();
    actions.insert(action.info.to_str(), action.clone());
    return Ok(action)
  }
  
  let described_action = DescribedAction::new_from_prompt(globals)?;
  
  Ok(described_action)
}

/// Создаёт несколько новых команд.
fn collect_multiple_commands() -> anyhow::Result<Vec<CustomCommand>> {
  use inquire::Confirm;
  
  let mut commands = Vec::new();
  let mut first = true;
  while Confirm::new(i18n::ADD_CMD).with_default(first).prompt()? {
    if let Ok(command) = CustomCommand::new_from_prompt() {
      commands.push(command);
    }
    first = false;
  }
  Ok(commands)
}

// fn collect_key_value_pairs(prompt: &str) -> anyhow::Result<HashMap<String, String>> {
//   use inquire::Text;
//   
//   let mut map = HashMap::new();
//   loop {
//     let key = Text::new(prompt).prompt()?;
//     if key.is_empty() {
//       break;
//     }
//     let value = Text::new("Enter value:").prompt()?;
//     map.insert(key, value);
//   }
//   Ok(map)
// }

pub(crate) fn cat_action(
  globals: &DeployerGlobalConfig,
  args: &CatActionArgs,
) -> anyhow::Result<()> {
  let action = match globals.actions_registry.get(&args.action_short_info_and_version) {
    None => exit(1),
    Some(action) => action,
  };
  
  let action_json = serde_json::to_string_pretty(&action).unwrap();
  println!("{}", action_json);
  
  Ok(())
}

pub(crate) fn edit_action(
  globals: &mut DeployerGlobalConfig,
  args: &CatActionArgs,
) -> anyhow::Result<()> {
  let described_action = match globals.actions_registry.get_mut(&args.action_short_info_and_version) {
    None => exit(1),
    Some(action) => action,
  };
  
  described_action.edit_action_from_prompt()?;
  
  Ok(())
}
