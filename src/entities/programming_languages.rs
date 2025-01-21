//! Programming languages module.

use serde::{Deserialize, Serialize};

/// Programming language.
///
/// Canonically represents most well-known by `deployer` author languages,
/// but you always specify yours.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum ProgrammingLanguage {
  Rust,
  Go,
  C,
  Cpp,
  Python,
  Other(String),
}

impl std::fmt::Display for ProgrammingLanguage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let lang = match self {
      Self::Rust => "Rust".to_string(),
      Self::Go => "Go".to_string(),
      Self::C => "C".to_string(),
      Self::Cpp => "C++".to_string(),
      Self::Python => "Python".to_string(),
      Self::Other(s) => s.to_owned(),
    };

    f.write_str(&lang)
  }
}
