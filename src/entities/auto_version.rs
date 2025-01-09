use anyhow::bail;
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
  
  pub(crate) fn new_from_prompt() -> anyhow::Result<Self> {
    let new_autover_rule = inquire::Select::new(
      i18n::SPECIFY_AUTO_VER,
      vec![i18n::AUTO_VER_CMD_STDOUT, i18n::AUTO_VER_PLAIN_FILE],
    ).prompt()?;
    let auto_version_rule = match new_autover_rule {
      i18n::AUTO_VER_CMD_STDOUT => AutoVersionExtractFromRule::CmdStdout({
        let mut cmd = CustomCommand::new_from_prompt_unspecified()?;
        cmd.show_success_output = true;
        cmd
      }),
      i18n::AUTO_VER_PLAIN_FILE => AutoVersionExtractFromRule::PlainFile({
        let path = inquire::Text::new(i18n::SPECIFY_AUTO_VER_RELATIVE_FILEPATH).prompt()?;
        PathBuf::from(path)
      }),
      _ => bail!("There is no such type"),
    };
    Ok(auto_version_rule)
  }
}
