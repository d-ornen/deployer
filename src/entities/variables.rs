//! Variables module.
//!
//! Variables allows you to modify your shell commands with no need to
//! edit project configuration file.

use anyhow::anyhow;
use env_file_reader::read_file;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

pub const VAULT_ADDR_ENV: &str = "DEPLOYER_VAULT_ADDR";
pub const VAULT_ADDR_TOKEN: &str = "DEPLOYER_VAULT_TOKEN";

/// Variable.
///
/// Contains some metadata about variable value.
#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub struct Variable {
  /// Name of the variable.
  pub title: String,
  /// Is the variable a secret.
  pub is_secret: bool,
  /// Variable value (will replace one of shell command's placeholders).
  pub value: VarValue,
}

impl Variable {
  /// Creates new plain variable from given title and value.
  ///
  /// Note that plain variable can't be a secret in any safe way. It will be
  /// stored inside your project configuration.
  pub fn new_plain(title: &str, value: &str) -> Self {
    Self {
      title: title.to_string(),
      is_secret: false,
      value: VarValue::Plain {
        value: value.to_string(),
      },
    }
  }

  /// Gets the variable's value.
  pub fn get_value(&self) -> anyhow::Result<String> {
    match &self.value {
      VarValue::Plain { value } => Ok(value.to_owned()),
      VarValue::FromEnvFile(info) => {
        let env_variables = read_file(&info.env_file_path)?;
        let val = env_variables
          .get(&info.key)
          .ok_or(anyhow!("There is no such key in your ENV file."))?;
        Ok(val.to_owned())
      }
      VarValue::FromEnvVar { var_name } => Ok(std::env::var(var_name)?),
      VarValue::FromHcVaultKv2(info) => {
        let vault_addr = std::env::var(VAULT_ADDR_ENV)?;
        let vault_token = std::env::var(VAULT_ADDR_TOKEN)?;

        let client = VaultClient::new(
          VaultClientSettingsBuilder::default()
            .address(vault_addr)
            .token(vault_token)
            .build()?,
        )?;

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(kv2::read(&client, &info.mount_path, &info.secret_path))
          .map_err(|e| anyhow!(e.to_string()))
      }
    }
  }
}

/// Variable value type.
#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum VarValue {
  /// Plain (stored inside this struct).
  Plain { value: String },
  /// Environment file's variable (specified by env file path and var's name).
  FromEnvFile(FromEnvFile),
  /// Environment variable (specified by env var's name).
  FromEnvVar { var_name: String },
  /// Vault KV2 secret (specified by `mount_path` and `secret_path`).
  FromHcVaultKv2(Kv2Paths),
}

/// Environment file's variable metadata.
#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub struct FromEnvFile {
  /// Path to the environment variable's file.
  pub env_file_path: PathBuf,
  /// Name of the variable by which it's stored inside given file.
  pub key: String,
}

/// Vault KV2 secret's metadata.
///
/// See [KV2 docs](https://developer.hashicorp.com/vault/api-docs/secret/kv/kv-v2).
#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub struct Kv2Paths {
  /// The path to the KV mount containing the secret to read.
  pub mount_path: String,
  /// Specifies the path of the secret to read.
  pub secret_path: String,
}

/// Some additional methods/
pub trait VarTraits {
  fn is_secret(&self, title: &str) -> bool;
  fn titles(&self) -> Vec<String>;
  fn find(&self, title: &str) -> Option<Variable>;
}

impl VarTraits for [Variable] {
  /// Is the list containing a secret?
  fn is_secret(&self, title: &str) -> bool {
    self
      .iter()
      .find(|v| v.title.as_str().eq(title))
      .is_some_and(|v| v.is_secret)
  }

  /// Gets variable titles.
  fn titles(&self) -> Vec<String> {
    self.iter().map(|v| v.title.to_owned()).collect::<Vec<_>>()
  }

  /// Searches the variable by given title.
  fn find(&self, title: &str) -> Option<Variable> {
    self.iter().find(|v| v.title.as_str().eq(title)).cloned()
  }
}
