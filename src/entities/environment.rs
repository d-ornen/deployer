//! Build environment module.
//! 
//! `BuildEnvironment` is just a struct that have all needed variables
//! and paths during Pipeline execution.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::entities::info::ShortName;
use crate::entities::remote_host::RemoteHost;

/// Build environment.
#[derive(Clone, Copy)]
pub(crate) struct BuildEnvironment<'a> {
  /// Actual build folder. This is where Deployer runs all the Actions.
  pub(crate) build_dir: &'a Path,
  /// System-wide or local common cache folder (usually `~/.cache`).
  pub(crate) cache_dir: &'a Path,
  /// System-wide or local common configuration folder (usually `~/.config`).
  pub(crate) config_dir: &'a Path,
  /// Project folder.
  pub(crate) project_dir: Option<&'a Path>,
  /// Deployer storage folder.
  pub(crate) storage_dir: &'a Path,
  /// Artifacts folder (usually `%project_dir%/artifacts`).
  pub(crate) artifacts_dir: &'a Path,
  /// Remote hosts's Registry. Used to find hosts by short name.
  pub(crate) remotes: &'a HashMap<ShortName, RemoteHost>,
  /// What files you should ignore when running Pipelines remotely.
  pub(crate) ignore: &'a HashSet<PathBuf>,
  /// New build flag. Pipeline execution will run all Actions.
  pub(crate) new_build: bool,
  /// Silent build flag. Deployer will not print anything on the screen.
  pub(crate) silent_build: bool,
  /// No I/O redirection flag. Deployer will not collect any command's output.
  pub(crate) no_pipe: bool,
}
