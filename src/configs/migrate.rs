//! Module for perform migrations from old-versioned configs to new ones.

use std::path::Path;

use crate::actions::buildlike::{BuildAction, PostBuildAction, PreBuildAction};
use crate::actions::deploylike::{ConfigureDeployAction, DeployAction, PostDeployAction};
use crate::actions::observe::ObserveAction;
use crate::actions::packlike::{DeliveryAction, InstallAction, PackAction};
use crate::actions::test::TestAction;
use crate::actions::{Action, DescribedAction};
use crate::configs::migrations::project_config_v2_to_v3_migrations::*;
use crate::configs::migrations::project_config_v3_to_v4_migrations::*;
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::entities::custom_command::{CustomCommand, Replacement, ReplacementGroup};
use crate::entities::programming_languages::ProgrammingLanguage;
use crate::entities::requirements::Requirement;
use crate::entities::targets::{OsVariant, OsVersionSpecification, TargetDescription};
use crate::entities::traits::ConfigAutoMigrate;
use crate::entities::variables::{VarValue, Variable};
use crate::hmap;
use crate::pipelines::DescribedPipeline;

use super::Placement;

fn migrate_custom_v3_to_v4(prev_cmd: &CustomCommandV3) -> CustomCommand {
  CustomCommand {
    bash_c: prev_cmd.bash_c.clone(),
    placeholders: prev_cmd.placeholders.clone(),
    ignore_fails: prev_cmd.ignore_fails,
    show_success_output: prev_cmd.show_success_output,
    show_bash_c: prev_cmd.show_bash_c,
    only_when_fresh: prev_cmd.only_when_fresh,
    remote_exec: prev_cmd.remote_exec.clone(),
    daemon: None,
    replacements: if let Some(prev_repls) = &prev_cmd.replacements {
      let mut repl_grps = vec![];
      for i1 in prev_repls {
        let mut repls = vec![];
        for i2 in i1 {
          repls.push(Replacement {
            from: i2.0.clone(),
            to: migrate_var_v3_to_v4(&i2.1),
          });
        }
        repl_grps.push(ReplacementGroup { group: repls });
      }
      Some(repl_grps)
    } else {
      None
    },
  }
}

fn migrate_pls_v3_to_v4(prev_lang: &ProgrammingLanguageV3) -> ProgrammingLanguage {
  match prev_lang {
    ProgrammingLanguageV3::Rust => ProgrammingLanguage::Rust,
    ProgrammingLanguageV3::Go => ProgrammingLanguage::Go,
    ProgrammingLanguageV3::C => ProgrammingLanguage::C,
    ProgrammingLanguageV3::Cpp => ProgrammingLanguage::Cpp,
    ProgrammingLanguageV3::Python => ProgrammingLanguage::Python,
    ProgrammingLanguageV3::Other(v) => ProgrammingLanguage::Other(v.to_owned()),
  }
}

fn migrate_target_v3_to_v4(target: &TargetDescriptionV3) -> TargetDescription {
  TargetDescription {
    arch: target.arch.clone(),
    os: match &target.os {
      OsVariantV3::Android => OsVariant::Android,
      OsVariantV3::iOS => OsVariant::iOS,
      OsVariantV3::Linux => OsVariant::Linux,
      OsVariantV3::UnixLike(_) => OsVariant::UnixLike,
      OsVariantV3::Windows => OsVariant::Windows,
      OsVariantV3::macOS => OsVariant::macOS,
      OsVariantV3::Other(v) => OsVariant::Other(v.clone()),
    },
    os_derivative: target.derivative.clone(),
    os_version: match &target.version {
      OsVersionSpecificationV3::No => OsVersionSpecification::No,
      OsVersionSpecificationV3::Weak(w) => OsVersionSpecification::Weak { version: w.clone() },
      OsVersionSpecificationV3::Strong(s) => OsVersionSpecification::Strong { version: s.clone() },
    },
  }
}

