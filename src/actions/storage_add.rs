//! AddToStorage Action.
//!
//! Based on automatic rule to extract version from project, allows you to
//! load your artifacts as content to Deployer's storage.

use serde::{Deserialize, Serialize};
use std::io::Read;

use crate::entities::auto_version::AutoVersionExtractFromRule;
use crate::entities::environment::RunEnvironment;
use crate::entities::info::ContentInfo;
use crate::entities::traits::Execute;
use crate::storage::add_to_storage;

/// AddToStorage Action.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct AddToStorageAction {
  /// Content's short name. Used to load to and sync from storage.
  pub short_name: String,
  /// Automatic rule to extract version from project.
  pub auto_version_rule: AutoVersionExtractFromRule,
}

impl Execute for AddToStorageAction {
  /// Adds content to storage from given run environment.
  fn execute(&self, env: RunEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    Ok(match &self.auto_version_rule {
      AutoVersionExtractFromRule::CmdStdout(cmd) => {
        let (succ, out) = cmd.execute(env)?;
        if !succ || out.is_empty() {
          (false, out)
        } else {
          let version = out.last().unwrap().trim().replace(">>> ", "");
          let info = ContentInfo::new(self.short_name.as_str(), version.as_str())?;
          if let Err(e) = add_to_storage(env.storage_dir, env.artifacts_dir, &info) {
            (false, vec![e.to_string()])
          } else {
            (true, vec![])
          }
        }
      }
      AutoVersionExtractFromRule::PlainFile(path) => {
        let mut file = std::fs::File::open(path)?;
        let version = {
          let mut ver = String::new();
          file.read_to_string(&mut ver)?;
          ver.trim().to_string()
        };
        let info = ContentInfo::new(self.short_name.as_str(), version.as_str())?;
        if let Err(e) = add_to_storage(env.storage_dir, env.artifacts_dir, &info) {
          (false, vec![e.to_string()])
        } else {
          (true, vec![])
        }
      }
    })
  }
}
