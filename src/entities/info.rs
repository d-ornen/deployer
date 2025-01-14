use anyhow::bail;
use regex::Regex;
use serde::{Deserialize, Serialize, Deserializer, Serializer};
use std::sync::LazyLock;

use crate::i18n;

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ShortName(String);

impl ShortName {
  pub(crate) fn new(short_name: impl AsRef<str>) -> anyhow::Result<Self> {
    if !validate_short_name(short_name.as_ref()) {
      bail!(i18n::INCORRECT_SHORT_NAME)
    }
    Ok(Self(short_name.as_ref().to_string()))
  }
  
  pub(crate) fn as_str(&self) -> &str { self.0.as_str() }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Version(String);

impl Version {
  pub(crate) fn new(version: impl AsRef<str>) -> anyhow::Result<Self> {
    if !validate_version(version.as_ref()) {
      bail!(i18n::INCORRECT_VERSION)
    }
    Ok(Self(version.as_ref().to_string()))
  }
  
  pub(crate) fn new_for_using(version: impl AsRef<str>) -> anyhow::Result<Self> {
    if !version.as_ref().eq("latest") && !validate_version(version.as_ref()) {
      bail!(i18n::INCORRECT_VERSION)
    }
    Ok(Self(version.as_ref().to_string()))
  }
  
  pub(crate) fn as_str(&self) -> &str { self.0.as_str() }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Info {
  short_name: ShortName,
  version: Version,
}

pub(crate) trait StrToInfo {
  fn to_info(&self) -> anyhow::Result<Info>;
}

impl StrToInfo for String { fn to_info(&self) -> anyhow::Result<Info> { Info::from_str(self) } }
impl StrToInfo for &String { fn to_info(&self) -> anyhow::Result<Info> { Info::from_str(self) } }
impl StrToInfo for &str { fn to_info(&self) -> anyhow::Result<Info> { Info::from_str(self) } }

impl<'de> Deserialize<'de> for Info {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> { str2info(deserializer) }
}

impl Serialize for Info {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer { info2str(self, serializer) }
}

static SHORT_NAME_VALIDATOR: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new("^[a-zA-Z_0-9-_]*$").unwrap()
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
      Ok(Self::from(ShortName(short_name.as_ref().to_owned()), Version(version.as_ref().to_owned())))
    }
  }
  
  pub(crate) fn from(short_name: ShortName, version: Version) -> Self {
    Self { short_name, version }
  }
  
  // allow use `latest` version for `UseFromStorage` Action
  pub(crate) fn new_for_using(short_name: impl AsRef<str>, version: impl AsRef<str>) -> anyhow::Result<Self> {
    if !validate_short_name(short_name.as_ref()) {
      bail!(i18n::INCORRECT_SHORT_NAME)
    } else if !version.as_ref().eq("latest") && !validate_version(version.as_ref()) {
      bail!(i18n::INCORRECT_VERSION)
    } else {
      Ok(Self::from(ShortName(short_name.as_ref().to_owned()), Version(version.as_ref().to_owned())))
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
    format!("{}@{}", self.short_name.as_str(), self.version.as_str())
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
