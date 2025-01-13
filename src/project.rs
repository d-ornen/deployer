use crate::cmd::InitArgs;
use crate::configs::{DeployerProjectOptions, DeployerGlobalConfig};
use crate::i18n;

pub(crate) fn init_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  _args: &InitArgs,
) -> anyhow::Result<()> {
  let curr_dir = std::env::current_dir()
    .expect("Can't get current dir!")
    .to_str()
    .expect("Can't convert current dir's path to string!")
    .to_owned();
  if !globals.projects.contains(&curr_dir) { globals.projects.push(curr_dir.to_owned()); }
  
  config.init_from_prompt(curr_dir)?;
  
  println!("{}", i18n::INIT_SUCC);
  
  Ok(())
}

pub(crate) fn edit_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("{}", i18n::CFG_INVALID); }
  
  config.edit_project_from_prompt(globals)?;
  Ok(())
}
