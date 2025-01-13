use serde::{de::DeserializeOwned, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::OnceLock;
use std::path::{Path, PathBuf};
use crate::{CACHE_DIR, LOGS_DIR, PROJECT_CONF};

pub(crate) static VERBOSE: OnceLock<bool> = OnceLock::new();
const LOG_FILE_DELIMETER: &str = "================================================================";

/// Считывает содержимое файла или предоставляет `Default::default()`, если не может.
/// 
/// Например, если файла не существует, или его содержимое не является валидным JSON'ом, то будет
/// возвращён `Default::default()`.
pub(crate) fn read<T: DeserializeOwned + Default>(folder: impl AsRef<Path>, file: impl AsRef<Path>) -> T {
  let mut path = PathBuf::new();
  path.push(folder);
  path.push(file);
  
  match read_checked(path) {
    Err(e) => {
      log(format!("Error on file read: {:?}", e));
      Default::default()
    },
    Ok(v) => v,
  }
}

/// Считывает содержимое файла как тип `T`.
pub(crate) fn read_checked<T: DeserializeOwned>(filepath: impl AsRef<Path>) -> anyhow::Result<T> {
  let file = File::open(filepath.as_ref())?;
  let reader = BufReader::new(file);
  
  match filepath.as_ref().extension().unwrap().to_str().unwrap().to_lowercase().as_str() {
    "json" => Ok(serde_json::from_reader(reader)?),
    _ => Err(anyhow::anyhow!("Unsupported file extension!"))
  }
}

/// Записывает `T` в файл, игнорируя ошибки записи и сериализации.
/// 
/// Все ошибки записываются только в лог, который можно увидеть с флагом `-V`.
pub(crate) fn write<T: Serialize>(folder: impl AsRef<Path>, file: impl AsRef<Path>, config: &T) {
  let mut path = PathBuf::new();
  path.push(folder);
  path.push(file.as_ref());
  let f = match File::create(path) {
    Ok(file) => file,
    Err(_) => {
      log(format!("Can't save `{:?}` config file!", file.as_ref().as_os_str()));
      return
    }
  };
  
  let writer = BufWriter::new(f);
  
  match serde_json::to_writer_pretty(writer, config) {
    Ok(_) => (),
    Err(_) => {
      log(format!("Can't save `{:?}` config file due to serialization error!", file.as_ref().as_os_str()));
    },
  }
}

/// Функция рекурсивного копирования содержимого.
/// 
/// Если `src` - это папка, то:
/// - сначала создаются все отсутствующие подпапки для `dst` и сама папка `dst`, если их нет;
/// - затем файлы копируются с перезаписью, симлинки - создаются, папки - копируются через вызов этой же функции.
/// 
/// Если `src` - это файл, то до `dst` создаются все подпапки, а потом файл копируется с перезаписью.
/// 
/// Ранее имеющиеся папки и файлы, - если не перезаписываются, - не изменяются и сохраняются на своих местах.
pub(crate) fn copy_all(src: impl AsRef<Path>, dst: impl AsRef<Path>, ignore: &[impl AsRef<Path>]) -> anyhow::Result<()> {
  if src.as_ref().is_file() {
    if let Some(parent) = dst.as_ref().parent() {
      std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(src.as_ref(), dst.as_ref())?;
    return Ok(())
  }
  std::fs::create_dir_all(&dst)?;
  
  for entry in std::fs::read_dir(src)? {
    let entry = entry?;
    let name = entry.file_name();
    
    if ignore.iter().any(|v| v.as_ref().as_os_str().eq(name.as_os_str())) { continue }
    
    log(format!("-> {:?}", name));
    
    let ty = entry.file_type()?;
    let d = dst.as_ref().join(entry.file_name());
    if ty.is_dir() {
      copy_all(entry.path(), d, ignore)?;
    } else if name == PROJECT_CONF {
      log(format!("Symlinking `{:?}` from {:?} to {:?}", name, entry.path(), d));
      symlink(std::fs::canonicalize(entry.path())?, d);
    } else if ty.is_file() {
      copy_if_different(entry.path(), d)?;
    } else if ty.is_symlink() {
      symlink(std::fs::canonicalize(name)?, d);
    }
  }
  
  Ok(())
}

/// Создаёт ссылку UNIX.
pub(crate) fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) {
  use std::os::unix::fs::symlink as os_symlink;
  
  match os_symlink(src.as_ref(), dst) {
    Ok(_) => (),
    Err(e) => {
      log(format!("Skip `{}` due to: {:?}", src.as_ref().to_str().unwrap(), e));
    },
  }
}

/// Копирует, только если файлы отличаются друг от друга.
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

    if src_bytes != dst_bytes { return Ok(std::fs::copy(src_path, dst_path).map(|_| ())?); }
    if src_bytes == 0 { break; }

    if src_buffer[..src_bytes] != dst_buffer[..dst_bytes] { return Ok(std::fs::copy(src_path, dst_path).map(|_| ())?); }
  }

  Ok(())
}

/// Используется для логгирования ошибок.
pub(crate) fn log(s: impl AsRef<str>) {
  if *VERBOSE.wait() {
    println!("{}", s.as_ref());
  }
}

/// Генерирует путь до лога сборки в зависимости от проекта и пайплайна.
pub(crate) fn generate_build_log_filepath(
  project_name: &str,
  pipeline_short_name: &str,
  cache_dir: &Path,
) -> PathBuf {
  use chrono::Local;
  
  let mut logs_path = PathBuf::new();
  logs_path.push(cache_dir);
  logs_path.push(CACHE_DIR);
  logs_path.push(LOGS_DIR);
  if !logs_path.exists() { std::fs::create_dir_all(logs_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", logs_path)); }
  
  let curr_dt = Local::now();
  
  let log_path = logs_path.join(format!("{}-{}-{}.txt", project_name.replace('/', "-"), pipeline_short_name, curr_dt.format("%Y-%m-%d-%H:%M")));
  if log_path.exists() { build_log(&log_path, &[LOG_FILE_DELIMETER.to_string()]).expect("Current log file is unwriteable!"); }
  
  log_path
}

/// Записывает лог сборки в файл.
pub(crate) fn build_log(
  path: &Path,
  output: &[String],
) -> anyhow::Result<()> {
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
