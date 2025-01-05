use safe_path::scoped_join;
use serde::{Serialize, Deserialize};
use smart_patcher::patch::PatchFile;
use std::path::PathBuf;

use crate::entities::{
  environment::BuildEnvironment,
  traits::{Execute, Edit},
};
use crate::i18n;

#[derive(Deserialize, Serialize, PartialEq, Default, Clone, Debug)]
pub(crate) struct PatchAction {
  pub(crate) patch: PathBuf,
}

impl PatchAction {
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let patch = PathBuf::from(inquire::Text::new(i18n::PATCH_SPECIFY_PATH).prompt()?);
    Ok(Self { patch })
  }
}

impl Execute for PatchAction {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let patch_file = scoped_join(env.build_dir, &self.patch)?;
    
    let file = std::fs::File::open(patch_file)?;
    let buf = std::io::BufReader::new(file);
    let patches: PatchFile = serde_json::from_reader(buf)?;
    
    match patches.patch(env.build_dir, env.build_dir) {
      Err(e) => Ok((false, vec![format!("{}: {}", i18n::PATCH_ERROR, e)])),
      Ok(0) => Ok((false, vec![format!("{}", i18n::PATCH_DONE_ZERO_TIMES)])),
      Ok(num) => Ok((true, vec![format!("{}", i18n::PATCH_DONE.replace("{}", format!("{}", num).as_str()))])),
    }
  }
}

impl Edit for PatchAction {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    self.patch = PathBuf::from(inquire::Text::new(i18n::PATCH_SPECIFY_PATH).with_default(&self.patch.to_string_lossy()).prompt()?);
    Ok(())
  }
}
