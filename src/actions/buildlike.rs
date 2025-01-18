//! Build-like Actions.
//! 
//! JSON example:
//! 
//! ```json
//! {
//!   "PostBuild": {
//!     "supported_langs": [
//!       "Rust",
//!       "Go",
//!       "C",
//!       "Cpp",
//!       "Python",
//!       {
//!         "Other": "any"
//!       }
//!     ],
//!     "commands": [
//!       {
//!         "bash_c": "upx <artifact>",
//!         "placeholders": [
//!           "<artifact>"
//!         ],
//!         "ignore_fails": false,
//!         "show_success_output": false,
//!         "show_bash_c": false,
//!         "only_when_fresh": false
//!       }
//!     ]
//!   }
//! }
//! ```
//! 
//! For build-like Actions, specialization in programming languages is specific:
//! depending on whether the set of languages used in the project matches the set
//! specified in the build-like Action, Deployer will warn you about using Actions
//! that are incompatible with the project.

use serde::{Deserialize, Serialize};

use crate::entities::{
  environment::BuildEnvironment,
  custom_command::CustomCommand,
  programming_languages::ProgrammingLanguage,
  traits::Execute,
};

/// Build-like Action.
/// 
/// This Action type depends on supported project programming languages.
#[derive(Deserialize, Serialize, PartialEq, Default, Clone)]
pub struct BuildAction {
  /// Programming languages supported by this Action.
  pub supported_langs: Vec<ProgrammingLanguage>,
  /// Commands to build-like Action.
  pub commands: Vec<CustomCommand>,
}

pub type PreBuildAction = BuildAction;
pub type PostBuildAction = BuildAction;
pub type TestAction = BuildAction;

impl Execute for BuildAction {
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
