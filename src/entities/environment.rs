use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::entities::info::ShortName;
use crate::entities::remote_host::RemoteHost;

#[derive(Clone, Copy)]
pub(crate) struct BuildEnvironment<'a> {
  pub(crate) build_dir: &'a Path,
  pub(crate) cache_dir: &'a Path,
  pub(crate) config_dir: &'a Path,
  pub(crate) storage_dir: &'a Path,
  pub(crate) artifacts_dir: &'a Path,
  pub(crate) remotes: &'a HashMap<ShortName, RemoteHost>,
  pub(crate) ignore: &'a HashSet<PathBuf>,
  pub(crate) new_build: bool,
  pub(crate) silent_build: bool,
  pub(crate) no_pipe: bool,
}
