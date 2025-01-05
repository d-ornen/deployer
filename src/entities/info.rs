use anyhow::bail;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Hash)]
pub(crate) struct Info {
  short_name: String,
  version: String,
}

static SHORT_NAME_VALIDATOR: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new("^[a-zA-Z_-]*$").unwrap()
});

static VERSION_VALIDATOR: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r#"^(0|[1-9]\d*)(?:\.(0|[1-9]\d*))?(?:\.(0|[1-9]\d*))?$"#).unwrap()
});

impl Info {
  pub(crate) fn new(short_name: impl AsRef<str>, version: impl AsRef<str>) -> anyhow::Result<Self> {
    if !SHORT_NAME_VALIDATOR.is_match(short_name.as_ref()) {
      bail!("Short names must only contain English characters and `_` and `-` characters.")
    } else if !VERSION_VALIDATOR.is_match(version.as_ref()) {
      bail!("Versions must be like this: `1`, `1.2`, or `1.2.3`.")
    } else {
      Ok(Self {
        short_name: short_name.as_ref().to_owned(),
        version: version.as_ref().to_owned(),
      })
    }
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
