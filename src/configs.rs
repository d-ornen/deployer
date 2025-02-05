//! Configurations module.
//!
//! Defines global and project configurations.

mod migrate;
mod migrations;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::actions::{Action, DescribedAction, buildlike::BuildAction};
use crate::entities::{
  custom_command::CustomCommand,
  info::{Info, ShortName},
  programming_languages::ProgrammingLanguage,
  remote_host::RemoteHost,
  requirements::Requirement,
  targets::TargetDescription,
  variables::Variable,
};
use crate::pipelines::DescribedPipeline;
use crate::utils::{ordered_map, ordered_set};
use crate::{hmap, hset};

const CURRENT_PROJECT_CONF_VERSION: u8 = 3;
fn get_default_project_conf_version() -> u8 {
  CURRENT_PROJECT_CONF_VERSION
}

/// Project configuration.
#[derive(Deserialize, Serialize, PartialEq)]
pub struct DeployerProjectOptions {
  /// Project name.
  pub project_name: String,

  /// Programming languages used by project.
  pub langs: Vec<ProgrammingLanguage>,

  /// Targets of the project.
  pub targets: Vec<TargetDescription>,

  /// Deploy toolkit (if needed).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deploy_toolkit: Option<String>,

  /// Cache files and folders' relative paths.
  ///
  /// Cache files are ignored by default when creating new build foler;
  /// but you can copy or symlink any cache files from project's folder
  /// by build options (see `crate::build::build` function).
  #[serde(serialize_with = "ordered_set")]
  pub cache_files: HashSet<PathBuf>,

  /// Project Pipelines.
  pub pipelines: Vec<DescribedPipeline>,

  /// Project artifacts' relative paths.
  ///
  /// After every Pipeline artifacts are placing inside project's folder.
  pub artifacts: Vec<PathBuf>,

  /// Project variables.
  ///
  /// This is how you can change your shell commands on the fly.
  pub variables: Vec<Variable>,

  /// Relative artifacts sources inside build folder and destinations inside
  /// `artifacts` folder in project's directory.
  pub place_artifacts_into_project_root: Vec<Placement>,

  /// Configuration version
  #[serde(default = "get_default_project_conf_version")]
  pub version: u8,
}

/// Artifact placement.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct Placement {
  pub from: PathBuf,
  pub to: PathBuf,
}

impl Default for DeployerProjectOptions {
  fn default() -> Self {
    Self {
      project_name: String::new(),
      langs: vec![],
      targets: vec![],
      deploy_toolkit: None,
      cache_files: hset!(),
      pipelines: vec![],
      artifacts: vec![],
      variables: vec![],
      place_artifacts_into_project_root: vec![],
      version: CURRENT_PROJECT_CONF_VERSION,
    }
  }
}

const CURRENT_GLOBAL_CONF_VERSION: u8 = 3;
fn get_default_global_conf_version() -> u8 {
  CURRENT_GLOBAL_CONF_VERSION
}

/// Global Deployer's configuration.
#[derive(Deserialize, Serialize)]
pub struct DeployerGlobalConfig {
  /// Project list.
  pub projects: Vec<String>,

  /// Available Actions Registry.
  #[serde(serialize_with = "ordered_map")]
  pub actions_registry: HashMap<Info, DescribedAction>,

  /// Available Pipelines Registry.
  #[serde(serialize_with = "ordered_map")]
  pub pipelines_registry: HashMap<Info, DescribedPipeline>,

  /// Available remote hosts Registry.
  #[serde(serialize_with = "ordered_map")]
  pub remote_hosts: HashMap<ShortName, RemoteHost>,

  /// Global configuration version
  #[serde(default = "get_default_global_conf_version")]
  pub version: u8,
}

impl DeployerGlobalConfig {
  /// Appends global configuration by Actions that can't be added by TUI.
  pub fn make_sure_contain_defaults(actions_registry: &mut HashMap<Info, DescribedAction>) {
    let info = Info::new("interrupt", "0.1").unwrap();
    actions_registry.insert(
      info.clone(),
      DescribedAction {
        title: "Interrupt Pipeline".into(),
        desc: "Interrupt Pipeline execution until user press a button.".into(),
        info,
        tags: vec![],
        action: Action::Interrupt,
        requirements: None,
        exec_in_project_dir: None,
      },
    );
  }
}

impl Default for DeployerGlobalConfig {
  fn default() -> Self {
    let mut actions_registry = hmap!();
    let pipelines_registry = hmap!();
    DeployerGlobalConfig::make_sure_contain_defaults(&mut actions_registry);

    let info = Info::new("cargo-rel", "0.1").unwrap();
    actions_registry.insert(
      info.clone(),
      DescribedAction {
        title: "Cargo Build (Release)".into(),
        desc: "Build the Rust project with Cargo default settings in release mode".into(),
        info,
        tags: vec!["rust".into(), "cargo".into()],
        action: Action::Build(BuildAction {
          supported_langs: vec![ProgrammingLanguage::Rust],
          commands: vec![CustomCommand {
            bash_c: "cargo build --release".into(),
            placeholders: None,
            replacements: None,
            ignore_fails: false,
            show_success_output: false,
            show_bash_c: true,
            only_when_fresh: None,
            remote_exec: None,
          }],
        }),
        requirements: Some(vec![Requirement::Exists {
          path: PathBuf::from("/bin/cargo"),
        }]),
        exec_in_project_dir: None,
      },
    );

    Self {
      projects: vec![],
      actions_registry,
      pipelines_registry,
      remote_hosts: hmap!(),
      version: CURRENT_GLOBAL_CONF_VERSION,
    }
  }
}
