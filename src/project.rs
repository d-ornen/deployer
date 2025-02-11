//! Project module.

use std::path::{Path, PathBuf};

use crate::cmd::{CleanArgs, InitArgs};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::run::Runs;
use crate::{ARTIFACTS_DIR, CACHE_DIR, hmap, i18n};

/// Inits the project.
pub fn init_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  args: &InitArgs,
  conf_file: &mut String,
) -> anyhow::Result<()> {
  let curr_dir = std::env::current_dir()
    .expect("Can't get current dir!")
    .to_str()
    .expect("Can't convert current dir's path to string!")
    .to_owned();
  if !globals.projects.contains(&curr_dir) {
    globals.projects.push(curr_dir.to_owned());
  }

  config.init_from_prompt(curr_dir)?;

  if let Some(preferred) = &globals.preferred_conf_format {
    match preferred.as_str() {
      "json" => *conf_file = "deploy-config.json".to_owned(),
      "yaml" => *conf_file = "deploy-config.yaml".to_owned(),
      "toml" => *conf_file = "deploy-config.toml".to_owned(),
      _ => {}
    }
  }

  if let Some(preferred) = &args.file_format {
    match preferred.as_str() {
      "json" => *conf_file = "deploy-config.json".to_owned(),
      "yaml" => *conf_file = "deploy-config.yaml".to_owned(),
      "toml" => *conf_file = "deploy-config.toml".to_owned(),
      _ => {}
    }
  }

  println!("{}", i18n::INIT_SUCC);

  Ok(())
}

/// Edits the project.
pub fn edit_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  conf_file: &mut String,
) -> anyhow::Result<()> {
  if *config == Default::default() {
    panic!("{}", i18n::CFG_INVALID);
  }

  config.edit_from_prompt(globals, conf_file)?;
  Ok(())
}

/// Cleans all project runs.
///
/// You can also specify `include_artifacts` option (`deployer clean -i`)
/// to cleanup `artifacts` folder.
pub fn clean_runs(
  config: &DeployerProjectOptions,
  runs: &mut Runs,
  cache_dir: &Path,
  args: &CleanArgs,
) -> anyhow::Result<()> {
  use fs_extra::dir::get_size;

  let mut path = PathBuf::new();
  path.push(cache_dir);
  path.push(CACHE_DIR);

  let mut total: u64 = 0;

  if let Some(project_builds) = runs
    .projects
    .iter_mut()
    .find(|p| p.name.as_str().eq(config.project_name.as_str()))
  {
    if !args.preserve_least {
      for folder in project_builds.runs.iter().map(|b| b.folder.clone()) {
        total += get_size(&folder)?;
        let _ = std::fs::remove_dir_all(folder);
      }
      project_builds.runs.clear();
    } else {
      let mut hm = hmap!();
      for folder in project_builds.runs.iter().cloned() {
        hm.insert(folder.exclusive_tag.clone().unwrap_or(String::new()), folder);
      }
      for folder in project_builds
        .runs
        .iter()
        .filter(|b| !hm.contains_key(&b.exclusive_tag.clone().unwrap_or_default()))
        .map(|b| b.folder.clone())
      {
        total += get_size(&folder)?;
        let _ = std::fs::remove_dir_all(folder);
      }
      project_builds.runs.clear();
      for folder in hm.values() {
        project_builds.runs.push(folder.clone());
      }
    }
  }

  if args.include_artifacts {
    let curr_dir = std::env::current_dir()?;
    let artifacts_dir = curr_dir.join(ARTIFACTS_DIR);
    if artifacts_dir.as_path().exists() {
      total += get_size(&artifacts_dir)?;
      let _ = std::fs::remove_dir_all(artifacts_dir);
    }
  }

  println!("{}: {}", i18n::CLEANED, format_size(total));

  Ok(())
}

/// Formats `u64` as file size (bytes).
fn format_size(size: u64) -> String {
  const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
  let mut size = size as f64;
  let mut unit_index = 0;

  while size >= 1024.0 && unit_index < UNITS.len() - 1 {
    size /= 1024.0;
    unit_index += 1;
  }

  if unit_index == 0 {
    format!("{} {}", size as u64, UNITS[unit_index])
  } else {
    format!("{:.1} {}", size, UNITS[unit_index])
  }
}
