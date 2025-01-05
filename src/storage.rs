use anyhow::bail;
use std::path::{Path, PathBuf};

use crate::entities::info::ContentInfo;
use crate::i18n;
use crate::STORAGE_DIR;
use crate::rw::copy_all;

pub(crate) fn use_from_storage(
  storage_dir: &Path,
  build_dir: &Path,
  content_info: &ContentInfo,
) -> anyhow::Result<()> {
  let mut content_path = PathBuf::from(storage_dir);
  content_path.push(STORAGE_DIR);
  content_path.push(content_info.to_str());
  
  if !content_path.exists() { bail!("{}: `{}`. {}", i18n::NO_SUCH_CONTENT, i18n::CONTENT_CONSIDER_ADD, content_info.to_str()) }
  
  copy_all(&content_path, build_dir, &[""])?;
  
  Ok(())
}
