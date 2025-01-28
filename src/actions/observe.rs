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

use crate::entities::custom_command::CustomCommand;
use crate::entities::environment::RunEnvironment;
use crate::entities::traits::Execute;

/// Observe Action.
///
/// This Action type implicitly overrides `no_pipe` run argument for given command.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct ObserveAction {
  pub command: CustomCommand,
}

impl Execute for ObserveAction {
  /// Executes commands with given run environment without I/O redirection.
  fn execute(&self, env: RunEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut observe_env = env;
    observe_env.no_pipe = true;
    self.command.execute(observe_env)
  }

  fn execute_observer(&self, env: RunEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut observe_env = env;
    observe_env.no_pipe = true;
    self.command.execute_observer(observe_env)
  }
}
