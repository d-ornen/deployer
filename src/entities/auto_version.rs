use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::entities::custom_command::CustomCommand;
use crate::i18n;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all="snake_case")]
pub(crate) enum AutoVersionExtractFromRule {
  CmdStdout(CustomCommand),
  PlainFile(PathBuf),
}

impl AutoVersionExtractFromRule {
  pub(crate) fn type_str(&self) -> &str {
    match self {
      Self::CmdStdout(_) => i18n::AUTO_VER_CMD_STDOUT,
      Self::PlainFile(_) => i18n::AUTO_VER_PLAIN_FILE,
    }
  }
}
