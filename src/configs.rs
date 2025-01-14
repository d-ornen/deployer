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
use crate::utils::ordered_map;

/// Конфигурация проекта.
#[derive(Deserialize, Serialize, PartialEq, Default)]
pub(crate) struct DeployerProjectOptions {
  /// Название проекта.
  pub(crate) project_name: String,
  /// Языки
  pub(crate) langs: Vec<ProgrammingLanguage>,
  /// Таргеты
  pub(crate) targets: Vec<TargetDescription>,
  /// Тулкит для развёртывания
  pub(crate) deploy_toolkit: Option<String>,
  
  /// Метки кэша
  pub(crate) cache_files: HashSet<PathBuf>,
  
  /// Пайплайны
  pub(crate) pipelines: Vec<DescribedPipeline>,
  
  /// Артефакты
  pub(crate) artifacts: Vec<PathBuf>,
  /// Переменные
  pub(crate) variables: Vec<Variable>,
  /// Правила размещения артефактов
  pub(crate) inplace_artifacts_into_project_root: Vec<(PathBuf, PathBuf)>,
}

/// Global Deployer's configuration.
#[derive(Deserialize, Serialize)]
pub(crate) struct DeployerGlobalConfig {
  /// Project list.
  pub(crate) projects: Vec<String>,
  // /// List available project templates.
  // pub(crate) templates: Vec<String>,
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
  pub(crate) fn make_sure_contain_defaults(actions_registry: &mut HashMap<Info, DescribedAction>) {
    let info = Info::new("interrupt", "0.1").unwrap();
    actions_registry.insert(info.clone(), DescribedAction {
      title: "Interrupt Pipeline".into(),
      desc: "Interrupt Pipeline execution until user press a button.".into(),
      info,
      tags: vec!["interrupt".into()],
      action: Action::Interrupt,
      requirements: None,
    });
    
    let info = Info::new("force-artifacts-enplace", "0.1").unwrap();
    actions_registry.insert(info.clone(), DescribedAction {
      title: "Force artifacts enplace".into(),
      desc: "Enplace available artifacts from build directory during Pipeline execution.".into(),
      info,
      tags: vec!["interrupt".into()],
      action: Action::ForceArtifactsEnplace,
      requirements: None,
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
    });
    
    Self {
      projects: vec![],
      actions_registry,
      pipelines_registry,
      remote_hosts: hmap!(),
    }
  }
}
