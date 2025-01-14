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
  check::CheckAction,
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
  custom_command::CustomCommand,
  info::{ActionInfo, ContentInfo, StrToInfo, info2str, str2info},
  programming_languages::ProgrammingLanguage,
  requirements::Requirement,
  targets::TargetDescription,
  variables::Variable,
};
use crate::hmap;
use crate::i18n;
use crate::rw::read_checked;

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
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) requirements: Option<Vec<Requirement>>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) enum Action {
  /// Действие прерывания. Используется, когда пользователю необходимо выполнить действия самостоятельно.
  Interrupt,
  
  /// Действие синхронизации папки проекта с удалённым хостом
  RemoteSync,
  
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
      Action::Interrupt | Action::ForceArtifactsEnplace | Action::Patch(_) | Action::UseFromStorage(_) | Action::AddToStorage(_) | Action::RemoteSync => self.action.clone(),
    };
    
    let mut described_action = self.clone();
    described_action.action = action;
    
    Ok(described_action)
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
  let info = action.info.clone();
  
  if !Confirm::new(i18n::ARE_YOU_SURE).prompt()? { return Ok(()) }
  
  globals.actions_registry.remove(&info);
  
  Ok(())
}

/// Добавляет новое действие.
pub(crate) fn new_action(
  globals: &mut DeployerGlobalConfig,
  args: &NewActionArgs,
) -> anyhow::Result<DescribedAction> {
  let actions = &mut globals.actions_registry;
  
  if let Some(from_file) = &args.from {
    let action = read_checked::<DescribedAction>(from_file).map_err(|e| {
      panic!("Can't read provided Action file due to: {}", e);
    }).unwrap();
    actions.insert(action.info.clone(), action.clone());
    return Ok(action)
  }
  
  let described_action = DescribedAction::new_from_prompt(globals)?;
  
  Ok(described_action)
}

pub(crate) fn cat_action(
  globals: &DeployerGlobalConfig,
  args: &CatActionArgs,
) -> anyhow::Result<()> {
  let action = match globals.actions_registry.get(&args.action_short_info_and_version.to_info()?) {
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
  let described_action = match globals.actions_registry.get_mut(&args.action_short_info_and_version.to_info()?) {
    None => exit(1),
    Some(action) => action,
  };
  
  described_action.edit_action_from_prompt()?;
  
  Ok(())
}
