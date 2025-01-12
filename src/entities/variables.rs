use anyhow::anyhow;
use env_file_reader::read_file;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) struct Variable {
  pub(crate) title: String,
  pub(crate) is_secret: bool,
  pub(crate) value: VarValue,
}

impl Variable {
  pub(crate) fn new_plain(title: &str, value: &str) -> Self {
    Self {
      title: title.to_string(),
      is_secret: false,
      value: VarValue::Plain(value.to_string()),
    }
  }
  
  pub(crate) fn get_value(&self) -> anyhow::Result<String> {
    match &self.value {
      VarValue::Plain(val) => Ok(val.to_owned()),
      VarValue::FromEnvFile(info) => {
        let env_variables = read_file(&info.env_file_path)?;
        let val = env_variables.get(&info.key).ok_or(anyhow!("There is no such key in your ENV file."))?;
        Ok(val.to_owned())
      },
      VarValue::FromHCVaultKv2(_) => unimplemented!(),
    }
  }
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) enum VarValue {
  Plain(String),
  FromEnvFile(FromEnvFile),
  FromHCVaultKv2(Kv2Paths),
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) struct FromEnvFile {
  pub(crate) env_file_path: PathBuf,
  pub(crate) key: String,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub(crate) struct Kv2Paths {
  pub(crate) mount_path: String,
  pub(crate) secret_path: String,
}

pub(crate) trait VarTraits {
  fn is_secret(&self, title: &str) -> bool;
  fn titles(&self) -> Vec<String>;
  fn find(&self, title: &str) -> Option<Variable>;
}

impl VarTraits for [Variable] {
  fn is_secret(&self, title: &str) -> bool {
    self.iter().find(|v| v.title.as_str().eq(title)).is_some_and(|v| v.is_secret)
  }
  
  fn titles(&self) -> Vec<String> {
    self.iter().map(|v| v.title.to_owned()).collect::<Vec<_>>()
  }
  
  fn find(&self, title: &str) -> Option<Variable> {
    self.iter().find(|v| v.title.as_str().eq(title)).cloned()
  }
}
