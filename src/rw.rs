//! R/W utils module.

use anyhow::bail;
use serde::{Serialize, de::DeserializeOwned};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::entities::traits::{ConfigAutoMigrate, Merge};
use crate::{CACHE_DIR, LOGS_DIR};

pub static VERBOSE: OnceLock<bool> = OnceLock::new();
const LOG_FILE_DELIMETER: &str = "================================================================";

/// Reads the contents of the file or provides `Default::default()` if it cannot.
pub fn read<T: DeserializeOwned + Default>(folder: impl AsRef<Path>, file: impl AsRef<Path>) -> T {
  let mut path = PathBuf::new();
  path.push(folder.as_ref());
  path.push(file.as_ref());

  read_checked(&path).unwrap_or_else(|e| {
    log(format!("Error on file read: {:?}", e));
    Default::default()
  })
}

/// Reads the contents of the file or provides `Default::default()` if it cannot.
pub fn read_or_migrate<T: DeserializeOwned + Default + ConfigAutoMigrate<T>>(
  folder: impl AsRef<Path>,
  file: impl AsRef<Path>,
) -> T {
  let mut path = PathBuf::new();
  path.push(folder.as_ref());
  path.push(file.as_ref());

  read_checked(&path).unwrap_or_else(|_| {
    T::migrate(&path).unwrap_or_else(|e| {
      log(format!("Error on file read & migration: {:?}", e));
      Default::default()
    })
  })
}

/// Reads the contents of a file as type `T`.
pub fn read_checked<T: DeserializeOwned>(filepath: impl AsRef<Path>) -> anyhow::Result<T> {
  let content = std::fs::read_to_string(filepath.as_ref())?;
  match filepath
    .as_ref()
    .extension()
    .map(|v| v.to_string_lossy().to_string())
    .as_deref()
  {
    Some("json") => serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("{}", e)),
    Some("yaml") | Some("yml") => serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("{}", e)),
    Some("toml") => toml::from_str(&content).map_err(|e| anyhow::anyhow!("{}", e)),
    _ => bail!("No supported format!"),
  }
}

/// Writes `T` to a file, ignoring write and serialization errors.
///
/// All errors are written only to the log, which can be seen with the `-V` flag.
pub fn write<T: Serialize>(folder: impl AsRef<Path>, file: impl AsRef<Path>, config: &T) {
  let mut path = PathBuf::new();
  path.push(folder);
  path.push(file.as_ref());

  let content = match path.extension().map(|v| v.to_string_lossy().to_string()).as_deref() {
    Some("json") => serde_json::to_string_pretty(config).map_err(|e| anyhow::anyhow!("{}", e)),
    Some("yaml") | Some("yml") => serde_yaml::to_string(config).map_err(|e| anyhow::anyhow!("{}", e)),
    Some("toml") => toml::to_string_pretty(config).map_err(|e| anyhow::anyhow!("{}", e)),
    _ => {
      log("No supported format!");
      return;
    }
  };
  let content = match content {
    Ok(s) => s,
    Err(_) => {
      log(format!("Can't save `{:?}` config file!", file.as_ref().as_os_str()));
      return;
    }
  };

  match std::fs::write(path, content) {
    Ok(()) => {}
    Err(_) => log(format!("Can't save `{:?}` config file!", file.as_ref().as_os_str())),
  }
}

/// Writes `T` to a file, ignoring write and serialization errors, but with merging.
///
/// All errors are written only to the log, which can be seen with the `-V` flag.
pub fn write_merge<T: Serialize + Merge + Default + DeserializeOwned + Clone>(
  folder: impl AsRef<Path>,
  file: impl AsRef<Path>,
  config: &T,
) {
  let mut path = PathBuf::new();
  path.push(folder);
  path.push(file.as_ref());

  let other = read_checked(&path).unwrap_or_default();
  let merged: T = config.merge(other).unwrap_or((*config).clone());

  let content = match path.extension().map(|v| v.to_string_lossy().to_string()).as_deref() {
    Some("json") => serde_json::to_string_pretty(&merged).map_err(|e| anyhow::anyhow!("{}", e)),
    Some("yaml") | Some("yml") => serde_yaml::to_string(&merged).map_err(|e| anyhow::anyhow!("{}", e)),
    Some("toml") => toml::to_string_pretty(&merged).map_err(|e| anyhow::anyhow!("{}", e)),
    _ => {
      log("No supported format!");
      return;
    }
  };
  let content = match content {
    Ok(s) => s,
    Err(_) => {
      log(format!("Can't save `{:?}` config file!", file.as_ref().as_os_str()));
      return;
    }
  };

  match std::fs::write(path, content) {
    Ok(()) => {}
    Err(_) => log(format!("Can't save `{:?}` config file!", file.as_ref().as_os_str())),
  }
}

