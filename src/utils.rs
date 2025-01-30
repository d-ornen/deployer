//! Other Deployer utils.

use regex::Regex;
use serde::Deserialize;

#[macro_export]
macro_rules! hmap {
  () => {
    std::collections::HashMap::new()
  };
}

#[macro_export]
macro_rules! hset {
  () => {
    std::collections::HashSet::new()
  };
}

pub fn get_current_working_dir() -> std::io::Result<std::path::PathBuf> {
  std::env::current_dir()
}

#[cfg(feature = "tui")]
pub fn tags_custom_type<'a>(message: &'a str, default: Option<&'a str>) -> inquire::CustomType<'a, Vec<String>> {
  inquire::CustomType {
    message,
    starting_input: if default.is_some() { default } else { None },
    default: Some(vec![]),
    placeholder: None,
    help_message: None,
    formatter: &|val| val.join(", ").to_string(),
    default_value_formatter: &|val| val.join(", ").to_string(),
    parser: &|a| Ok(a.split(',').map(|s| s.trim().to_owned()).collect::<Vec<_>>()),
    validators: inquire::CustomType::DEFAULT_VALIDATORS,
    error_message: "Invalid input".into(),
    render_config: inquire::ui::RenderConfig::default(),
  }
}

pub fn str2regex_simple(s: &str) -> anyhow::Result<Regex> {
  Ok(Regex::new(s)?)
}

pub fn str2regexopt<'de, D>(deserializer: D) -> Result<Option<Regex>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  use serde::de::Error;
  Option::<String>::deserialize(deserializer).and_then(|option| match option {
    Some(string) => Regex::new(string.as_str())
      .map(Some)
      .map_err(|err| Error::custom(err.to_string())),
    None => Ok(None),
  })
}

pub fn regexopt2str<S>(v: &Option<Regex>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  match v {
    Some(r) => serializer.serialize_str(r.as_str()),
    None => serializer.serialize_none(),
  }
}

pub fn ordered_map<S, K: Ord + serde::Serialize, V: serde::Serialize>(
  value: &std::collections::HashMap<K, V>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  use serde::Serialize;
  let ordered: std::collections::BTreeMap<_, _> = value.iter().collect();
  ordered.serialize(serializer)
}

pub fn ordered_set<S, V: Ord + serde::Serialize>(
  value: &std::collections::HashSet<V>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  use serde::Serialize;
  let ordered: std::collections::BTreeSet<_> = value.iter().collect();
  ordered.serialize(serializer)
}
