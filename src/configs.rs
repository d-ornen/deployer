//! Configurations module.
//!
//! Defines global and project configurations.

mod migrations;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::actions::{Action, DescribedAction, buildlike::BuildAction};
use crate::configs::migrations::project_config_v2_to_v3_migrations::*;
use crate::entities::{
  // containered_opts::ContaineredOpts,
  custom_command::CustomCommand,
  info::{Info, ShortName},
  programming_languages::ProgrammingLanguage,
  remote_host::RemoteHost,
  requirements::Requirement,
  targets::TargetDescription,
  traits::ConfigAutoMigrate,
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

  // /// Information for containered builds.
  // #[serde(skip_serializing_if = "Option::is_none")]
  // pub containered_options: Option<ContaineredOpts>,
  /// Configuration version
  #[serde(default = "get_default_project_conf_version")]
  pub version: u8,
}

impl ConfigAutoMigrate<DeployerProjectOptions> for DeployerProjectOptions {
  #[allow(clippy::let_and_return)]
  fn migrate(path: &Path) -> anyhow::Result<DeployerProjectOptions> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let val = serde_json::from_reader::<_, serde_json::Value>(reader).map_err(|e| anyhow::anyhow!("{}", e))?;

    let intermediate = if val.get("version").is_none_or(|v| v.as_u64().is_some_and(|v| v < 2)) {
      let mut config = DeployerProjectOptionsV2::default();

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

      config.version = 2;

      Ok(config)
    } else {
      let file = std::fs::File::open(path)?;
      let reader = std::io::BufReader::new(file);
      serde_json::from_reader::<_, DeployerProjectOptionsV2>(reader).map_err(|e| anyhow::anyhow!("{}", e))
    };

    let intermediate = if let Ok(intermediate) = intermediate
      && intermediate.version < 3
    {
      let config = DeployerProjectOptions {
        artifacts: intermediate.artifacts,
        cache_files: intermediate.cache_files,
        deploy_toolkit: intermediate.deploy_toolkit,
        langs: intermediate.langs,
        pipelines: {
          let mut new_pipelines = vec![];
          for pipeline in &intermediate.pipelines {
            new_pipelines.push({
              let mut new_actions = vec![];
              for action in &pipeline.actions {
                new_actions.push(DescribedAction {
                  action: match action.action.to_owned() {
                    ActionV2::AddToStorage(a) => Action::AddToStorage(a),
                    ActionV2::Build(b) => Action::Build(b),
                    ActionV2::Check(c) => Action::Check(c),
                    ActionV2::ConfigureDeploy(cd) => Action::ConfigureDeploy(cd),
                    ActionV2::Custom(cu) => Action::Custom(cu),
                    ActionV2::Deliver(del) => Action::Deliver(del),
                    ActionV2::Deploy(dep) => Action::Deploy(dep),
                    ActionV2::Install(ins) => Action::Install(ins),
                    ActionV2::Interrupt => Action::Interrupt,
                    ActionV2::Observe(obs) => Action::Observe(obs),
                    ActionV2::Pack(p) => Action::Pack(p),
                    ActionV2::Patch(pa) => Action::Patch(pa),
                    ActionV2::PostBuild(pb) => Action::PostBuild(pb),
                    ActionV2::PreBuild(prb) => Action::PreBuild(prb),
                    ActionV2::PostDeploy(ptd) => Action::PostDeploy(ptd),
                    ActionV2::SyncFromRemote(sfr) => Action::SyncToRemote { remote_host_name: sfr },
                    ActionV2::SyncToRemote(stre) => Action::SyncToRemote { remote_host_name: stre },
                    ActionV2::Test(t) => Action::Test(t),
                    ActionV2::UseFromStorage(u) => Action::UseFromStorage { content_info: u },
                  },
                  desc: action.desc.to_owned(),
                  exec_in_project_dir: action.exec_in_project_dir,
                  info: action.info.to_owned(),
                  requirements: action.requirements.to_owned(),
                  tags: action.tags.to_owned(),
                  title: action.title.to_owned(),
                });
              }
              DescribedPipeline {
                actions: new_actions,
                default: pipeline.default,
                desc: pipeline.desc.to_owned(),
                exclusive_exec_tag: pipeline.exclusive_exec_tag.to_owned(),
                info: pipeline.info.to_owned(),
                tags: pipeline.tags.to_owned(),
                title: pipeline.title.to_owned(),
              }
            });
          }
          new_pipelines
        },
        place_artifacts_into_project_root: intermediate.place_artifacts_into_project_root,
        project_name: intermediate.project_name,
        targets: intermediate.targets,
        variables: intermediate.variables,
        version: 3,
      };

      Ok(config)
    } else {
      let file = std::fs::File::open(path)?;
      let reader = std::io::BufReader::new(file);
      serde_json::from_reader::<_, DeployerProjectOptions>(reader).map_err(|e| anyhow::anyhow!("{}", e))
    };

    intermediate
  }
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
      // containered_options: None,
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

impl ConfigAutoMigrate<DeployerGlobalConfig> for DeployerGlobalConfig {
  #[allow(clippy::let_and_return)]
  fn migrate(path: &Path) -> anyhow::Result<DeployerGlobalConfig> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let val = serde_json::from_reader::<_, serde_json::Value>(reader).map_err(|e| anyhow::anyhow!("{}", e))?;

