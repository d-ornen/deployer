use serde::{Deserialize, Serialize};

use crate::i18n;
use crate::utils::tags_custom_type;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub(crate) enum ProgrammingLanguage {
  Rust,
  Go,
  C,
  Cpp,
  Python,
  Other(String),
}

impl std::fmt::Display for ProgrammingLanguage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let lang = match self {
      Self::Rust => "Rust".to_string(),
      Self::Go => "Go".to_string(),
      Self::C => "C".to_string(),
      Self::Cpp => "C++".to_string(),
      Self::Python => "Python".to_string(),
      Self::Other(s) => s.to_owned(),
    };
    
    f.write_str(&lang)
  }
}

/// Парсит вводимые языки программирования.
pub(crate) fn specify_programming_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  use inquire::MultiSelect;
  
  let langs = vec!["Rust", "Go", "C", "C++", "Python", "Others"];
  let selected = MultiSelect::new(i18n::PL_SELECT, langs).prompt()?;
  
  let mut result = Vec::new();
  for lang in selected {
    let lang = match lang {
      "Rust" => ProgrammingLanguage::Rust,
      "Go" => ProgrammingLanguage::Go,
      "C" => ProgrammingLanguage::C,
      "C++" => ProgrammingLanguage::Cpp,
      "Python" => ProgrammingLanguage::Python,
      "Others" => {
        let langs = collect_multiple_languages()?;
        result.extend_from_slice(&langs);
        continue
      }
      _ => unreachable!(),
    };
    result.push(lang);
  }
  
  Ok(result)
}

fn collect_multiple_languages() -> anyhow::Result<Vec<ProgrammingLanguage>> {
  let langs = tags_custom_type(i18n::PL_COLLECT, None).prompt()?;
  let mut v = vec![];
  
  for lang in langs {
    match lang.as_str() {
      "Rust" | "Go" | "C" | "C++" | "Python" => continue,
      lang => v.push(ProgrammingLanguage::Other(lang.to_owned())),
    }
  }
  
  Ok(v)
}
