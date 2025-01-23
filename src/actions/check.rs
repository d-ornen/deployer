//! Check Action.
//!
//! JSON example:
//! ```json
//! {
//!   "Check": {
//!     "command": {
//!       "bash_c": "<af>",
//!       "placeholders": [
//!         "<af>"
//!       ],
//!       "ignore_fails": true,
//!       "show_success_output": false,
//!       "show_bash_c": false,
//!       "only_when_fresh": false
//!     },
//!     "success_when_found": "some rust regex",
//!     "success_when_not_found": null
//!   }
//! }
//! ```
//!
//! Allows you to automatically check output of your command by given regular expressions `success_when_found` and `success_when_not_found`.
//!
//! If both regular expressions specified, the Action will be considered successful if the first matches and the second does not match the command output.

use colored::Colorize;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::entities::{custom_command::CustomCommand, environment::BuildEnvironment, traits::Execute};
use crate::i18n;
use crate::utils::{regexopt2str, str2regexopt};

/// Check Action.
///
/// Checks your command output by given regular expressions.
#[derive(Deserialize, Serialize, Clone)]
pub struct CheckAction {
  /// Command to execute.
  pub command: CustomCommand,
  /// Regular expression that means successful check if it matches the command's output.
  #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt")]
  pub success_when_found: Option<Regex>,
  /// Regular expression that means successful check if it doesn't match the command's output.
  #[serde(serialize_with = "regexopt2str", deserialize_with = "str2regexopt")]
  pub success_when_not_found: Option<Regex>,
}

impl Execute for CheckAction {
  /// Executes commands with given build environment and checks its output.
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let mut output = vec![];

    let (status, command_out) = self.command.execute(env)?;
    if !status && !self.command.ignore_fails {
      return Ok((false, command_out));
    }

    if let Some(re) = &self.success_when_found {
      let text = command_out.join("\n");
      if re.is_match(text.as_str()) {
        output.push(format!("{} `{}` {}!", i18n::PATTERN, re.as_str().green(), i18n::FOUND));
      } else {
        output.push(format!(
          "{} `{}` {}!",
          i18n::PATTERN,
          re.as_str().green(),
          i18n::NOT_FOUND
        ));
        return Ok((false, output));
      }
    }

    if let Some(re) = &self.success_when_not_found {
      let text = command_out.join("\n");
      if !re.is_match(text.as_str()) {
        output.push(format!(
          "{} `{}` {}!",
          i18n::PATTERN,
          re.as_str().green(),
          i18n::NOT_FOUND
        ));
      } else {
        output.push(format!("{} `{}` {}!", i18n::PATTERN, re.as_str().green(), i18n::FOUND));
        return Ok((false, output));
      }
    }

    Ok((true, output))
  }
}

impl Eq for CheckAction {}

impl std::hash::Hash for CheckAction {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.command.hash(state);
    if let Some(succ_found) = &self.success_when_found {
      succ_found.as_str().hash(state);
    }
    if let Some(succ_not_found) = &self.success_when_not_found {
      succ_not_found.as_str().hash(state);
    }
  }
}

impl PartialEq for CheckAction {
  fn eq(&self, other: &Self) -> bool {
    self.command.eq(&other.command)
      && ((self.success_when_found.is_none() && other.success_when_found.is_none())
        || (self.success_when_found.as_ref().is_some_and(|a| {
          other
            .success_when_found
            .as_ref()
            .is_some_and(|b| a.as_str().eq(b.as_str()))
        })))
      && ((self.success_when_not_found.is_none() && other.success_when_not_found.is_none())
        || (self.success_when_not_found.as_ref().is_some_and(|a| {
          other
            .success_when_not_found
            .as_ref()
            .is_some_and(|b| a.as_str().eq(b.as_str()))
        })))
  }
}
