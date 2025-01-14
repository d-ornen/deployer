use anyhow::bail;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

use crate::i18n;

#[derive(Debug, Clone)]
pub(crate) struct Info {
  short_name: String,
  version: String,
}

impl PartialEq for Info {
  fn eq(&self, other: &Self) -> bool {
    self.short_name.as_str().eq(other.short_name.as_str())
  }
}

impl PartialOrd for Info {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    self.version.as_str().partial_cmp(other.version.as_str())
  }
}

static SHORT_NAME_VALIDATOR: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new("^[a-zA-Z_0-9-]*$").unwrap()
});

static VERSION_VALIDATOR: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r#"^(0|[1-9]\d*)(?:\.(0|[1-9]\d*))?(?:\.(0|[1-9]\d*))?$"#).unwrap()
});

pub(crate) fn validate_short_name(short_name: &str) -> bool {
  SHORT_NAME_VALIDATOR.is_match(short_name)
}

pub(crate) fn validate_version(version: &str) -> bool {
  VERSION_VALIDATOR.is_match(version)
}

impl Info {
  pub(crate) fn new(short_name: impl AsRef<str>, version: impl AsRef<str>) -> anyhow::Result<Self> {
    if !validate_short_name(short_name.as_ref()) {
      bail!(i18n::INCORRECT_SHORT_NAME)
    } else if !validate_version(version.as_ref()) {
      bail!(i18n::INCORRECT_VERSION)
    } else {
      Ok(Self {
        short_name: short_name.as_ref().to_owned(),
        version: version.as_ref().to_owned(),
      })
    }
  }
  
  // allow use `latest` version for `UseFromStorage` Action
  pub(crate) fn new_for_using(short_name: impl AsRef<str>, version: impl AsRef<str>) -> anyhow::Result<Self> {
    if !short_name.as_ref().eq("latest") && !validate_short_name(short_name.as_ref()) {
      bail!(i18n::INCORRECT_SHORT_NAME)
    } else if !validate_version(version.as_ref()) {
      bail!(i18n::INCORRECT_VERSION)
    } else {
      Ok(Self {
        short_name: short_name.as_ref().to_owned(),
        version: version.as_ref().to_owned(),
      })
    }
  }
  
  pub(crate) fn short_name(&self) -> &str {
    self.short_name.as_str()
  }
  
  #[allow(dead_code)]
  pub(crate) fn version(&self) -> &str {
    self.version.as_str()
  }
  
  pub(crate) fn to_str(&self) -> String {
    format!("{}@{}", self.short_name, self.version)
  }
  
  pub(crate) fn from_str(short_name_and_ver: &str) -> anyhow::Result<Self> {
    let vals = short_name_and_ver.split('@').collect::<Vec<_>>();
    if let Some(short_name) = vals.first() && let Some(version) = vals.get(1) {
      Info::new(short_name, version)
    } else {
      bail!("Short name and version must be divided by `@` character!")
    }
  }
}

pub(crate) type ActionInfo = Info;
pub(crate) type PipelineInfo = Info;
pub(crate) type ContentInfo = Info;

pub(crate) fn str2info<'de, D>(deserializer: D) -> Result<Info, D::Error>
where
  D: serde::Deserializer<'de>,
{
  use serde::de::Error;
  String::deserialize(deserializer).and_then(|string| {
    match Info::from_str(string.as_str()) {
      Ok(v) => Ok(v),
      Err(e) => Err(Error::custom(&e))
    }
  })
}

pub(crate) fn info2str<S>(v: &Info, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  serializer.serialize_str(v.to_str().as_str())
}
