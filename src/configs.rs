//! Configurations module.
//! 
//! Defines global and project configurations.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::actions::{DescribedAction, Action, buildlike::BuildAction};
use crate::pipelines::DescribedPipeline;
use crate::entities::{
  custom_command::CustomCommand,
  info::{Info, ShortName},
  targets::TargetDescription,
  programming_languages::ProgrammingLanguage,
  remote_host::RemoteHost,
  requirements::Requirement,
  variables::Variable,
};
use crate::hmap;
use crate::utils::{ordered_map, ordered_set};

/// Project configuration.
#[derive(Deserialize, Serialize, PartialEq, Default)]
pub(crate) struct DeployerProjectOptions {
  /// Project name.
  pub(crate) project_name: String,
  
  /// Programming languages used by project.
  pub(crate) langs: Vec<ProgrammingLanguage>,
  
  /// Targets of the project.
  pub(crate) targets: Vec<TargetDescription>,
  
  /// Deploy toolkit (if needed).
  pub(crate) deploy_toolkit: Option<String>,
  
  /// Cache files and folders' relative paths.
  /// 
  /// Cache files are ignored by default when creating new build foler;
  /// but you can copy or symlink any cache files from project's folder
  /// by build options (see `crate::build::build` function).
  #[serde(serialize_with = "ordered_set")]
  pub(crate) cache_files: HashSet<PathBuf>,
  
  /// Project Pipelines.
  pub(crate) pipelines: Vec<DescribedPipeline>,
  
  /// Project artifacts' relative paths.
  /// 
  /// After every Pipeline artifacts are placing inside project's folder.
  pub(crate) artifacts: Vec<PathBuf>,
  
  /// Project variables.
  /// 
  /// This is how you can change your shell commands on the fly.
  pub(crate) variables: Vec<Variable>,
  
  /// Relative artifacts sources inside build folder and destinations inside
  /// `artifacts` folder in project's directory.
  pub(crate) place_artifacts_into_project_root: Vec<(PathBuf, PathBuf)>,
}

/// Global Deployer's configuration.
#[derive(Deserialize, Serialize)]
pub(crate) struct DeployerGlobalConfig {
  /// Project list.
  pub(crate) projects: Vec<String>,
  
  /// Available Actions Registry.
  #[serde(serialize_with = "ordered_map")]
  pub(crate) actions_registry: HashMap<Info, DescribedAction>,
  
  /// Available Pipelines Registry.
  #[serde(serialize_with = "ordered_map")]
  pub(crate) pipelines_registry: HashMap<Info, DescribedPipeline>,
  
  /// Available remote hosts Registry.
  #[serde(serialize_with = "ordered_map")]
  pub(crate) remote_hosts: HashMap<ShortName, RemoteHost>,
}

impl DeployerGlobalConfig {
  /// Appends global configuration by Actions that can't be added by TUI.
  pub(crate) fn make_sure_contain_defaults(actions_registry: &mut HashMap<Info, DescribedAction>) {
    let info = Info::new("interrupt", "0.1").unwrap();
    actions_registry.insert(info.clone(), DescribedAction {
      title: "Interrupt Pipeline".into(),
      desc: "Interrupt Pipeline execution until user press a button.".into(),
      info,
      tags: vec![],
      action: Action::Interrupt,
      requirements: None,
      exec_in_project_dir: None,
    });
  }
}

impl Default for DeployerGlobalConfig {
  fn default() -> Self {
    let mut actions_registry = hmap!();
    let pipelines_registry = hmap!();
    DeployerGlobalConfig::make_sure_contain_defaults(&mut actions_registry);
    
    let info = Info::new("cargo-rel", "0.1").unwrap();
    actions_registry.insert(info.clone(), DescribedAction {
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
      requirements: Some(vec![Requirement::Exists(PathBuf::from("/bin/cargo"))]),
      exec_in_project_dir: None,
    });
    
    Self {
      projects: vec![],
      actions_registry,
      pipelines_registry,
      remote_hosts: hmap!(),
    }
  }
}
