//! Project module.

use crate::cmd::InitArgs;
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::i18n;

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
