//! Deploy-like Actions.
//! 
//! JSON example:
//! 
//! ```json
//! {
//!   "title": "Build Docker Compose Image",
//!   "desc": "Build Docker image with Docker Compose",
//!   "info": "docker-compose-build@0.1.0",
//!   "tags": [
//!     "docker",
//!     "compose"
//!   ],
//!   "action": {
//!     "ConfigureDeploy": {
//!       "deploy_toolkit": "docker-compose",
//!       "tags": [
//!         "docker",
//!         "compose"
//!       ],
//!       "commands": [
//!         {
//!           "bash_c": "docker compose build",
//!           "ignore_fails": false,
//!           "show_success_output": false,
//!           "show_bash_c": true,
//!           "only_when_fresh": false
//!         }
//!       ]
//!     }
//!   }
//! }
//! ```
//! 
//! For deploy-like Actions, specialization in deploy toolkit is specific:
//! depending on whether the deploy toolkit (e.g., `docker`, `podman`, `k8s`, etc.)
//! used in the project matches the toolkit specified in the deploy-like Action,
//! Deployer will warn you about using Actions that are incompatible with the project.

use serde::{Deserialize, Serialize};

use crate::entities::environment::BuildEnvironment;
use crate::entities::custom_command::CustomCommand;
use crate::entities::traits::Execute;

/// Deploy-like Action.
/// 
/// This Action type depends on supported project deploy toolkit.
#[derive(Deserialize, Serialize, PartialEq, Default, Clone)]
pub struct DeployAction {
  /// Deploy toolkit supported by Action.
  pub deploy_toolkit: Option<String>,
  /// Commands to deploy-like Action.
  pub commands: Vec<CustomCommand>,
}

pub type ConfigureDeployAction = DeployAction;
pub type PostDeployAction = DeployAction;

impl Execute for DeployAction {
  /// Executes commands with given build environment.
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut total_output = vec![];
    
    for cmd in &self.commands {
      let (status, out) = cmd.execute(env)?;
      total_output.extend_from_slice(&out);
      
      if !status {
        return Ok((false, total_output))
      }
    }
    
    Ok((true, total_output))
  }
}
