//! Actions module.
//!
//! Action is the main entity of Deployer. Actions as part of Pipelines are used to build, install, and deploy processes.

#[cfg(feature = "tui")]
use colored::Colorize;
use serde::{Deserialize, Serialize};
#[cfg(feature = "tui")]
use std::process::exit;

pub mod buildlike;
pub mod check;
pub mod deploylike;
pub mod observe;
pub mod packlike;
pub mod patch;
pub mod storage_add;

use crate::actions::{
  buildlike::{BuildAction, PostBuildAction, PreBuildAction, TestAction},
  check::CheckAction,
  deploylike::{ConfigureDeployAction, DeployAction, PostDeployAction},
  observe::ObserveAction,
  packlike::{DeliveryAction, InstallAction, PackAction},
  patch::PatchAction,
  storage_add::AddToStorageAction,
};
#[cfg(feature = "tui")]
use crate::cmd::{CatActionArgs, NewActionArgs};
#[cfg(feature = "tui")]
use crate::configs::DeployerGlobalConfig;
#[cfg(feature = "tui")]
use crate::entities::info::StrToInfo;
use crate::entities::{
  custom_command::CustomCommand,
  info::{ActionInfo, ContentInfo, ShortName, info2str, str2info, str2info_wl},
  requirements::Requirement,
};
#[cfg(feature = "tui")]
use crate::hmap;
#[cfg(feature = "tui")]
use crate::i18n;
#[cfg(feature = "tui")]
use crate::rw::read_checked;

/// Described Action.
///
/// Represents common info about Action, some properties such as requirements, and
/// Action definition.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct DescribedAction {
  /// Name of the Action.
  pub title: String,

  /// Description.
  pub desc: String,

  /// Short name and version.
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub info: ActionInfo,

  /// List of tags (to use with `grep` when searching through `deployer ls actions`).
  pub tags: Vec<String>,

  /// Action definition.
  pub action: Action,

  /// Requirements list.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub requirements: Option<Vec<Requirement>>,

  /// Flag of execution inside project folder.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exec_in_project_dir: Option<bool>,
}

/// Action types.
///
/// See [DOCS.en.md](/DOCS.en.md) and [DOCS.ru.md](/DOCS.ru.md).
#[derive(Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Action {
  /// Used when the user needs to perform actions independently.
  Interrupt,

  /// Action to synchronize build folder with remote host.
  SyncToRemote { remote_host_name: ShortName },
  /// Action to synchronize build folder from remote host.
  SyncFromRemote { remote_host_name: ShortName },

  /// Custom Pipeline commands.
  Custom(CustomCommand),
  /// Used to check output of custom command.
  Check(CheckAction),

  /// Action to prepare project files to build.
  PreBuild(PreBuildAction),
  /// Action to build the project.
  Build(BuildAction),
  /// Action to do something after successful build.
  PostBuild(PostBuildAction),
  /// Action to run tests.
  Test(TestAction),

  /// Action to pack code into packages
  /// (`.msi`, `.deb`, `.rpm`, `.apk`/`.aab`, `.ipa`, `.dmg`, etc.).
  Pack(PackAction),
  /// Action do deliver the packages (upload to repositories/sites/markets, etc.).
  Deliver(DeliveryAction),
  /// Action to install (on this or any remote host, via ADB, etc.).
  Install(InstallAction),

  /// Action to prepare deploy environment
  /// (for example, to start podman's QEMU machine, etc.).
  ConfigureDeploy(ConfigureDeployAction),
  /// Action to deploy your application (run `docker compose up -d`, etc.).
  Deploy(DeployAction),
  /// Action to update some states or run tests on deployment.
  PostDeploy(PostDeployAction),

  /// Action to run observers (`btop`, Jaeger, Prometheus, Grafana, etc.).
  Observe(ObserveAction),

  /// Action to copy content with given info from Deployer's storage.
  UseFromStorage {
    #[serde(serialize_with = "info2str", deserialize_with = "str2info_wl")]
    content_info: ContentInfo,
  },
  /// Action to add all available artifacts to Deployer's storage.
  AddToStorage(AddToStorageAction),

  /// Action to apply `smart-patcher` patches
  /// (see [`smart-patcher` repository](https://github.com/impulse-sw/smart-patcher)).
  Patch(PatchAction),
}

/// Prints all available Actions on the screen.
#[cfg(feature = "tui")]
pub fn list_actions(globals: &DeployerGlobalConfig) {
  println!("{}", i18n::ACTIONS_AVAILABLE);

  let mut actions = globals.actions_registry.values().collect::<Vec<_>>();
  actions.sort_by_key(|a| a.info.to_str());

  for action in actions {
    let action_info = action.info.to_str();
    let action_title = format!("[{}]", action.title);
    let tags = if action.tags.is_empty() {
      String::new()
    } else {
      format!(" ({}: {})", i18n::TAGS, action.tags.join(", ").as_str().blue().italic())
    };
    println!(
      "• {} {}{}",
      action_info.blue().bold(),
      action_title.green().bold(),
      tags
    );
    if !action.desc.is_empty() {
      println!("\t> {}", action.desc.green().italic());
    }
  }
}

/// Removes selected Action.
#[cfg(feature = "tui")]
pub fn remove_action(globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
  use inquire::{Confirm, Select};

  if globals.actions_registry.is_empty() {
    println!("{}", i18n::NO_ACTIONS);
    return Ok(());
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

  if !Confirm::new(i18n::ARE_YOU_SURE).prompt()? {
    return Ok(());
  }

  globals.actions_registry.remove(&info);

  Ok(())
}

/// Adds new Action.
#[cfg(feature = "tui")]
pub fn new_action(globals: &mut DeployerGlobalConfig, args: &NewActionArgs) -> anyhow::Result<DescribedAction> {
  let actions = &mut globals.actions_registry;

  if let Some(from_file) = &args.from {
    let action = read_checked::<DescribedAction>(from_file)
      .map_err(|e| {
        panic!("Can't read provided Action file due to: {}", e);
      })
      .unwrap();
    actions.insert(action.info.clone(), action.clone());
    return Ok(action);
  }

  let described_action = DescribedAction::new_from_prompt(globals)?;

  Ok(described_action)
}

/// Prints Action as JSON.
#[cfg(feature = "tui")]
pub fn cat_action(globals: &DeployerGlobalConfig, args: &CatActionArgs) -> anyhow::Result<()> {
  let action = match globals
    .actions_registry
    .get(&args.action_short_info_and_version.to_info()?)
  {
    None => exit(1),
    Some(action) => action,
  };

  let action_json = serde_json::to_string_pretty(&action).unwrap();
  println!("{}", action_json);

  Ok(())
}

/// Edits the Action.
#[cfg(feature = "tui")]
pub fn edit_action(globals: &mut DeployerGlobalConfig, args: &CatActionArgs) -> anyhow::Result<()> {
  let described_action = match globals
    .actions_registry
    .get_mut(&args.action_short_info_and_version.to_info()?)
  {
    None => exit(1),
    Some(action) => action,
  };

  described_action.edit_action_from_prompt()?;

  Ok(())
}
