pub(crate) mod project_config_v2_to_v3_migrations {
  use serde::{Deserialize, Serialize};
  use std::collections::{HashMap, HashSet};
  use std::path::PathBuf;

  use crate::actions::{
    buildlike::*, check::CheckAction, deploylike::*, observe::ObserveAction, packlike::*, patch::PatchAction,
    storage_add::AddToStorageAction,
  };
  use crate::entities::{
    custom_command::CustomCommand,
    info::{ActionInfo, ContentInfo, Info, PipelineInfo, ShortName, info2str, str2info, str2info_wl},
    programming_languages::ProgrammingLanguage,
    remote_host::RemoteHost,
    requirements::Requirement,
    targets::TargetDescription,
    variables::Variable,
  };
  use crate::utils::{ordered_map, ordered_set};
  use crate::{hmap, hset};

  #[derive(Deserialize, Serialize, PartialEq)]
  pub struct DeployerProjectOptionsV2 {
    pub project_name: String,
    pub langs: Vec<ProgrammingLanguage>,
    pub targets: Vec<TargetDescription>,
    pub deploy_toolkit: Option<String>,
    #[serde(serialize_with = "ordered_set")]
    pub cache_files: HashSet<PathBuf>,
    pub pipelines: Vec<DescribedPipelineV2>,
    pub artifacts: Vec<PathBuf>,
    pub variables: Vec<Variable>,
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
        // containered_options: None,
        version: 2,
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
    pub requirements: Option<Vec<Requirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_in_project_dir: Option<bool>,
  }

  #[derive(Deserialize, Serialize, PartialEq, Clone)]
  pub enum ActionV2 {
    Interrupt,
    SyncToRemote(ShortName),
    SyncFromRemote(ShortName),
    Custom(CustomCommand),
    Check(CheckAction),
    PreBuild(PreBuildAction),
    Build(BuildAction),
    PostBuild(PostBuildAction),
    Test(TestAction),
    Pack(PackAction),
    Deliver(DeliveryAction),
    Install(InstallAction),
    ConfigureDeploy(ConfigureDeployAction),
    Deploy(DeployAction),
    PostDeploy(PostDeployAction),
    Observe(ObserveAction),
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

  fn get_default_global_conf_version() -> u8 {
    2
  }

  impl Default for DeployerGlobalConfigV2 {
    fn default() -> Self {
      Self {
        projects: vec![],
        actions_registry: hmap!(),
        pipelines_registry: hmap!(),
        remote_hosts: hmap!(),
        version: 2,
      }
    }
  }
}