/// Function of recursive copying of contents.
///
/// If `src` is a folder, then:
/// - first, all missing subfolders for `dst` and the `dst` folder itself are created if they do not exist;
/// - then files are copied and overwritten, symlinks are created, folders are copied by calling the same function.
///
/// If `src` is a file, then all subfolders up to `dst` are created, and then the file is copied and overwritten.
///
/// Previously existing folders and files, unless overwritten, are not changed and are stored in their places.
pub fn copy_all(
  root: impl AsRef<Path>,
  src: impl AsRef<Path>,
  dst: impl AsRef<Path>,
  ignore: &[impl AsRef<Path>],
) -> anyhow::Result<()> {
  if src.as_ref().is_file() {
    if let Some(parent) = dst.as_ref().parent() {
      std::fs::create_dir_all(parent)?;
    }
    if let Err(e) = std::fs::copy(src.as_ref(), dst.as_ref()) {
      log(format!("-> {:?} :: {}", src.as_ref(), e));
    }
    return Ok(());
  }
  std::fs::create_dir_all(&dst)?;

  for entry in std::fs::read_dir(src.as_ref())? {
    let entry = entry?;
    let entry_path = entry.path();
    let relative_entry = entry_path.strip_prefix(root.as_ref())?;

    if ignore.iter().any(|v| v.as_ref().eq(relative_entry)) {
      continue;
    }

    log(format!("-> {:?}", relative_entry));

    let ty = entry.file_type()?;
    let d = dst.as_ref().join(entry.file_name());
    if ty.is_dir() {
      copy_all(root.as_ref(), entry.path(), d, ignore)?;
    } else if ty.is_file() {
      copy_if_different(entry.path(), d)?;
    } else if ty.is_symlink() {
      symlink(std::fs::canonicalize(entry.path())?, d);
    }
  }

  Ok(())
}

/// Creates UNIX symlink.
pub fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) {
  use std::os::unix::fs::symlink as os_symlink;

  match os_symlink(src.as_ref(), dst) {
    Ok(_) => (),
    Err(e) => {
      log(format!("Skip `{}` due to: {:?}", src.as_ref().to_str().unwrap(), e));
    }
  }
}

/// Copies only if the files are different from each other.
fn copy_if_different(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
  use std::io::Read;

  let src_path = src.as_ref();
  let dst_path = dst.as_ref();

  if !dst_path.exists() {
    return Ok(std::fs::copy(src_path, dst_path).map(|_| ())?);
  }

  if src_path.metadata()?.len() != dst_path.metadata()?.len() {
    return Ok(std::fs::copy(src_path, dst_path).map(|_| ())?);
  }

  let mut src_file = std::fs::File::open(src_path)?;
  let mut dst_file = std::fs::File::open(dst_path)?;

  let mut src_buffer = [0; 8192]; // 8KB chunks
  let mut dst_buffer = [0; 8192];

  loop {
    let src_bytes = src_file.read(&mut src_buffer)?;
    let dst_bytes = dst_file.read(&mut dst_buffer)?;

    if src_bytes != dst_bytes {
      return Ok(std::fs::copy(src_path, dst_path).map(|_| ())?);
    }
    if src_bytes == 0 {
      break;
    }

    if src_buffer[..src_bytes] != dst_buffer[..dst_bytes] {
      return Ok(std::fs::copy(src_path, dst_path).map(|_| ())?);
    }
  }

  Ok(())
}

pub fn log(s: impl AsRef<str>) {
  if *VERBOSE.wait() {
    println!("{}", s.as_ref());
  }
}

/// Generates the path to the build log depending on the project and Pipeline.
pub fn generate_build_log_filepath(project_name: &str, pipeline_short_name: &str, cache_dir: &Path) -> PathBuf {
  use chrono::Local;

  let mut logs_path = PathBuf::new();
  logs_path.push(cache_dir);
  logs_path.push(CACHE_DIR);
  logs_path.push(LOGS_DIR);
  if !logs_path.exists() {
    std::fs::create_dir_all(logs_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", logs_path));
  }

  let curr_dt = Local::now();

  let log_path = logs_path.join(format!(
    "{}-{}-{}.txt",
    project_name.replace('/', "-"),
    pipeline_short_name,
    curr_dt.format("%Y-%m-%d-%H:%M")
  ));
  if log_path.exists() {
    build_log(&log_path, &[LOG_FILE_DELIMETER.to_string()]).expect("Current log file is unwriteable!");
  }

  log_path
}

/// Writes a build log message to a file.
pub fn build_log(path: &Path, output: &[String]) -> anyhow::Result<()> {
  use std::io::Write;

  let file = File::options().create(true).append(true).open(path)?;
  let mut writer = BufWriter::new(file);
  for line in output {
    let line = strip_ansi_escapes::strip(line.as_bytes());
    writer.write_all(&line)?;
    writer.write_all("\n".as_bytes())?;
  }

  Ok(())
}
