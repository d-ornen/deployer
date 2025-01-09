use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::entities::{
  auto_version::AutoVersionExtractFromRule,
  custom_command::CustomCommand,
};
use crate::i18n;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) struct AddToStorageAction {
  pub(crate) short_name: String,
  pub(crate) auto_version_rule: AutoVersionExtractFromRule,
}

impl AddToStorageAction {
  pub(crate) fn edit_from_prompt(&mut self) -> anyhow::Result<()> {
    loop {
      let short_name = inquire::Text::new(i18n::SPECIFY_SHORT_NAME_FOR_ADD_TO_STORAGE).with_default(&self.short_name).prompt()?;
      if crate::entities::info::validate_short_name(&short_name) { self.short_name = short_name; break }
      println!("{}", i18n::INCORRECT_SHORT_NAME);
    }
    
    if inquire::Confirm::new(&format!(
      "{} {}",
      i18n::IF_NEEDED_TO_CHANGE_AUTOVER,
      self.auto_version_rule.type_str(),
    )).with_default(false).prompt()? {
      let new_autover_rule = inquire::Select::new(
        i18n::SPECIFY_AUTO_VER,
        vec![i18n::AUTO_VER_CMD_STDOUT, i18n::AUTO_VER_PLAIN_FILE],
      ).prompt()?;
      self.auto_version_rule = match new_autover_rule {
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
    }
    
    Ok(())
  }
}
