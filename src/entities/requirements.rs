use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::actions::check::CheckAction;
use crate::entities::environment::BuildEnvironment;
use crate::entities::traits::Execute;

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub(crate) enum Requirement {
  Exists(PathBuf),
  ExistsAny(Vec<PathBuf>),
  CheckSuccess(CheckAction),
}

impl std::fmt::Display for Requirement {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Exists(path) => write!(f, "{:?}", path),
      Self::ExistsAny(paths) => if !paths.is_empty() { write!(f, "{:?}", paths) } else { write!(f, "[]") },
      Self::CheckSuccess(check_action) => write!(f, "`{}`", check_action.command.bash_c),
    }
  }
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

pub(crate) enum SatisfyErr<'a> {
  Exists(&'a PathBuf),
  ExistsAny(&'a Vec<PathBuf>),
  Check(Vec<String>),
}

pub(crate) trait Satisfy<'a> {
  fn satisfy(&'a self, env: BuildEnvironment) -> Result<(), SatisfyErr<'a>>;
}

impl<'a> Satisfy<'a> for Requirement {
  fn satisfy(&'a self, env: BuildEnvironment) -> Result<(), SatisfyErr<'a>> {
    match self {
      Self::Exists(path) => if path.resolve_exists() { Ok(()) } else { Err(SatisfyErr::Exists(path)) },
      Self::ExistsAny(paths) => if paths.iter().any(|p| p.resolve_exists()) { Ok(()) } else { Err(SatisfyErr::ExistsAny(paths)) },
      Self::CheckSuccess(check_action) => {
        let (status, out) = check_action.execute(env).map_err(|e| SatisfyErr::Check(vec![e.to_string()]))?;
        if status { Ok(()) } else { Err(SatisfyErr::Check(out)) }
      },
    }
  }
}

impl<'a> Satisfy<'a> for HashSet<Requirement> {
  fn satisfy(&'a self, env: BuildEnvironment) -> Result<(), SatisfyErr<'a>> {
    for req in self.iter() { req.satisfy(env)?; }
    Ok(())
  }
}
