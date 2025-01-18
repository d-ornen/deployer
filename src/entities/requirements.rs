//! Requirements module.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::actions::check::CheckAction;
use crate::entities::environment::BuildEnvironment;
use crate::entities::info::ShortName;
use crate::entities::traits::Execute;
use crate::i18n;

/// Requirement.
/// 
/// Deployer tries to satisfy Pipeline's Actions requirements before every Pipeline execution.
/// If a single requirement fails to satisfy, Deployer exits.
#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub enum Requirement {
  /// Requirement of path existence.
  Exists(PathBuf),
  /// Requirement of at least single path of a given list existence.
  ExistsAny(Vec<PathBuf>),
  /// Requirement that executes some Check Action to be satisfied.
  CheckSuccess(CheckAction),
  /// Requirement that checks the given remote host. See `RemoteHost::check`.
  RemoteAccessibleAndReady(ShortName),
}

trait ResolveExists {
  fn resolve_exists(&self) -> bool;
}

impl ResolveExists for PathBuf {
  fn resolve_exists(&self) -> bool {
    use dirs::home_dir;
    
    let lossy = self.to_string_lossy();
    if lossy.contains("~") {
      PathBuf::from(lossy.replace("~", &home_dir().unwrap().to_string_lossy())).exists()
    } else {
      self.exists()
    }
  }
}

/// Satisfy errors type.
pub enum SatisfyErr<'a> {
  Exists(&'a PathBuf),
  ExistsAny(&'a Vec<PathBuf>),
  Check(Vec<String>),
  Remote(String),
}

pub trait Satisfy<'a> {
  fn satisfy(&'a self, env: BuildEnvironment) -> Result<(), SatisfyErr<'a>>;
}

impl<'a> Satisfy<'a> for Requirement {
  /// Tries to satisfy the given requirement.
  fn satisfy(&'a self, env: BuildEnvironment) -> Result<(), SatisfyErr<'a>> {
    match self {
      Self::Exists(path) => if path.resolve_exists() { Ok(()) } else { Err(SatisfyErr::Exists(path)) },
      Self::ExistsAny(paths) => if paths.iter().any(|p| p.resolve_exists()) { Ok(()) } else { Err(SatisfyErr::ExistsAny(paths)) },
      Self::CheckSuccess(check_action) => {
        let (status, out) = check_action.execute(env).map_err(|e| SatisfyErr::Check(vec![e.to_string()]))?;
        if status { Ok(()) } else { Err(SatisfyErr::Check(out)) }
      },
      Self::RemoteAccessibleAndReady(remote) => {
        let globals = crate::rw::read::<crate::configs::DeployerGlobalConfig>(&env.config_dir, crate::GLOBAL_CONF);
        if let Some(remote) = globals.remote_hosts.get(remote) { remote.check().map_err(|e| SatisfyErr::Remote(e.to_string())) }
        else { Err(SatisfyErr::Remote(i18n::NO_SUCH_REMOTE.to_string())) }
      },
    }
  }
}

impl<'a> Satisfy<'a> for HashSet<Requirement> {
  /// Tries to satisfy all unique requirements (because of `HashSet`).
  fn satisfy(&'a self, env: BuildEnvironment) -> Result<(), SatisfyErr<'a>> {
    for req in self.iter() { req.satisfy(env)?; }
    Ok(())
  }
}

impl std::fmt::Display for Requirement {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Exists(path) => write!(f, "{:?}", path),
      Self::ExistsAny(paths) => if !paths.is_empty() { write!(f, "{:?}", paths) } else { write!(f, "[]") },
      Self::CheckSuccess(check_action) => write!(f, "`{}`", check_action.command.bash_c),
      Self::RemoteAccessibleAndReady(host_info) => write!(f, "`{}`", host_info.as_str()),
    }
  }
}
