use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process::exit;

use crate::actions::DescribedAction;
use crate::cmd::{NewPipelineArgs, CatPipelineArgs, WithPipelineArgs};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::entities::info::{PipelineInfo, info2str, str2info};
use crate::hmap;
use crate::i18n;
use crate::rw::read_checked;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) struct DescribedPipeline {
  /// Заголовок Пайплайна.
  pub(crate) title: String,
  /// Описание Пайплайна.
  pub(crate) desc: String,
  /// Короткое имя и версия.
  #[serde(serialize_with = "info2str", deserialize_with = "str2info")]
  pub(crate) info: PipelineInfo,
  /// Список меток для фильтрации Действий при выборе из Реестра.
  pub(crate) tags: Vec<String>,
  pub(crate) actions: Vec<DescribedAction>,
  /// Информация для проекта: запускать ли Пайплайн по умолчанию.
  /// 
  /// Если не установлен, считается как `false`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) default: Option<bool>,
  /// Информация для проекта: должен ли пайплайн выполняться в определённой среде (например, в отдельных папках сборки).
  /// 
  /// Если зависит, то пайплайн будет выполняться в папках с указанным тегом сборки.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) exclusive_exec_tag: Option<String>,
}

/// Перечисляет все доступные пайплайны.
pub(crate) fn list_pipelines(
  globals: &DeployerGlobalConfig,
) -> anyhow::Result<()> {
  println!("{}", i18n::PIPELINES_AVAILABLE);
  
  let mut pipelines = globals.pipelines_registry.values().collect::<Vec<_>>();
  pipelines.sort_by_key(|a| a.info.to_str());
  
  for pipeline in pipelines {
    let pipeline_info = pipeline.info.to_str();
    let pipeline_title = format!("[{}]", pipeline.title);
    let tags = if pipeline.tags.is_empty() { String::new() } else { format!(" ({}: {})", i18n::TAGS, pipeline.tags.join(", ").as_str().blue().italic()) };
    println!("• {} {}{}", pipeline_info.blue().bold(), pipeline_title.green().bold(), tags);
    if !pipeline.desc.is_empty() { println!("\t> {}", pipeline.desc.green().italic()); }
  }
  
  Ok(())
}

/// Создаёт новый пайплайн.
pub(crate) fn new_pipeline(
  globals: &mut DeployerGlobalConfig,
  args: &NewPipelineArgs,
) -> anyhow::Result<()> {
  if let Some(from_file) = &args.from {
    let pipeline = read_checked::<DescribedPipeline>(from_file).map_err(|e| {
      panic!("Can't read provided Pipeline file due to: {}", e);
    }).unwrap();
    globals.pipelines_registry.insert(pipeline.info.to_str(), pipeline);
    return Ok(())
  }
  
  let described_pipeline = DescribedPipeline::new_from_prompt(globals)?;
  
  if
    globals.pipelines_registry.contains_key(&described_pipeline.info.to_str()) &&
    !inquire::Confirm::new(&i18n::PIPELINE_REG_ALREADY_HAVE.replace("{}", &described_pipeline.info.to_str())).prompt()?
  {
    return Ok(())
  }
  
  globals.pipelines_registry.insert(described_pipeline.info.to_str(), described_pipeline);
  
  Ok(())
}

