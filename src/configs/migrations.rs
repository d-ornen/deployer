//! Migrations module.

/// Structures for `v2` -> `v3` migration.
pub(crate) mod project_config_v2_to_v3_migrations {
  use serde::{Deserialize, Serialize};
  use std::collections::{HashMap, HashSet};
  use std::path::PathBuf;

  use super::project_config_v3_to_v4_migrations::*;
  use crate::actions::{patch::PatchAction, storage_add::AddToStorageAction};
  use crate::entities::{
    info::{ActionInfo, ContentInfo, Info, PipelineInfo, ShortName, info2str, str2info, str2info_wl},
    remote_host::RemoteHost,
  };
  use crate::utils::{ordered_map, ordered_set};
  use crate::{hmap, hset};

  #[derive(Deserialize, Serialize, PartialEq)]
  pub struct DeployerProjectOptionsV2 {
    pub project_name: String,
    pub langs: Vec<ProgrammingLanguageV3>,
    pub targets: Vec<TargetDescriptionV3>,
    pub deploy_toolkit: Option<String>,
    #[serde(serialize_with = "ordered_set")]
    pub cache_files: HashSet<PathBuf>,
    pub pipelines: Vec<DescribedPipelineV2>,
    pub artifacts: Vec<PathBuf>,
    pub variables: Vec<VariableV3>,
    pub place_artifacts_into_project_root: Vec<(PathBuf, PathBuf)>,
    #[serde(default = "get_default_project_conf_version")]
    pub version: u8,
  }

