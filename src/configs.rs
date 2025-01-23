//! Configurations module.
//!
//! Defines global and project configurations.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::actions::{Action, DescribedAction, buildlike::BuildAction};
use crate::entities::{
  custom_command::CustomCommand,
  info::{Info, ShortName},
  programming_languages::ProgrammingLanguage,
  remote_host::RemoteHost,
  requirements::Requirement,
  targets::TargetDescription,
  traits::ConfigAutoMigrate,
  variables::Variable,
};
use crate::hmap;
use crate::pipelines::DescribedPipeline;
use crate::utils::{ordered_map, ordered_set};

const CURRENT_PROJECT_CONF_VERSION: u8 = 2;
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
  pub place_artifacts_into_project_root: Vec<(PathBuf, PathBuf)>,

  /// Configuration version
  #[serde(default = "get_default_project_conf_version")]
  pub version: u8,
}

impl ConfigAutoMigrate<DeployerProjectOptions> for DeployerProjectOptions {
  fn migrate(path: &Path) -> anyhow::Result<DeployerProjectOptions> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let val = serde_json::from_reader::<_, serde_json::Value>(reader).map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut config = DeployerProjectOptions::default();

    if let Some(version) = val.get("version").and_then(|v| v.as_u64()) {
      config.version = version
        .try_into()
        .map_err(|_| anyhow::anyhow!("Error converting `version` to u8!"))?;

      if let Some(project_name) = val.get("project_name").and_then(|v| v.as_str()) {
        config.project_name = project_name.to_owned();
      } else {
        anyhow::bail!("No `project_name` field!");
      }

      if let Some(langs) = val.get("langs") {
        config.langs =
          serde_json::from_value(langs.clone()).map_err(|e| anyhow::anyhow!("Error parsing `langs`: {}", e))?;
      }
      if let Some(targets) = val.get("targets") {
        config.targets =
          serde_json::from_value(targets.clone()).map_err(|e| anyhow::anyhow!("Error parsing `targets`: {}", e))?;
      }
      if let Some(toolkit) = val.get("deploy_toolkit") {
        config.deploy_toolkit = serde_json::from_value(toolkit.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `deploy_toolkit`: {}", e))?;
      }
      if let Some(cache_files) = val.get("cache_files") {
        config.cache_files = serde_json::from_value(cache_files.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `cache_files`: {}", e))?;
      }
      if let Some(pipelines) = val.get("pipelines") {
        config.pipelines =
          serde_json::from_value(pipelines.clone()).map_err(|e| anyhow::anyhow!("Error parsing `pipelines`: {}", e))?;
      }
      if let Some(artifacts) = val.get("artifacts") {
        config.artifacts =
          serde_json::from_value(artifacts.clone()).map_err(|e| anyhow::anyhow!("Error parsing `artifacts`: {}", e))?;
      }
      if let Some(variables) = val.get("variables") {
        config.variables =
          serde_json::from_value(variables.clone()).map_err(|e| anyhow::anyhow!("Error parsing `variables`: {}", e))?;
      }
      if let Some(placement) = val.get("place_artifacts_into_project_root") {
        config.place_artifacts_into_project_root = serde_json::from_value(placement.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `place_artifacts_into_project_root`: {}", e))?;
      }

      Ok(config)
    } else {
      if let Some(project_name) = val.get("project_name").and_then(|v| v.as_str()) {
        config.project_name = project_name.to_owned();
      } else {
        anyhow::bail!("No `project_name` field!");
      }

      if let Some(langs) = val.get("langs") {
        config.langs =
          serde_json::from_value(langs.clone()).map_err(|e| anyhow::anyhow!("Error parsing `langs`: {}", e))?;
      }
      if let Some(targets) = val.get("targets") {
        config.targets =
          serde_json::from_value(targets.clone()).map_err(|e| anyhow::anyhow!("Error parsing `targets`: {}", e))?;
      }
      if let Some(toolkit) = val.get("deploy_toolkit") {
        config.deploy_toolkit = serde_json::from_value(toolkit.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `deploy_toolkit`: {}", e))?;
      }
      if let Some(cache_files) = val.get("cache_files") {
        config.cache_files = serde_json::from_value(cache_files.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `cache_files`: {}", e))?;
      }
      if let Some(pipelines) = val.get("pipelines") {
        config.pipelines =
          serde_json::from_value(pipelines.clone()).map_err(|e| anyhow::anyhow!("Error parsing `pipelines`: {}", e))?;
      }
      if let Some(artifacts) = val.get("artifacts") {
        config.artifacts =
          serde_json::from_value(artifacts.clone()).map_err(|e| anyhow::anyhow!("Error parsing `artifacts`: {}", e))?;
      }
      if let Some(variables) = val.get("variables") {
        config.variables =
          serde_json::from_value(variables.clone()).map_err(|e| anyhow::anyhow!("Error parsing `variables`: {}", e))?;
      }
      if let Some(placement) = val.get("inplace_artifacts_into_project_root") {
        config.place_artifacts_into_project_root = serde_json::from_value(placement.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `inplace_artifacts_into_project_root`: {}", e))?;
      }

      Ok(config)
    }
  }
}

impl Default for DeployerProjectOptions {
  fn default() -> Self {
    Self {
      project_name: String::new(),
      langs: vec![],
      targets: vec![],
      deploy_toolkit: None,
      cache_files: HashSet::new(),
      pipelines: vec![],
      artifacts: vec![],
      variables: vec![],
      place_artifacts_into_project_root: vec![],
      version: CURRENT_PROJECT_CONF_VERSION,
    }
  }
}

const CURRENT_GLOBAL_CONF_VERSION: u8 = 2;
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

impl ConfigAutoMigrate<DeployerGlobalConfig> for DeployerGlobalConfig {
  fn migrate(path: &Path) -> anyhow::Result<DeployerGlobalConfig> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let val = serde_json::from_reader::<_, serde_json::Value>(reader).map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut config = DeployerGlobalConfig::default();

    // Parse version
    if let Some(version) = val.get("version").and_then(|v| v.as_u64()) {
      config.version = version
        .try_into()
        .map_err(|_| anyhow::anyhow!("Error converting `version` to u8!"))?;

      if let Some(projects) = val.get("projects") {
        config.projects =
          serde_json::from_value(projects.clone()).map_err(|e| anyhow::anyhow!("Error parsing `projects`: {}", e))?;
      }
      if let Some(actions) = val.get("actions_registry") {
        config.actions_registry = serde_json::from_value(actions.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `actions_registry`: {}", e))?;
      }
      if let Some(pipelines) = val.get("pipelines_registry") {
        config.pipelines_registry = serde_json::from_value(pipelines.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `pipelines_registry`: {}", e))?;
      }
      if let Some(hosts) = val.get("remote_hosts") {
        config.remote_hosts =
          serde_json::from_value(hosts.clone()).map_err(|e| anyhow::anyhow!("Error parsing `remote_hosts`: {}", e))?;
      }

      Ok(config)
    } else {
      if let Some(projects) = val.get("projects") {
        config.projects =
          serde_json::from_value(projects.clone()).map_err(|e| anyhow::anyhow!("Error parsing `projects`: {}", e))?;
      }
      if let Some(actions) = val.get("actions_registry") {
        config.actions_registry = serde_json::from_value(actions.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `actions_registry`: {}", e))?;
      }
      if let Some(pipelines) = val.get("pipelines_registry") {
        config.pipelines_registry = serde_json::from_value(pipelines.clone())
          .map_err(|e| anyhow::anyhow!("Error parsing `pipelines_registry`: {}", e))?;
      }
      if let Some(hosts) = val.get("remote_hosts") {
        config.remote_hosts =
          serde_json::from_value(hosts.clone()).map_err(|e| anyhow::anyhow!("Error parsing `remote_hosts`: {}", e))?;
      }

      Ok(config)
    }
  }
}

impl DeployerGlobalConfig {
  /// Appends global configuration by Actions that can't be added by TUI.
  pub fn make_sure_contain_defaults(actions_registry: &mut HashMap<Info, DescribedAction>) {
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
      version: CURRENT_GLOBAL_CONF_VERSION,
    }
  }
}