    let intermediate = if val.get("version").is_none_or(|v| v.as_u64().is_some_and(|v| v < 2)) {
      let mut config = DeployerGlobalConfigV2::default();

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
      let file = std::fs::File::open(path)?;
      let reader = std::io::BufReader::new(file);
      serde_json::from_reader::<_, DeployerGlobalConfigV2>(reader).map_err(|e| anyhow::anyhow!("{}", e))
    };

    let intermediate = if let Ok(intermediate) = intermediate
      && intermediate.version < 3
    {
      let mut config = DeployerGlobalConfig {
        projects: intermediate.projects,
        remote_hosts: intermediate.remote_hosts,
        version: 3,
        ..Default::default()
      };

      let mut new_actions = hmap!();
      for action in &intermediate.actions_registry {
        new_actions.insert(action.0.to_owned(), DescribedAction {
          action: match action.1.action.to_owned() {
            ActionV2::AddToStorage(a) => Action::AddToStorage(a),
            ActionV2::Build(b) => Action::Build(b),
            ActionV2::Check(c) => Action::Check(c),
            ActionV2::ConfigureDeploy(cd) => Action::ConfigureDeploy(cd),
            ActionV2::Custom(cu) => Action::Custom(cu),
            ActionV2::Deliver(del) => Action::Deliver(del),
            ActionV2::Deploy(dep) => Action::Deploy(dep),
            ActionV2::Install(ins) => Action::Install(ins),
            ActionV2::Interrupt => Action::Interrupt,
            ActionV2::Observe(obs) => Action::Observe(obs),
            ActionV2::Pack(p) => Action::Pack(p),
            ActionV2::Patch(pa) => Action::Patch(pa),
            ActionV2::PostBuild(pb) => Action::PostBuild(pb),
            ActionV2::PreBuild(prb) => Action::PreBuild(prb),
            ActionV2::PostDeploy(ptd) => Action::PostDeploy(ptd),
            ActionV2::SyncFromRemote(sfr) => Action::SyncToRemote { remote_host_name: sfr },
            ActionV2::SyncToRemote(stre) => Action::SyncToRemote { remote_host_name: stre },
            ActionV2::Test(t) => Action::Test(t),
            ActionV2::UseFromStorage(u) => Action::UseFromStorage { content_info: u },
          },
          desc: action.1.desc.to_owned(),
          exec_in_project_dir: action.1.exec_in_project_dir,
          info: action.1.info.to_owned(),
          requirements: action.1.requirements.to_owned(),
          tags: action.1.tags.to_owned(),
          title: action.1.title.to_owned(),
        });
      }
      config.actions_registry = new_actions;

      let mut new_pipelines = hmap!();
      for pipeline in &intermediate.pipelines_registry {
        new_pipelines.insert(pipeline.0.to_owned(), {
          let mut new_actions = vec![];
          for action in &pipeline.1.actions {
            new_actions.push(DescribedAction {
              action: match action.action.to_owned() {
                ActionV2::AddToStorage(a) => Action::AddToStorage(a),
                ActionV2::Build(b) => Action::Build(b),
                ActionV2::Check(c) => Action::Check(c),
                ActionV2::ConfigureDeploy(cd) => Action::ConfigureDeploy(cd),
                ActionV2::Custom(cu) => Action::Custom(cu),
                ActionV2::Deliver(del) => Action::Deliver(del),
                ActionV2::Deploy(dep) => Action::Deploy(dep),
                ActionV2::Install(ins) => Action::Install(ins),
                ActionV2::Interrupt => Action::Interrupt,
                ActionV2::Observe(obs) => Action::Observe(obs),
                ActionV2::Pack(p) => Action::Pack(p),
                ActionV2::Patch(pa) => Action::Patch(pa),
                ActionV2::PostBuild(pb) => Action::PostBuild(pb),
                ActionV2::PreBuild(prb) => Action::PreBuild(prb),
                ActionV2::PostDeploy(ptd) => Action::PostDeploy(ptd),
                ActionV2::SyncFromRemote(sfr) => Action::SyncToRemote { remote_host_name: sfr },
                ActionV2::SyncToRemote(stre) => Action::SyncToRemote { remote_host_name: stre },
                ActionV2::Test(t) => Action::Test(t),
                ActionV2::UseFromStorage(u) => Action::UseFromStorage { content_info: u },
              },
              desc: action.desc.to_owned(),
              exec_in_project_dir: action.exec_in_project_dir,
              info: action.info.to_owned(),
              requirements: action.requirements.to_owned(),
              tags: action.tags.to_owned(),
              title: action.title.to_owned(),
            });
          }
          DescribedPipeline {
            actions: new_actions,
            default: pipeline.1.default,
            desc: pipeline.1.desc.to_owned(),
            exclusive_exec_tag: pipeline.1.exclusive_exec_tag.to_owned(),
            info: pipeline.1.info.to_owned(),
            tags: pipeline.1.tags.to_owned(),
            title: pipeline.1.title.to_owned(),
          }
        });
      }
      config.pipelines_registry = new_pipelines;

      Ok(config)
    } else {
      let file = std::fs::File::open(path)?;
      let reader = std::io::BufReader::new(file);
      serde_json::from_reader::<_, DeployerGlobalConfig>(reader).map_err(|e| anyhow::anyhow!("{}", e))
    };

    intermediate
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
