use serde::{Deserialize, Serialize};

use crate::entities::environment::RunEnvironment;
use crate::entities::info::ContentInfo;
use crate::storage::use_from_storage;

/// Options for containered runs.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct ContaineredOpts {
  /// Base image tag.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub base_image: Option<String>,

  /// Set of commands to install on a given base image.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub preflight_cmds: Option<Vec<String>>,

  /// Base Deployer build image tag.
  ///
  /// It should contain `cargo` and `rustc` to compile Deployer.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub build_deployer_base_image: Option<String>,

  /// Commands to:
  ///
  /// 1. Install `git` to clone `deployer`.
  /// 2. Install `libpython3.xx.so.*` on a given base image (to build `deployer` itself with `python` feature enabled).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub preflight_deployer_build_deps: Option<String>,

  /// Commands to build Deployer itself.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deployer_build_cmds: Option<Vec<String>>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub run_strategies: Option<Vec<ContainerizedRunStrategy>>,
}

impl ContaineredOpts {
  pub fn sync_fake_content(&self, env: RunEnvironment) -> anyhow::Result<()> {
    if let Some(strategies) = &self.run_strategies {
      for strategy in strategies {
        strategy.sync_fake_content(env)?;
      }
      Ok(())
    } else {
      Ok(())
    }
  }

  pub fn concat_strategies(&self) -> Option<String> {
    if let Some(strategies) = &self.run_strategies {
      let mut strs = vec![];
      for strategy in strategies {
        strs.push(strategy.concat());
      }
      Some(strs.join("\n"))
    } else {
      None
    }
  }
}

/// Run strategy.
///
/// Docker (and other container build environments) supports build cache, which allows you to
/// specify your build strategy.
///
/// For example, you can use two-stage builds for Rust projects.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct ContainerizedRunStrategy {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub fake_content: Option<ContentInfo>,
  pub copy_cmds: Vec<String>,
  pub pre_cache_cmds: Vec<String>,
}

impl ContainerizedRunStrategy {
  fn sync_fake_content(&self, env: RunEnvironment) -> anyhow::Result<()> {
    if let Some(content_info) = &self.fake_content {
      use_from_storage(env.storage_dir, env.run_dir, content_info)?;
    }
    Ok(())
  }

  fn concat(&self) -> String {
    self.copy_cmds.join("\n") + "\n" + self.pre_cache_cmds.join("\n").as_str()
  }
}