fn migrate_req_v3_to_v4(prev_req: &RequirementV3) -> Requirement {
  match prev_req {
    RequirementV3::Exists(e) => Requirement::Exists { path: e.to_owned() },
    RequirementV3::ExistsAny(v) => Requirement::ExistsAny { paths: v.to_owned() },
    RequirementV3::CheckSuccess(c) => Requirement::CheckSuccess {
      action: TestAction {
        command: migrate_custom_v3_to_v4(&c.command),
        success_when_found: c.success_when_found.clone(),
        success_when_not_found: c.success_when_not_found.clone(),
      },
    },
    RequirementV3::RemoteAccessibleAndReady(h) => Requirement::RemoteAccessibleAndReady {
      remote_host_name: h.to_owned(),
    },
  }
}

fn migrate_var_v3_to_v4(prev_var: &VariableV3) -> Variable {
  Variable {
    title: prev_var.title.to_owned(),
    is_secret: prev_var.is_secret,
    value: match &prev_var.value {
      VarValueV3::Plain(p) => VarValue::Plain { value: p.to_owned() },
      VarValueV3::FromEnvFile(e) => VarValue::FromEnvFile(e.to_owned()),
      VarValueV3::FromEnvVar(ev) => VarValue::FromEnvVar {
        var_name: ev.to_owned(),
      },
      VarValueV3::FromHCVaultKv2(hc) => VarValue::FromHcVaultKv2(hc.to_owned()),
    },
  }
}

fn migrate_action_v3_to_v4(action: &DescribedActionV3) -> DescribedAction {
  DescribedAction {
    action: match &action.action {
      ActionV3::AddToStorage(a) => Action::AddToStorage(a.to_owned()),
      ActionV3::Build(b) => Action::Build(BuildAction {
        supported_langs: {
          let mut langs = vec![];
          for prev_lang in &b.supported_langs {
            langs.push(migrate_pls_v3_to_v4(prev_lang));
          }
          langs
        },
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &b.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
      }),
      ActionV3::Check(c) => Action::Test(TestAction {
        command: migrate_custom_v3_to_v4(&c.command),
        success_when_found: c.success_when_found.to_owned(),
        success_when_not_found: c.success_when_not_found.to_owned(),
      }),
      ActionV3::ConfigureDeploy(cd) => Action::ConfigureDeploy(ConfigureDeployAction {
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &cd.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
        deploy_toolkit: cd.deploy_toolkit.to_owned(),
      }),
      ActionV3::Custom(cu) => Action::Custom(migrate_custom_v3_to_v4(cu)),
      ActionV3::Deliver(del) => Action::Deliver(DeliveryAction {
        target: del.target.as_ref().map(migrate_target_v3_to_v4),
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &del.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
      }),
      ActionV3::Deploy(dep) => Action::Deploy(DeployAction {
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &dep.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
        deploy_toolkit: dep.deploy_toolkit.to_owned(),
      }),
      ActionV3::Install(ins) => Action::Install(InstallAction {
        target: ins.target.as_ref().map(migrate_target_v3_to_v4),
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &ins.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
      }),
      ActionV3::Interrupt => Action::Interrupt,
      ActionV3::Observe(obs) => Action::Observe(ObserveAction {
        command: migrate_custom_v3_to_v4(&obs.command),
      }),
      ActionV3::Pack(p) => Action::Pack(PackAction {
        target: p.target.as_ref().map(migrate_target_v3_to_v4),
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &p.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
      }),
      ActionV3::Patch(pa) => Action::Patch(pa.to_owned()),
      ActionV3::PostBuild(pb) => Action::PostBuild(PostBuildAction {
        supported_langs: {
          let mut langs = vec![];
          for prev_lang in &pb.supported_langs {
            langs.push(migrate_pls_v3_to_v4(prev_lang));
          }
          langs
        },
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &pb.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
      }),
      ActionV3::PreBuild(prb) => Action::PreBuild(PreBuildAction {
        supported_langs: {
          let mut langs = vec![];
          for prev_lang in &prb.supported_langs {
            langs.push(migrate_pls_v3_to_v4(prev_lang));
          }
          langs
        },
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &prb.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
      }),
      ActionV3::PostDeploy(ptd) => Action::PostDeploy(PostDeployAction {
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &ptd.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
        deploy_toolkit: ptd.deploy_toolkit.to_owned(),
      }),
      ActionV3::SyncFromRemote { remote_host_name: sfr } => Action::SyncToRemote {
        remote_host_name: sfr.to_owned(),
      },
      ActionV3::SyncToRemote { remote_host_name: stre } => Action::SyncToRemote {
        remote_host_name: stre.to_owned(),
      },
      ActionV3::Test(t) => Action::PostBuild(PostBuildAction {
        supported_langs: {
          let mut langs = vec![];
          for prev_lang in &t.supported_langs {
            langs.push(migrate_pls_v3_to_v4(prev_lang));
          }
          langs
        },
        commands: {
          let mut cmds = vec![];
          for prev_cmd in &t.commands {
            cmds.push(migrate_custom_v3_to_v4(prev_cmd));
          }
          cmds
        },
      }),
      ActionV3::UseFromStorage { content_info: u } => Action::UseFromStorage {
        content_info: u.to_owned(),
      },
      ActionV3::SubPipeline(p) => Action::SubPipeline(Box::new(migrate_pipe_v3_to_v4(p.as_ref()))),
    },
    desc: action.desc.to_owned(),
    exec_in_project_dir: action.exec_in_project_dir,
    info: action.info.to_owned(),
    requirements: if let Some(prev_reqs) = &action.requirements {
      let mut reqs = vec![];
      for prev_req in prev_reqs {
        reqs.push(migrate_req_v3_to_v4(prev_req));
      }
      Some(reqs)
    } else {
      None
    },
    tags: action.tags.to_owned(),
    title: action.title.to_owned(),
  }
}