pub(crate) fn remove_pipeline(
  globals: &mut DeployerGlobalConfig,
) -> anyhow::Result<()> {
  use inquire::{Select, Confirm};
  
  if globals.pipelines_registry.is_empty() {
    println!("{}", i18n::NO_PIPELINES);
    return Ok(())
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
  
  if !Confirm::new(i18n::ARE_YOU_SURE).prompt()? { return Ok(()) }
  
  globals.pipelines_registry.remove(&pipeline.info.to_str());
  
  Ok(())
}

pub(crate) fn cat_pipeline(
  globals: &DeployerGlobalConfig,
  args: &CatPipelineArgs,
) -> anyhow::Result<()> {
  let pipeline = match globals.pipelines_registry.get(&args.pipeline_short_info_and_version) {
    None => exit(1),
    Some(pipeline) => pipeline,
  };
  
  let pipeline_json = serde_json::to_string_pretty(&pipeline).unwrap();
  println!("{}", pipeline_json);
  
  Ok(())
}

pub(crate) fn cat_project_pipelines(
  config: &DeployerProjectOptions,
) -> anyhow::Result<()> {
  for pipeline in &config.pipelines {
    let pipeline_json = serde_json::to_string_pretty(&pipeline).unwrap();
    println!("{}", pipeline_json);
  }
  
  Ok(())
}

fn reorder_pipelines_in_project(
  pipelines_unordered: Vec<DescribedPipeline>,
) -> anyhow::Result<Vec<DescribedPipeline>> {
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

pub(crate) fn assign_pipeline_to_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  args: &WithPipelineArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("{}", i18n::CFG_INVALID); }
  
  let mut pipeline = if let Some(tag) = &args.tag {
    globals
      .pipelines_registry
      .get(tag)
      .ok_or_else(|| anyhow::anyhow!(i18n::NO_SUCH_PIPELINE))?
      .clone()
  } else if !globals.pipelines_registry.is_empty() {
    const NEW_PIPELINE: &str = i18n::PIPELINE_SPECIFY_ANOTHER;
    
    let mut ptags = hmap!();
    let mut tags = vec![];
    
    globals
      .pipelines_registry
      .iter()
      .map(|(k, v)| (format!("`{}` - {}", k.blue().bold(), v.title.green().bold()), v))
      .for_each(|(t, p)| { tags.push(t.clone()); ptags.insert(t, p); });
    tags.push(NEW_PIPELINE.to_string());
    
    let selected = inquire::Select::new(i18n::PIPELINE_SELECT_FOR_PROJECT, tags).prompt()?;
    
    if selected.as_str() == NEW_PIPELINE {
      DescribedPipeline::new_from_prompt(globals)?
    } else {
      let pipeline = ptags
        .get(&selected)
        .ok_or(anyhow::anyhow!(i18n::NO_SUCH_PIPELINE))?;
      (*pipeline).clone()
    }
  } else {
    DescribedPipeline::new_from_prompt(globals)?
  };
  
  for action in &mut pipeline.actions {
    *action = action.prompt_setup_for_project(&config.langs, &config.deploy_toolkit, &config.targets, &config.variables, &config.artifacts)?;
  }
  
  let short_name = if let Some(short_name) = args.r#as.as_ref() {
    short_name.to_owned()
  } else {
    inquire::Text::new(i18n::PIPELINE_SHORT_NAME_FOR_PROJECT).prompt()?
  };
  
  pipeline.desc = format!(r#"{} `{}`.{}{}"#, i18n::GOT_FROM, pipeline.title, if pipeline.desc.is_empty() { "" } else { " " }, pipeline.desc);
  pipeline.title = short_name.clone();
  
  if specify_short_name(config, &mut pipeline.title).is_err() { return Ok(()) };
  
  if let Some(old_default) = config.pipelines.iter_mut().find(|p| p.default.is_some_and(|v| v)) {
    if inquire::Confirm::new(&i18n::PIPELINE_NEW_DEFAULT_REPLACE.replace("{}", old_default.title.as_str())).prompt()? {
      old_default.default = None;
      pipeline.default = Some(true);
    }
  } else if inquire::Confirm::new(i18n::PIPELINE_NEW_DEFAULT).prompt()? {
    pipeline.default = Some(true);
  }
  
  remove_old_pipeline(config, &short_name);
  config.pipelines.push(pipeline);
  
  if config.pipelines.len() >= 2 {
    config.pipelines = reorder_pipelines_in_project(config.pipelines.clone())?;
  }
  
  println!("{}", i18n::PIPELINE_DEFAULT_SET);
  
  Ok(())
}

fn specify_short_name(
  config: &mut DeployerProjectOptions,
  short_name: &mut String,
) -> anyhow::Result<()> {
  while
    config.pipelines.iter().any(|p| p.title.as_str() == short_name) &&
    !inquire::Confirm::new(&i18n::PIPELINE_SHORT_NAME_FOR_PROJECT_OVERRIDE.replace("{}", short_name.as_str())).prompt()?
  {
    *short_name = inquire::Text::new(&format!("{} {}:", i18n::PIPELINE_SHORT_NAME_FOR_PROJECT, i18n::HIT_ESC))
      .prompt_skippable()?
      .ok_or_else(|| anyhow::anyhow!("Hitted Escape."))?;
  }
  
  Ok(())
}

fn remove_old_pipeline(
  config: &mut DeployerProjectOptions,
  short_name: &str,
) {
  if let Some(i) = config.pipelines.iter().position(|p| p.title.as_str() == short_name) {
    config.pipelines.remove(i);
  }
}

pub(crate) fn edit_pipeline(
  globals: &mut DeployerGlobalConfig,
  args: &CatPipelineArgs,
) -> anyhow::Result<()> {
  let mut pipeline = match globals.pipelines_registry.contains_key(&args.pipeline_short_info_and_version) {
    false => panic!("There is no such Pipeline!"),
    true => {
      let pipeline = globals.pipelines_registry.get(&args.pipeline_short_info_and_version).unwrap().clone();
      globals.pipelines_registry.remove(&args.pipeline_short_info_and_version);
      pipeline
    },
  };
  
  pipeline.edit_pipeline_from_prompt(globals)?;
  globals.pipelines_registry.insert(pipeline.info.to_str(), pipeline);
  
  Ok(())
}
