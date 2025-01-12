use serde::{Deserialize, Serialize};

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
  
  pub(crate) fn get_value(&self) -> anyhow::Result<&str> {
    match &self.value {
      VarValue::Plain(val) => Ok(val.as_str()),
      VarValue::FromEnvFile(_) => unimplemented!(),
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
  pub(crate) env_file_path: String,
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