fn migrate_pipe_v3_to_v4(prev_pipe: &DescribedPipelineV3) -> DescribedPipeline {
  let mut new_actions = vec![];
  for action in &prev_pipe.actions {
    new_actions.push(migrate_action_v3_to_v4(action));
  }
  DescribedPipeline {
    actions: new_actions,
    default: prev_pipe.default,
    desc: prev_pipe.desc.to_owned(),
    exclusive_exec_tag: prev_pipe.exclusive_exec_tag.to_owned(),
    info: prev_pipe.info.to_owned(),
    tags: prev_pipe.tags.to_owned(),
    title: prev_pipe.title.to_owned(),
    containered_opts: None,
  }
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
      let config = DeployerProjectOptionsV3 {
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
                new_actions.push(DescribedActionV3 {
                  action: match action.action.to_owned() {
                    ActionV2::AddToStorage(a) => ActionV3::AddToStorage(a),
                    ActionV2::Build(b) => ActionV3::Build(b),
                    ActionV2::Check(c) => ActionV3::Check(c),
                    ActionV2::ConfigureDeploy(cd) => ActionV3::ConfigureDeploy(cd),
                    ActionV2::Custom(cu) => ActionV3::Custom(cu),
                    ActionV2::Deliver(del) => ActionV3::Deliver(del),
                    ActionV2::Deploy(dep) => ActionV3::Deploy(dep),
                    ActionV2::Install(ins) => ActionV3::Install(ins),
                    ActionV2::Interrupt => ActionV3::Interrupt,
                    ActionV2::Observe(obs) => ActionV3::Observe(obs),
                    ActionV2::Pack(p) => ActionV3::Pack(p),
                    ActionV2::Patch(pa) => ActionV3::Patch(pa),
                    ActionV2::PostBuild(pb) => ActionV3::PostBuild(pb),
                    ActionV2::PreBuild(prb) => ActionV3::PreBuild(prb),
                    ActionV2::PostDeploy(ptd) => ActionV3::PostDeploy(ptd),
                    ActionV2::SyncFromRemote(sfr) => ActionV3::SyncToRemote { remote_host_name: sfr },
                    ActionV2::SyncToRemote(stre) => ActionV3::SyncToRemote { remote_host_name: stre },
                    ActionV2::Test(t) => ActionV3::Test(t),
                    ActionV2::UseFromStorage(u) => ActionV3::UseFromStorage { content_info: u },
                  },
                  desc: action.desc.to_owned(),
                  exec_in_project_dir: action.exec_in_project_dir,
                  info: action.info.to_owned(),
                  requirements: action.requirements.to_owned(),
                  tags: action.tags.to_owned(),
                  title: action.title.to_owned(),
                });
              }
              DescribedPipelineV3 {
                actions: new_actions,
                default: pipeline.default,
                desc: pipeline.desc.to_owned(),
                exclusive_exec_tag: pipeline.exclusive_exec_tag.to_owned(),
                info: pipeline.info.to_owned(),
                tags: pipeline.tags.to_owned(),
                title: pipeline.title.to_owned(),
                containered_opts: None,
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
      serde_json::from_reader::<_, DeployerProjectOptionsV3>(reader).map_err(|e| anyhow::anyhow!("{}", e))
    };

    let intermediate = if let Ok(intermediate) = intermediate
      && intermediate.version < 4
    {
      let config = DeployerProjectOptions {
        artifacts: intermediate.artifacts,
        cache_files: intermediate.cache_files,
        deploy_toolkit: intermediate.deploy_toolkit,
        langs: {
          let mut langs = vec![];
          for prev_lang in &intermediate.langs {
            langs.push(migrate_pls_v3_to_v4(prev_lang));
          }
          langs
        },
        pipelines: {
          let mut new_pipelines = vec![];
          for pipeline in &intermediate.pipelines {
            new_pipelines.push(migrate_pipe_v3_to_v4(pipeline));
          }
          new_pipelines
        },
        place_artifacts_into_project_root: {
          let mut placements = vec![];
          for prev_pl in &intermediate.place_artifacts_into_project_root {
            placements.push(Placement {
              from: prev_pl.0.clone(),
              to: prev_pl.1.clone(),
            });
          }
          placements
        },
        project_name: intermediate.project_name,
        targets: {
          let mut targets = vec![];
          for prev_target in &intermediate.targets {
            targets.push(migrate_target_v3_to_v4(prev_target));
          }
          targets
        },
        variables: {
          let mut vars = vec![];
          for prev_var in &intermediate.variables {
            vars.push(migrate_var_v3_to_v4(prev_var));
          }
          vars
        },
        version: 4,
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
      let mut config = DeployerGlobalConfigV3 {
        projects: intermediate.projects,
        remote_hosts: intermediate.remote_hosts,
        version: 3,
        ..Default::default()
      };

      let mut new_actions = hmap!();
      for action in &intermediate.actions_registry {
        new_actions.insert(
          action.0.to_owned(),
          DescribedActionV3 {
            action: match action.1.action.to_owned() {
              ActionV2::AddToStorage(a) => ActionV3::AddToStorage(a),
              ActionV2::Build(b) => ActionV3::Build(b),
              ActionV2::Check(c) => ActionV3::Check(c),
              ActionV2::ConfigureDeploy(cd) => ActionV3::ConfigureDeploy(cd),
              ActionV2::Custom(cu) => ActionV3::Custom(cu),
              ActionV2::Deliver(del) => ActionV3::Deliver(del),
              ActionV2::Deploy(dep) => ActionV3::Deploy(dep),
              ActionV2::Install(ins) => ActionV3::Install(ins),
              ActionV2::Interrupt => ActionV3::Interrupt,
              ActionV2::Observe(obs) => ActionV3::Observe(obs),
              ActionV2::Pack(p) => ActionV3::Pack(p),
              ActionV2::Patch(pa) => ActionV3::Patch(pa),
              ActionV2::PostBuild(pb) => ActionV3::PostBuild(pb),
              ActionV2::PreBuild(prb) => ActionV3::PreBuild(prb),
              ActionV2::PostDeploy(ptd) => ActionV3::PostDeploy(ptd),
              ActionV2::SyncFromRemote(sfr) => ActionV3::SyncToRemote { remote_host_name: sfr },
              ActionV2::SyncToRemote(stre) => ActionV3::SyncToRemote { remote_host_name: stre },
              ActionV2::Test(t) => ActionV3::Test(t),
              ActionV2::UseFromStorage(u) => ActionV3::UseFromStorage { content_info: u },
            },
            desc: action.1.desc.to_owned(),
            exec_in_project_dir: action.1.exec_in_project_dir,
            info: action.1.info.to_owned(),
            requirements: action.1.requirements.to_owned(),
            tags: action.1.tags.to_owned(),
            title: action.1.title.to_owned(),
          },
        );
      }
      config.actions_registry = new_actions;

      let mut new_pipelines = hmap!();
      for pipeline in &intermediate.pipelines_registry {
        new_pipelines.insert(pipeline.0.to_owned(), {
          let mut new_actions = vec![];
          for action in &pipeline.1.actions {
            new_actions.push(DescribedActionV3 {
              action: match action.action.to_owned() {
                ActionV2::AddToStorage(a) => ActionV3::AddToStorage(a),
                ActionV2::Build(b) => ActionV3::Build(b),
                ActionV2::Check(c) => ActionV3::Check(c),
                ActionV2::ConfigureDeploy(cd) => ActionV3::ConfigureDeploy(cd),
                ActionV2::Custom(cu) => ActionV3::Custom(cu),
                ActionV2::Deliver(del) => ActionV3::Deliver(del),
                ActionV2::Deploy(dep) => ActionV3::Deploy(dep),
                ActionV2::Install(ins) => ActionV3::Install(ins),
                ActionV2::Interrupt => ActionV3::Interrupt,
                ActionV2::Observe(obs) => ActionV3::Observe(obs),
                ActionV2::Pack(p) => ActionV3::Pack(p),
                ActionV2::Patch(pa) => ActionV3::Patch(pa),
                ActionV2::PostBuild(pb) => ActionV3::PostBuild(pb),
                ActionV2::PreBuild(prb) => ActionV3::PreBuild(prb),
                ActionV2::PostDeploy(ptd) => ActionV3::PostDeploy(ptd),
                ActionV2::SyncFromRemote(sfr) => ActionV3::SyncToRemote { remote_host_name: sfr },
                ActionV2::SyncToRemote(stre) => ActionV3::SyncToRemote { remote_host_name: stre },
                ActionV2::Test(t) => ActionV3::Test(t),
                ActionV2::UseFromStorage(u) => ActionV3::UseFromStorage { content_info: u },
              },
              desc: action.desc.to_owned(),
              exec_in_project_dir: action.exec_in_project_dir,
              info: action.info.to_owned(),
              requirements: action.requirements.to_owned(),
              tags: action.tags.to_owned(),
              title: action.title.to_owned(),
            });
          }
          DescribedPipelineV3 {
            actions: new_actions,
            default: pipeline.1.default,
            desc: pipeline.1.desc.to_owned(),
            exclusive_exec_tag: pipeline.1.exclusive_exec_tag.to_owned(),
            info: pipeline.1.info.to_owned(),
            tags: pipeline.1.tags.to_owned(),
            title: pipeline.1.title.to_owned(),
            containered_opts: None,
          }
        });
      }
      config.pipelines_registry = new_pipelines;

      Ok(config)
    } else {
      let file = std::fs::File::open(path)?;
      let reader = std::io::BufReader::new(file);
      serde_json::from_reader::<_, DeployerGlobalConfigV3>(reader).map_err(|e| anyhow::anyhow!("{}", e))
    };

    let intermediate = if let Ok(intermediate) = intermediate
      && intermediate.version < 4
    {
      let mut config = DeployerGlobalConfig {
        projects: intermediate.projects,
        remote_hosts: intermediate.remote_hosts,
        version: 4,
        ..Default::default()
      };

      let mut new_actions = hmap!();
      for action in &intermediate.actions_registry {
        new_actions.insert(action.0.to_owned(), migrate_action_v3_to_v4(action.1));
      }
      config.actions_registry = new_actions;

      let mut new_pipelines = hmap!();
      for pipeline in &intermediate.pipelines_registry {
        new_pipelines.insert(pipeline.0.to_owned(), {
          let mut new_actions = vec![];
          for action in &pipeline.1.actions {
            new_actions.push(migrate_action_v3_to_v4(action));
          }
          DescribedPipeline {
            actions: new_actions,
            default: pipeline.1.default,
            desc: pipeline.1.desc.to_owned(),
            exclusive_exec_tag: pipeline.1.exclusive_exec_tag.to_owned(),
            info: pipeline.1.info.to_owned(),
            tags: pipeline.1.tags.to_owned(),
            title: pipeline.1.title.to_owned(),
            containered_opts: None,
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
