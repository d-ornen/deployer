//! Automatic rule to extract version module.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::entities::custom_command::CustomCommand;
use crate::i18n;

/// Rule to extract version from the project automatically.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all="snake_case")]
pub(crate) enum AutoVersionExtractFromRule {
  /// Extract from `stdout` of command.
  CmdStdout(CustomCommand),
  /// Extract from plain file.
  PlainFile(PathBuf),
}

impl AutoVersionExtractFromRule {
  /// Returns rule type as `&str`.
  pub(crate) fn type_str(&self) -> &str {
    match self {
      Self::CmdStdout(_) => i18n::AUTO_VER_CMD_STDOUT,
      Self::PlainFile(_) => i18n::AUTO_VER_PLAIN_FILE,
    }
  }
}
