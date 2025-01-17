//! Observe Action.
//! 
//! JSON example:
//! {
//!   "command": {
//!     "bash_c": "btop",
//!     "ignore_fails": true,
//!     "show_success_output": true,
//!     "show_bash_c": false,
//!     "only_when_fresh": false
//!   }
//! }
//! 
//! Observe Action is used when you need to disable I/O redirection and
//! interact with running programs.

use serde::{Deserialize, Serialize};

use crate::entities::environment::BuildEnvironment;
use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;

/// Observe Action.
/// 
/// This Action type implicitly overrides `no_pipe` build argument for given command.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) struct ObserveAction {
  pub(crate) command: CustomCommand,
}

impl Execute for ObserveAction {
  /// Executes commands with given build environment without I/O redirection.
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut observe_env = env;
    observe_env.no_pipe = true;
    self.command.execute(observe_env)
  }
}
