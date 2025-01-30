//! Storage module.

use anyhow::bail;
#[cfg(feature = "tui")]
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::STORAGE_DIR;
use crate::entities::info::ContentInfo;
#[cfg(feature = "tui")]
use crate::entities::info::ShortName;
use crate::i18n;
use crate::rw::copy_all;

pub const CUSTOM_STORAGE_PATH: &str = "DEPLOYER_STORAGE_PATH";

/// Lists all available content in Deployer's storage.
#[cfg(feature = "tui")]
pub fn list_content(storage_dir: &Path) -> anyhow::Result<()> {
  let content_path = if let Ok(custom_storage_dir) = std::env::var(CUSTOM_STORAGE_PATH) {
    PathBuf::from(custom_storage_dir)
  } else {
    PathBuf::from(storage_dir).join(STORAGE_DIR)
  };

  println!("{}", i18n::CONTENT_AVAILABLE);

  let mut content = vec![];
  match std::fs::read_dir(content_path) {
    Err(_) => {}
    Ok(entries) => {
      for entry in entries {
        let entry = entry?;
        if !entry.path().is_dir() {
          continue;
        }

        if let Ok(name) = entry.file_name().into_string()
          && let Ok(info) = ContentInfo::try_from_str(name.as_str())
        {
          content.push(format!(
            "• {} ({}: {:?})",
            info.to_str().blue().bold(),
            i18n::PATH,
            entry.path()
          ));
        }
      }
    }
  }
  content.sort_unstable();
  for line in content {
    println!("{}", line);
  }

  Ok(())
}

/// Creates a new content from given folder.
#[cfg(feature = "tui")]
pub fn new_content(storage_dir: &Path) -> anyhow::Result<()> {
  use inquire::Text;

  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);

  println!("{}", i18n::CONTENT_GUIDE_1);
  println!("{}", i18n::CONTENT_GUIDE_2);
  println!("{}", i18n::CONTENT_GUIDE_3);

  let path = PathBuf::from(Text::new(i18n::CONTENT_SPECIFY_PATH).prompt()?);
  if !path.exists() || !path.is_dir() {
    bail!("There is no such folder!")
  }

  println!("{}", i18n::CONTENT_GUIDE_4);
  println!("{}", i18n::CONTENT_GUIDE_5);

  let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
  let version = Text::new(i18n::CONTENT_VER).prompt()?;
  let info = ContentInfo::new(short_name, version)?;

  let new_path = content_path.join(info.to_str());
  if new_path.exists() {
    std::fs::remove_dir_all(&new_path)?;
  }
  copy_all(&path, &path, &new_path, &[""])?;

  println!(
    "{}",
    i18n::CONTENT_ADDED_SUCC
      .replace("{1}", &info.to_str())
      .replace("{2}", &format!("{:?}", new_path))
  );

  Ok(())
}

/// Syncs content files to build folder.
pub fn use_from_storage(storage_dir: &Path, build_dir: &Path, content_info: &ContentInfo) -> anyhow::Result<()> {
  let content_info_str = content_info.to_str();

  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);

  if !content_info_str.ends_with("latest") {
    content_path.push(&content_info_str);
    if !content_path.exists() {
      bail!(
        "{}: `{}`. {}",
        i18n::NO_SUCH_CONTENT,
        content_info_str,
        i18n::CONTENT_CONSIDER_ADD
      )
    }
    copy_all(&content_path, &content_path, build_dir, &[""])?;
  } else {
    let mut versions = vec![];
    for entry in std::fs::read_dir(&content_path)? {
      let entry = entry?;
      let name = entry.file_name().to_str().unwrap().to_owned();
      if name.starts_with(content_info.short_name()) {
        versions.push(name);
      }
    }
    if versions.is_empty() {
      bail!(
        "{}: `{}`. {}",
        i18n::NO_SUCH_CONTENT,
        content_info_str,
        i18n::CONTENT_CONSIDER_ADD
      )
    }
    let max = versions.iter().max().unwrap();
    content_path.push(max);
    if !content_path.exists() {
      bail!(
        "{}: `{}`. {}",
        i18n::NO_SUCH_CONTENT,
        content_info_str,
        i18n::CONTENT_CONSIDER_ADD
      )
    }
    copy_all(&content_path, &content_path, build_dir, &[""])?;
  }

  Ok(())
}

/// Adds project artifacts as a content.
pub fn add_to_storage(storage_dir: &Path, artifacts_dir: &Path, content_info: &ContentInfo) -> anyhow::Result<()> {
  let content_info_str = content_info.to_str();

  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);
  content_path.push(&content_info_str);

  if content_path.exists() {
    return Ok(());
  }
  copy_all(artifacts_dir, artifacts_dir, &content_path, &[""])?;

  Ok(())
}

/// Removes the content from Deployer's storage.
#[cfg(feature = "tui")]
pub fn remove_content(storage_dir: &Path) -> anyhow::Result<()> {
  let content_short_name = ShortName::new(inquire::Text::new(i18n::CONTENT_INFO).prompt()?)?;

  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);

  let mut versions = vec![];
  for entry in std::fs::read_dir(&content_path)? {
    let entry = entry?;
    let name = entry.file_name().to_str().unwrap().to_owned();
    if name.starts_with(content_short_name.as_str()) {
      versions.push(name);
    }
  }

  if versions.is_empty() {
    return Ok(());
  }

  let variant = content_path.join(inquire::Select::new(i18n::CONTENT_SELECT_TO_REMOVE, versions).prompt()?);
  if variant.exists() {
    std::fs::remove_dir_all(variant)?;
  }

  Ok(())
}
