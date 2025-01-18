//! Target module.
//! 
//! Simply, target is a goal of installation - combination of CPU arch and operation system specs.

use serde::{Deserialize, Serialize};

/// Target description.
/// 
/// Contains description of installation goal.
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct TargetDescription {
  /// CPU architecture specified in plain text.
  /// 
  /// You can write just an arch `x86_64`, simple form `pc`/`arm`,
  /// extended form `x86_64-sse-4`, until you really use these architectures
  /// specialization in your Actions (for example, you can specify `sse-4` existence
  /// for running `RUSTFLAGS='-C target_cpu=x86-64-v4' cargo build --release`).
  pub arch: String,
  /// Operation system or specific kernel (maybe, you writes a BIOS application).
  pub os: OsVariant,
  /// OS derivative.
  /// 
  /// Usually it means some specific edition or distributive
  /// (you may leave it empty or just write `any`, if there is no).
  /// For example: `Ubuntu` (Linux distribution), `AOSP`/`MIUI` (Android distributions).
  pub derivative: String,
  /// OS version specification.
  pub version: OsVersionSpecification,
}

/// OS variant.
/// 
/// Variants presented here are not complete list of operation systems.
/// You can specify your OS variant as `Other("OS Name")`.
/// 
/// Unix-like is related to `BSD` and other POSIX-compatible systems.
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub enum OsVariant {
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

/// OS version specification.
/// 
/// You can choose weak specification, if you targetting at some OS version,
/// but your software can be run with some other (newer or older) versions.
/// 
/// Strong specification, on the contrary, locks on chosen OS version.
#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
pub enum OsVersionSpecification {
  #[default]
  No,
  Weak(String),
  Strong(String),
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
