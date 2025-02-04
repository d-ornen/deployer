use serde::{Deserialize, Serialize};

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
}
