//! Pipelines module.
//!
//! Pipeline is a list of Actions.

#[cfg(feature = "tui")]
use colored::Colorize;
use serde::{Deserialize, Serialize};
#[cfg(feature = "tui")]
use std::process::exit;

use crate::actions::Action;
use crate::actions::DescribedAction;
#[cfg(feature = "tui")]
use crate::cmd::{CatPipelineArgs, CatProjectArgs, NewPipelineArgs, WithPipelineArgs};
#[cfg(feature = "tui")]
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::entities::containered_opts::ContaineredOpts;
#[cfg(feature = "tui")]
use crate::entities::info::StrToInfo;
use crate::entities::info::{PipelineInfo, info2str, str2info};
#[cfg(feature = "tui")]
use crate::hmap;
#[cfg(feature = "tui")]
use crate::i18n;
#[cfg(feature = "tui")]
use crate::rw::read_checked;
#[cfg(feature = "tui")]
use crate::tui::setup::specify_pipeline_short_name;

/// Described Pipeline.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct DescribedPipeline {
  /// Pipeline name.
  ///
  /// When you're using project's assigned Pipelines, make sure that
  /// title is simple enough to use it as argument to `deployer build name1,name2,name3,..`.
  pub title: String,

  /// Pipeline description.
  pub desc: String,

  /// Short name and version.
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub info: PipelineInfo,

  /// List of tags (to use with `grep` when searching through `deployer ls pipelines`).
  pub tags: Vec<String>,

  /// Used Actions with execution order.
  pub actions: Vec<DescribedAction>,

  /// This field is used only in projects.
  ///
  /// If set to `true`, and no Pipeline specified to `deployer build`, runs automatically.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default: Option<bool>,

  /// Indicates that this Pipeline should be executed inside containered environment.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub containered_opts: Option<ContaineredOpts>,

  /// Specify exclusive execution tag to save unique cache.
  ///
  /// If set to any string, Deployer will perform this Pipeline only in special build folder.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exclusive_exec_tag: Option<String>,
}

impl DescribedPipeline {
  pub fn return_all_cmds(&self) -> Vec<String> {
    let mut cmds = vec![];

    for action in &self.actions {
      match &action.action {
        Action::Interrupt => {}
        Action::SyncToRemote { .. } => cmds.push("<sync-to-remote>".to_string()),
        Action::SyncFromRemote { .. } => cmds.push("<sync-from-remote>".to_string()),
        Action::Custom(cmd) => cmds.push(cmd.bash_c.to_owned()),
        Action::Check(check) => cmds.push(format!("<check> {}", check.command.bash_c)),
        Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => cmds.extend_from_slice(
          a.commands
            .iter()
            .map(|c| c.bash_c.to_owned())
            .collect::<Vec<_>>()
            .as_slice(),
        ),
        Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => cmds.extend_from_slice(
          a.commands
            .iter()
            .map(|c| c.bash_c.to_owned())
            .collect::<Vec<_>>()
            .as_slice(),
        ),
        Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => cmds.extend_from_slice(
          a.commands
            .iter()
            .map(|c| c.bash_c.to_owned())
            .collect::<Vec<_>>()
            .as_slice(),
        ),
        Action::Observe(a) => cmds.push(format!("<observe> {}", a.command.bash_c)),
        Action::UseFromStorage { .. } => cmds.push("<use-from-storage>".to_string()),
        Action::AddToStorage(_) => cmds.push("<add-to-storage>".to_string()),
        Action::Patch(p) => cmds.push(format!("<patch-with-file> {:?}", p.patch)),
        Action::SubPipeline(sp) => {
          cmds.push(format!("<sub-pipeline {}>", sp.info.to_str()));
          let sp_cmds = sp
            .return_all_cmds()
            .iter()
            .map(|v| format!("  {}", v))
            .collect::<Vec<_>>();
          cmds.extend_from_slice(&sp_cmds);
          cmds.push("</sub-pipeline>".to_string());
        }
      }
    }

    cmds
  }
}

/// Lists all available Pipelines.
#[cfg(feature = "tui")]
pub fn list_pipelines(globals: &DeployerGlobalConfig) -> anyhow::Result<()> {
  println!("{}", i18n::PIPELINES_AVAILABLE);

  let mut pipelines = globals.pipelines_registry.values().collect::<Vec<_>>();
  pipelines.sort_by_key(|a| a.info.to_str());

  for pipeline in pipelines {
    let pipeline_info = pipeline.info.to_str();
    let pipeline_title = format!("[{}]", pipeline.title);
    let tags = if pipeline.tags.is_empty() {
      String::new()
    } else {
      format!(
        " ({}: {})",
        i18n::TAGS,
        pipeline.tags.join(", ").as_str().blue().italic()
      )
    };
    println!(
      "• {} {}{}",
      pipeline_info.blue().bold(),
      pipeline_title.green().bold(),
      tags
    );
    if !pipeline.desc.is_empty() {
      println!("\t> {}", pipeline.desc.green().italic());
    }
  }

  Ok(())
}

