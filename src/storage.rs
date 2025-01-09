use anyhow::bail;
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::entities::info::ContentInfo;
use crate::i18n;
use crate::STORAGE_DIR;
use crate::rw::copy_all;

pub(crate) fn list_content(
  storage_dir: &Path,
) -> anyhow::Result<()> {
  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);
  
  println!("{}", i18n::CONTENT_AVAILABLE);
  
  for entry in std::fs::read_dir(content_path)? {
    let entry = entry?;
    if !entry.path().is_dir() { continue }
    
    if let Ok(name) = entry.file_name().into_string() && let Ok(info) = ContentInfo::from_str(name.as_str()) {
      println!("• {} ({}: {:?})", info.to_str().blue().bold(), i18n::PATH, entry.path());
    }
  }
  
  Ok(())
}

pub(crate) fn new_content(
  storage_dir: &Path,
) -> anyhow::Result<()> {
  use inquire::Text;
  
  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);
  
  println!("{}", i18n::CONTENT_GUIDE_1);
  println!("{}", i18n::CONTENT_GUIDE_2);
  println!("{}", i18n::CONTENT_GUIDE_3);
  
  let path = PathBuf::from(Text::new(i18n::CONTENT_SPECIFY_PATH).prompt()?);
  if !path.exists() || !path.is_dir() { bail!("There is no such folder!") }
  
  println!("{}", i18n::CONTENT_GUIDE_4);
  println!("{}", i18n::CONTENT_GUIDE_5);
  
  let short_name = Text::new(i18n::CONTENT_INFO).prompt()?;
  let version = Text::new(i18n::CONTENT_VER).prompt()?;
  let info = ContentInfo::new(short_name, version)?;
  
  let new_path = content_path.join(info.to_str());
  if new_path.exists() { std::fs::remove_dir_all(&new_path)?; }
  copy_all(path, &new_path, &[""])?;
  
  println!("{}", i18n::CONTENT_ADDED_SUCC.replace("{1}", &info.to_str()).replace("{2}", &format!("{:?}", new_path)));
  
  Ok(())
}

pub(crate) fn use_from_storage(
  storage_dir: &Path,
  build_dir: &Path,
  content_info: &ContentInfo,
) -> anyhow::Result<()> {
  let content_info_str = content_info.to_str();
  
  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);
  
  if !content_info_str.ends_with("latest") {
    content_path.push(&content_info_str);
    if !content_path.exists() { bail!("{}: `{}`. {}", i18n::NO_SUCH_CONTENT, i18n::CONTENT_CONSIDER_ADD, content_info_str) }
    copy_all(&content_path, build_dir, &[""])?;
  } else {
    let mut versions = vec![];
    for entry in std::fs::read_dir(&content_path)? {
      let entry = entry?;
      let name = entry.file_name().to_str().unwrap().to_owned();
      if name.starts_with(content_info.short_name()) { versions.push(name); }
    }
    if versions.is_empty() { bail!("{}: `{}`. {}", i18n::NO_SUCH_CONTENT, i18n::CONTENT_CONSIDER_ADD, content_info_str) }
    let max = versions.iter().max().unwrap();
    content_path.push(max);
    if !content_path.exists() { bail!("{}: `{}`. {}", i18n::NO_SUCH_CONTENT, i18n::CONTENT_CONSIDER_ADD, content_info_str) }
    copy_all(&content_path, build_dir, &[""])?;
  }
  
  Ok(())
}

pub(crate) fn add_to_storage(
  storage_dir: &Path,
  artifacts_dir: &Path,
  content_info: &ContentInfo,
) -> anyhow::Result<()> {
  let content_info_str = content_info.to_str();
  
  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);
  content_path.push(&content_info_str);
  
  if content_path.exists() { std::fs::remove_dir_all(&content_path)?; }
  copy_all(artifacts_dir, &content_path, &[""])?;
  
  Ok(())
}
