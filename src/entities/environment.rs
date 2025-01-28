//! Run environment module.
//!
//! `RunEnvironment` is just a struct that have all needed variables
//! and paths during Pipeline execution.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::entities::info::ShortName;
use crate::entities::remote_host::RemoteHost;

/// Run environment.
#[derive(Clone, Copy)]
pub struct RunEnvironment<'a> {
  /// Actual run folder. This is where Deployer runs all the Actions.
  pub run_dir: &'a Path,
  /// System-wide or local common cache folder (usually `~/.cache`).
  pub cache_dir: &'a Path,
  /// System-wide or local common configuration folder (usually `~/.config`).
  pub config_dir: &'a Path,
  /// Project folder.
  pub project_dir: Option<&'a Path>,
  /// Deployer storage folder.
  pub storage_dir: &'a Path,
  /// Artifacts folder (usually `%project_dir%/artifacts`).
  pub artifacts_dir: &'a Path,
  /// Remote hosts's Registry. Used to find hosts by short name.
  pub remotes: &'a HashMap<ShortName, RemoteHost>,
  /// What files you should ignore when running Pipelines remotely.
  pub ignore: &'a HashSet<PathBuf>,
  /// New run flag. Pipeline execution will run all Actions.
  pub new_build: bool,
  /// Silent run flag. Deployer will not print anything on the screen.
  pub silent_build: bool,
  /// No I/O redirection flag. Deployer will not collect any command's output.
  pub no_pipe: bool,
}