  impl Default for DeployerProjectOptionsV2 {
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
        version: get_default_project_conf_version(),
      }
    }
  }

  fn get_default_project_conf_version() -> u8 {
    2
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  pub struct DescribedPipelineV2 {
    pub title: String,
    pub desc: String,
    #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
    pub info: PipelineInfo,
    pub tags: Vec<String>,
    pub actions: Vec<DescribedActionV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_exec_tag: Option<String>,
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  pub struct DescribedActionV2 {
    pub title: String,
    pub desc: String,
    #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
    pub info: ActionInfo,
    pub tags: Vec<String>,
    pub action: ActionV2,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<RequirementV3>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_in_project_dir: Option<bool>,
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  pub enum ActionV2 {
    Interrupt,
    SyncToRemote(ShortName),
    SyncFromRemote(ShortName),
    Custom(CustomCommandV3),
    Check(CheckActionV3),
    PreBuild(PreBuildActionV3),
    Build(BuildActionV3),
    PostBuild(PostBuildActionV3),
    Test(TestActionV3),
    Pack(PackActionV3),
    Deliver(DeliveryActionV3),
    Install(InstallActionV3),
    ConfigureDeploy(ConfigureDeployActionV3),
    Deploy(DeployActionV3),
    PostDeploy(PostDeployActionV3),
    Observe(ObserveActionV3),
    #[serde(serialize_with = "info2str", deserialize_with = "str2info_wl")]
    UseFromStorage(ContentInfo),
    AddToStorage(AddToStorageAction),
    Patch(PatchAction),
  }

  #[derive(Deserialize, Serialize)]
  pub struct DeployerGlobalConfigV2 {
    pub projects: Vec<String>,
    #[serde(serialize_with = "ordered_map")]
    pub actions_registry: HashMap<Info, DescribedActionV2>,
    #[serde(serialize_with = "ordered_map")]
    pub pipelines_registry: HashMap<Info, DescribedPipelineV2>,
    #[serde(serialize_with = "ordered_map")]
    pub remote_hosts: HashMap<ShortName, RemoteHost>,
    #[serde(default = "get_default_global_conf_version")]
    pub version: u8,
  }

  impl Default for DeployerGlobalConfigV2 {
    fn default() -> Self {
      Self {
        projects: vec![],
        actions_registry: hmap!(),
        pipelines_registry: hmap!(),
        remote_hosts: hmap!(),
        version: get_default_global_conf_version(),
      }
    }
  }

  fn get_default_global_conf_version() -> u8 {
    2
  }
}

/// Structures for `v3` -> `v4` migration.
pub(crate) mod project_config_v3_to_v4_migrations {
  use regex::Regex;
  use serde::{Deserialize, Serialize};
  use std::collections::{HashMap, HashSet};
  use std::path::PathBuf;

  use crate::actions::{patch::PatchAction, storage_add::AddToStorageAction};
  use crate::entities::{
    containered_opts::ContaineredOpts,
    info::{ActionInfo, ContentInfo, Info, PipelineInfo, ShortName, info2str, str2info, str2info_wl},
    remote_host::RemoteHost,
    variables::{FromEnvFile, Kv2Paths},
  };
  use crate::utils::{ordered_map, ordered_set, regexopt2str, str2regexopt};
  use crate::{hmap, hset};

  #[derive(Deserialize, Serialize, PartialEq)]
  pub struct DeployerProjectOptionsV3 {
    pub project_name: String,
    pub langs: Vec<ProgrammingLanguageV3>,
    pub targets: Vec<TargetDescriptionV3>,
    pub deploy_toolkit: Option<String>,
    #[serde(serialize_with = "ordered_set")]
    pub cache_files: HashSet<PathBuf>,
    pub pipelines: Vec<DescribedPipelineV3>,
    pub artifacts: Vec<PathBuf>,
    pub variables: Vec<VariableV3>,
    pub place_artifacts_into_project_root: Vec<(PathBuf, PathBuf)>,
    #[serde(default = "get_default_project_conf_version")]
    pub version: u8,
  }

  impl Default for DeployerProjectOptionsV3 {
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
        version: get_default_project_conf_version(),
      }
    }
  }

  pub fn get_default_project_conf_version() -> u8 {
    3
  }

  #[derive(Deserialize, Serialize)]
  pub struct DeployerGlobalConfigV3 {
    pub projects: Vec<String>,
    #[serde(serialize_with = "ordered_map")]
    pub actions_registry: HashMap<Info, DescribedActionV3>,
    #[serde(serialize_with = "ordered_map")]
    pub pipelines_registry: HashMap<Info, DescribedPipelineV3>,
    #[serde(serialize_with = "ordered_map")]
    pub remote_hosts: HashMap<ShortName, RemoteHost>,
    #[serde(default = "get_default_global_conf_version")]
    pub version: u8,
  }

  impl Default for DeployerGlobalConfigV3 {
    fn default() -> Self {
      Self {
        projects: vec![],
        actions_registry: hmap!(),
        pipelines_registry: hmap!(),
        remote_hosts: hmap!(),
        version: get_default_global_conf_version(),
      }
    }
  }

  pub fn get_default_global_conf_version() -> u8 {
    3
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  pub struct DescribedPipelineV3 {
    pub title: String,
    pub desc: String,
    #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
    pub info: PipelineInfo,
    pub tags: Vec<String>,
    pub actions: Vec<DescribedActionV3>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containered_opts: Option<ContaineredOpts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_exec_tag: Option<String>,
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  pub struct DescribedActionV3 {
    pub title: String,
    pub desc: String,
    #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
    pub info: ActionInfo,
    pub tags: Vec<String>,
    pub action: ActionV3,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<RequirementV3>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_in_project_dir: Option<bool>,
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  #[serde(rename_all = "snake_case", tag = "type")]
  pub enum ActionV3 {
    Interrupt,
    SyncToRemote {
      remote_host_name: ShortName,
    },
    SyncFromRemote {
      remote_host_name: ShortName,
    },
    Custom(CustomCommandV3),
    Check(CheckActionV3),
    PreBuild(PreBuildActionV3),
    Build(BuildActionV3),
    PostBuild(PostBuildActionV3),
    Test(TestActionV3),
    Pack(PackActionV3),
    Deliver(DeliveryActionV3),
    Install(InstallActionV3),
    ConfigureDeploy(ConfigureDeployActionV3),
    Deploy(DeployActionV3),
    PostDeploy(PostDeployActionV3),
    Observe(ObserveActionV3),
    UseFromStorage {
      #[serde(serialize_with = "info2str", deserialize_with = "str2info_wl")]
      content_info: ContentInfo,
    },
    AddToStorage(AddToStorageAction),
    Patch(PatchAction),
    SubPipeline(Box<DescribedPipelineV3>),
  }

  #[derive(Deserialize, Serialize, PartialEq, Default, Clone)]
  pub struct BuildActionV3 {
    pub supported_langs: Vec<ProgrammingLanguageV3>,
    pub commands: Vec<CustomCommandV3>,
  }

  pub type PreBuildActionV3 = BuildActionV3;
  pub type PostBuildActionV3 = BuildActionV3;
  pub type TestActionV3 = BuildActionV3;

  #[derive(Deserialize, Serialize, Clone)]
  pub struct CheckActionV3 {
    pub command: CustomCommandV3,
    #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt")]
    pub success_when_found: Option<Regex>,
    #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt")]
    pub success_when_not_found: Option<Regex>,
  }

  impl Eq for CheckActionV3 {}

  impl std::hash::Hash for CheckActionV3 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
      self.command.hash(state);
      if let Some(succ_found) = &self.success_when_found {
        succ_found.as_str().hash(state);
      }
      if let Some(succ_not_found) = &self.success_when_not_found {
        succ_not_found.as_str().hash(state);
      }
    }
  }

  impl PartialEq for CheckActionV3 {
    fn eq(&self, other: &Self) -> bool {
      self.command.eq(&other.command)
        && ((self.success_when_found.is_none() && other.success_when_found.is_none())
          || (self.success_when_found.as_ref().is_some_and(|a| {
            other
              .success_when_found
              .as_ref()
              .is_some_and(|b| a.as_str().eq(b.as_str()))
          })))
        && ((self.success_when_not_found.is_none() && other.success_when_not_found.is_none())
          || (self.success_when_not_found.as_ref().is_some_and(|a| {
            other
              .success_when_not_found
              .as_ref()
              .is_some_and(|b| a.as_str().eq(b.as_str()))
          })))
    }
  }

  #[derive(Deserialize, Serialize, PartialEq, Default, Clone)]
  pub struct PackActionV3 {
    pub target: Option<TargetDescriptionV3>,
    pub commands: Vec<CustomCommandV3>,
  }

  pub type DeliveryActionV3 = PackActionV3;
  pub type InstallActionV3 = PackActionV3;

  #[derive(Deserialize, Serialize, PartialEq, Default, Clone)]
  pub struct DeployActionV3 {
    pub deploy_toolkit: Option<String>,
    pub commands: Vec<CustomCommandV3>,
  }

  pub type ConfigureDeployActionV3 = DeployActionV3;
  pub type PostDeployActionV3 = DeployActionV3;

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  pub struct ObserveActionV3 {
    pub command: CustomCommandV3,
  }

  #[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
  pub struct CustomCommandV3 {
    pub bash_c: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholders: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacements: Option<Vec<Vec<(String, VariableV3)>>>,
    pub ignore_fails: bool,
    pub show_success_output: bool,
    pub show_bash_c: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_when_fresh: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_exec: Option<Vec<ShortName>>,
  }

  #[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
  pub enum ProgrammingLanguageV3 {
    Rust,
    Go,
    C,
    Cpp,
    Python,
    Other(String),
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
  pub struct TargetDescriptionV3 {
    pub arch: String,
    pub os: OsVariantV3,
    pub derivative: String,
    pub version: OsVersionSpecificationV3,
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
  pub enum OsVariantV3 {
    Android,
    #[allow(non_camel_case_types)]
    iOS,
    Linux,
    UnixLike(String),
    Windows,
    #[allow(non_camel_case_types)]
    macOS,
    Other(String),
  }

  #[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
  pub enum OsVersionSpecificationV3 {
    #[default]
    No,
    Weak(String),
    Strong(String),
  }

  #[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
  pub struct VariableV3 {
    pub title: String,
    pub is_secret: bool,
    pub value: VarValueV3,
  }

  #[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
  pub enum VarValueV3 {
    Plain(String),
    FromEnvFile(FromEnvFile),
    FromEnvVar(String),
    FromHCVaultKv2(Kv2Paths),
  }

  #[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
  pub enum RequirementV3 {
    Exists(PathBuf),
    ExistsAny(Vec<PathBuf>),
    CheckSuccess(CheckActionV3),
    RemoteAccessibleAndReady(ShortName),
  }
}
