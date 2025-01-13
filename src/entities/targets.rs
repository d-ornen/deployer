use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) struct TargetDescription {
  pub(crate) arch: String,
  pub(crate) os: OsVariant,
  pub(crate) derivative: String,
  pub(crate) version: OsVersionSpecification,
}

impl std::fmt::Display for TargetDescription {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let os = match &self.os {
      OsVariant::Android => "android",
      OsVariant::iOS => "ios",
      OsVariant::Linux => "linux",
      OsVariant::UnixLike(nix) => &format!("unix-{}", nix),
      OsVariant::Windows => "windows",
      OsVariant::macOS => "macos",
      OsVariant::Other(other) => other,
    };
    
    let os_ver = match &self.version {
      OsVersionSpecification::No => "any",
      OsVersionSpecification::Weak(ver) => &format!("^{}", ver),
      OsVersionSpecification::Strong(ver) => ver,
    };
    
    f.write_str(&format!("{}/{}@{}@{}", self.arch, os, self.derivative, os_ver))
  }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub(crate) enum OsVariant {
  Android,
  #[allow(non_camel_case_types)]
  iOS,
  Linux,
  UnixLike(String),
  Windows,
  #[allow(non_camel_case_types)]
  macOS,
  Other(String),
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub(crate) enum OsVersionSpecification {
  #[default]
  No,
  /// Даже если указана версия, при несоответствии версий может заработать.
  Weak(String),
  Strong(String),
}
