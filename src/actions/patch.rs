//! Patch Action.
//!
//! JSON example:
//!
//! ```json
//! {
//!   "patch": "path/to/patch/file.json"
//! }
//! ```
//!
//! This Action calls `PatchFile::patch` from `smart-patcher` library.

use safe_path::scoped_join;
use serde::{Deserialize, Serialize};
use smart_patcher::patch::PatchFile;
use std::path::PathBuf;

use crate::entities::{environment::BuildEnvironment, traits::Execute};
use crate::i18n;

/// Patch Action.
///
/// This Action type allows you to patch any files, including binaries and archives, based on rules.
#[derive(Deserialize, Serialize, PartialEq, Default, Clone)]
pub struct PatchAction {
  /// Path to patch file.
  pub patch: PathBuf,
}

impl Execute for PatchAction {
  /// Performs the patch in the build folder.
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let patch_file = scoped_join(env.build_dir, &self.patch)?;

    let file = std::fs::File::open(patch_file)?;
    let buf = std::io::BufReader::new(file);
    let patches: PatchFile = serde_json::from_reader(buf)?;

    match patches.patch(env.build_dir, env.build_dir) {
      Err(e) => Ok((false, vec![format!("{}: {}", i18n::PATCH_ERROR, e)])),
      Ok(0) => Ok((false, vec![format!("{}", i18n::PATCH_DONE_ZERO_TIMES)])),
      Ok(num) => Ok((true, vec![format!(
        "{}",
        i18n::PATCH_DONE.replace("{}", format!("{}", num).as_str())
      )])),
    }
  }
}
