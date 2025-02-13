//! Pack-like Action.
//!
//! JSON example:
//!
//! ```json
//! {
//!   "type": "pack",
//!   "target": {
//!     "arch": "x86_64",
//!     "os": "Linux",
//!     "derivative": "any",
//!     "version": "No"
//!   },
//!   "commands": [
//!     {
//!       "bash_c": "upx <af>",
//!       "placeholders": [
//!         "<af>"
//!       ],
//!       "ignore_fails": false,
//!       "show_success_output": false,
//!       "show_bash_c": false,
//!       "only_when_fresh": false
//!     }
//!   ]
//! }
//! ```
//!
//! For pack-like Actions, specialization in targets is specific:
//! depending on whether target aimed by the project matches the target
//! aimed by the pack-like Action, Deployer will warn you about using Actions
//! that are incompatible with the project.

use serde::{Deserialize, Serialize};

use crate::entities::{
  custom_command::CustomCommand, environment::RunEnvironment, targets::TargetDescription, traits::Execute,
};

/// Pack-like Action.
///
/// This Action type depends on aimed installation target.
#[derive(Deserialize, Serialize, PartialEq, Default, Clone)]
pub struct PackAction {
  /// Target (combination of CPU arch, OS and its version or derivative).
  pub target: Option<TargetDescription>,
  /// Commands to pack-like Action.
  pub commands: Vec<CustomCommand>,
}

pub type DeliveryAction = PackAction;
pub type InstallAction = PackAction;

impl Execute for PackAction {
  /// Executes commands with given run environment.
  fn execute(&self, env: &RunEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];

    for cmd in &self.commands {
      let (status, out) = cmd.execute(env)?;
      total_output.extend_from_slice(&out);

      if !status {
        return Ok((false, total_output));
      }
    }

    Ok((true, total_output))
  }
}