/// Creates a new Pipeline.
#[cfg(feature = "tui")]
pub fn new_pipeline(globals: &mut DeployerGlobalConfig, args: &NewPipelineArgs) -> anyhow::Result<DescribedPipeline> {
  if let Some(from_file) = &args.from {
    let pipeline = read_checked::<DescribedPipeline>(from_file)
      .map_err(|e| {
        panic!("Can't read provided Pipeline file due to: {}", e);
      })
      .unwrap();
    globals
      .pipelines_registry
      .insert(pipeline.info.clone(), pipeline.clone());
    return Ok(pipeline);
  }

  let described_pipeline = DescribedPipeline::new_from_prompt(globals)?;

  if globals.pipelines_registry.contains_key(&described_pipeline.info)
    && !inquire::Confirm::new(&i18n::PIPELINE_REG_ALREADY_HAVE.replace("{}", &described_pipeline.info.to_str()))
      .prompt()?
  {
    return Ok(described_pipeline);
  }

  globals
    .pipelines_registry
    .insert(described_pipeline.info.clone(), described_pipeline.clone());

  Ok(described_pipeline)
}

/// Removes a Pipeline.
#[cfg(feature = "tui")]
pub fn remove_pipeline(globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
  use inquire::{Confirm, Select};

  if globals.pipelines_registry.is_empty() {
    println!("{}", i18n::NO_PIPELINES);
    return Ok(());
  }

  let (pipelines, keys) = {
    let mut h = hmap!();
    let mut k = vec![];

    for key in globals.pipelines_registry.keys() {
      let pipeline = globals.pipelines_registry.get(key).unwrap();
      let new_key = format!("{} - {}", pipeline.info.to_str(), pipeline.title);
      h.insert(new_key.clone(), pipeline);
      k.push(new_key);
    }

    k.sort();

    (h, k)
  };

  let selected_pipeline = Select::new(i18n::PIPELINE_REGISTRY_CHOOSE_TO_REMOVE, keys).prompt()?;
  let pipeline = *pipelines.get(&selected_pipeline).unwrap();
  let info = pipeline.info.clone();

  if !Confirm::new(i18n::ARE_YOU_SURE).prompt()? {
    return Ok(());
  }

  globals.pipelines_registry.remove(&info);

  Ok(())
}

/// Prints a Pipeline as JSON.
#[cfg(feature = "tui")]
pub fn cat_pipeline(globals: &DeployerGlobalConfig, args: &CatPipelineArgs) -> anyhow::Result<()> {
  let pipeline = match globals
    .pipelines_registry
    .get(&args.pipeline_short_info_and_version.to_info()?)
  {
    None => exit(1),
    Some(pipeline) => pipeline,
  };

  let pipeline_json = serde_json::to_string_pretty(&pipeline).unwrap();
  println!("{}", pipeline_json);

  Ok(())
}

/// Prints all project Pipelines as JSON.
///
/// If you specify `cat_all_shell_commands` option (`deployer cat project -n`),
/// Deployer will print all shell commands of all project Pipelines.
#[cfg(feature = "tui")]
pub fn cat_project_pipelines(config: &DeployerProjectOptions, args: CatProjectArgs) -> anyhow::Result<()> {
  for pipeline in &config.pipelines {
    if !args.cat_all_shell_commands {
      let pipeline_json = serde_json::to_string_pretty(&pipeline).unwrap();
      println!("{}", pipeline_json);
    } else {
      let cmds = pipeline.return_all_cmds();
      println!("{} `{}`:", i18n::PIPELINE, pipeline.title.blue().italic());
      for cmd in cmds {
        println!(">>> {}", cmd.green());
      }
    }
  }

  Ok(())
}

/// Reorders Pipelines.
#[cfg(feature = "tui")]
fn reorder_pipelines_in_project(pipelines_unordered: Vec<DescribedPipeline>) -> anyhow::Result<Vec<DescribedPipeline>> {
  use inquire::ReorderableList;

  let mut h = hmap!();
  let mut k = vec![];

  for pipeline in pipelines_unordered {
    let key = format!("{} - {}", pipeline.info.to_str(), pipeline.title);
    k.push(key.clone());
    h.insert(key, pipeline);
  }

  let reordered = ReorderableList::new(i18n::PIPELINES_REORDER, k).prompt()?;

  let mut pipelines_ordered = vec![];
  for key in reordered {
    pipelines_ordered.push((*h.get(&key).unwrap()).clone());
  }

  Ok(pipelines_ordered)
}

/// Tries to assign and setup the Pipeline from Registry.
///
/// While setup, Deployer checks programming languages, target specs and
/// deploy toolkit to make sure that Pipeline is compatible with your project.
#[cfg(feature = "tui")]
pub fn assign_pipeline_to_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  args: &WithPipelineArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() {
    panic!("{}", i18n::CFG_INVALID);
  }

  let mut pipeline = if let Some(tag) = &args.tag {
    globals
      .pipelines_registry
      .get(&tag.to_info()?)
      .ok_or_else(|| anyhow::anyhow!(i18n::NO_SUCH_PIPELINE))?
      .clone()
  } else if !globals.pipelines_registry.is_empty() {
    const NEW_PIPELINE: &str = i18n::PIPELINE_SPECIFY_ANOTHER;

    let mut ptags = hmap!();
    let mut tags = vec![];

    globals
      .pipelines_registry
      .iter()
      .map(|(k, v)| {
        (
          format!("`{}` - {}", k.to_str().blue().bold(), v.title.green().bold()),
          v,
        )
      })
      .for_each(|(t, p)| {
        tags.push(t.clone());
        ptags.insert(t, p);
      });
    tags.push(NEW_PIPELINE.to_string());

    let selected = inquire::Select::new(i18n::PIPELINE_SELECT_FOR_PROJECT, tags).prompt()?;

    if selected.as_str() == NEW_PIPELINE {
      DescribedPipeline::new_from_prompt(globals)?
    } else {
      let pipeline = ptags.get(&selected).ok_or(anyhow::anyhow!(i18n::NO_SUCH_PIPELINE))?;
      (*pipeline).clone()
    }
  } else {
    DescribedPipeline::new_from_prompt(globals)?
  };

  for action in &mut pipeline.actions {
    *action = action.prompt_setup_for_project(
      &config.langs,
      &config.deploy_toolkit,
      &config.targets,
      &config.variables,
      &config.artifacts,
    )?;
  }

  let short_name = if let Some(short_name) = args.r#as.as_ref() {
    short_name.to_owned()
  } else {
    inquire::Text::new(i18n::PIPELINE_SHORT_NAME_FOR_PROJECT).prompt()?
  };

  pipeline.desc = format!(
    r#"{} `{}`.{}{}"#,
    i18n::GOT_FROM,
    pipeline.title,
    if pipeline.desc.is_empty() { "" } else { " " },
    pipeline.desc
  );
  pipeline.title = short_name.clone();

  if specify_pipeline_short_name(config, &mut pipeline.title).is_err() {
    return Ok(());
  };

  if let Some(old_default) = config.pipelines.iter_mut().find(|p| p.default.is_some_and(|v| v)) {
    if inquire::Confirm::new(&i18n::PIPELINE_NEW_DEFAULT_REPLACE.replace("{}", old_default.title.as_str())).prompt()? {
      old_default.default = None;
      pipeline.default = Some(true);
    }
  } else if inquire::Confirm::new(i18n::PIPELINE_NEW_DEFAULT).prompt()? {
    pipeline.default = Some(true);
  }

  if let Some(i) = config.pipelines.iter().position(|p| p.title.as_str() == short_name) {
    config.pipelines.remove(i);
  }
  config.pipelines.push(pipeline);

  if config.pipelines.len() >= 2 {
    config.pipelines = reorder_pipelines_in_project(config.pipelines.clone())?;
  }

  println!("{}", i18n::PIPELINE_DEFAULT_SET);

  Ok(())
}

/// Edits the Pipeline.
#[cfg(feature = "tui")]
pub fn edit_pipeline(globals: &mut DeployerGlobalConfig, args: &CatPipelineArgs) -> anyhow::Result<()> {
  let info = args.pipeline_short_info_and_version.to_info()?;

  let mut pipeline = match globals.pipelines_registry.contains_key(&info) {
    false => panic!("There is no such Pipeline!"),
    true => {
      let pipeline = globals.pipelines_registry.get(&info).unwrap().clone();
      globals.pipelines_registry.remove(&info);
      pipeline
    }
  };

  pipeline.edit_pipeline_from_prompt(globals)?;
  globals.pipelines_registry.insert(pipeline.info.clone(), pipeline);

  Ok(())
}
