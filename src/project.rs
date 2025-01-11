use std::path::PathBuf;

use crate::cmd::InitArgs;
use crate::configs::{DeployerProjectOptions, DeployerGlobalConfig};
use crate::entities::{
  programming_languages::{ProgrammingLanguage, specify_programming_languages},
  traits::{Edit, EditExtended},
};
use crate::hmap;
use crate::i18n;
use crate::tui::add::{collect_af_inplacements, collect_artifacts, collect_targets, collect_variables};

pub(crate) fn init_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
  _args: &InitArgs,
) -> anyhow::Result<()> {
  let curr_dir = std::env::current_dir().expect("Can't get current dir!").to_str().expect("Can't convert current dir's path to string!").to_owned();
  if !globals.projects.contains(&curr_dir) { globals.projects.push(curr_dir.to_owned()); }
  
  config.init_from_prompt(curr_dir)?;
  
  println!("{}", i18n::INIT_SUCC);
  
  Ok(())
}

impl DeployerProjectOptions {
  pub(crate) fn init_from_prompt(&mut self, curr_dir: String) -> anyhow::Result<()> {
    use inquire::Text;
    
    #[cfg(unix)]
    let curr_dir = curr_dir.split('/').last().unwrap();
    let project_name_proposal = if self.project_name.is_empty() {
      curr_dir.to_owned()
    } else {
      self.project_name.to_owned()
    };
    self.project_name = Text::new(i18n::PROJECT_NAME).with_initial_value(project_name_proposal.as_str()).prompt()?;
    
    self.cache_files.push(PathBuf::from(".git"));
    println!("{}", i18n::PROJECT_SPECIFY_PLS);
    self.langs = specify_programming_languages()?;
    for lang in &self.langs {
      match lang {
        ProgrammingLanguage::Rust => self.cache_files.extend_from_slice(&[PathBuf::from("Cargo.lock"), PathBuf::from("target")]),
        ProgrammingLanguage::Go => self.cache_files.extend_from_slice(&[PathBuf::from("go.sum"), PathBuf::from("vendor")]),
        ProgrammingLanguage::Python => self.cache_files.extend_from_slice(&[PathBuf::from("__pycache__"), PathBuf::from("dist")]),
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => self.cache_files.extend_from_slice(&[PathBuf::from("CMakeFiles"), PathBuf::from("CMakeCache.txt")]),
        _ => {},
      }
    }
    
    self.deploy_toolkit = Text::new(&format!("{} {}:", i18n::PROJECT_DEPL_TOOLKIT, i18n::OR_HIT_ESC)).prompt_skippable()?;
    self.targets = collect_targets()?;
    self.variables = collect_variables()?;
    self.artifacts = collect_artifacts()?;
    self.inplace_artifacts_into_project_root = collect_af_inplacements(&self.artifacts)?;
    
    Ok(())
  }
  
  pub(crate) fn edit_project_from_prompt(&mut self, globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
    let actions = vec![
      i18n::EDIT_PROJECT_PIPELINES,
      i18n::EDIT_DEFAULT,
      i18n::EDIT_PROJECT_NAME,
      i18n::EDIT_PROJECT_REASSIGN,
      i18n::EDIT_CACHE,
      i18n::EDIT_PLS,
      i18n::EDIT_TARGETS,
      i18n::EDIT_DEPL_TOOLKIT,
      i18n::EDIT_PROJECT_VARS,
      i18n::EDIT_ARTIFACTS,
      i18n::EDIT_AF_INPLACE,
    ];
    
    while let Some(action) = inquire::Select::new(
      &format!("{} {}:", i18n::EDIT_ACTION_PROMPT, i18n::HIT_ESC),
      actions.clone(),
    ).prompt_skippable()? {
      match action {
        i18n::EDIT_PROJECT_NAME => self.project_name = inquire::Text::new(i18n::PROJECT_NAME).prompt()?,
        i18n::EDIT_DEFAULT => self.select_default_pipeline()?,
        i18n::EDIT_CACHE => self.cache_files.edit_from_prompt()?,
        i18n::EDIT_PLS => self.langs.edit_from_prompt()?,
        i18n::EDIT_TARGETS => self.targets.edit_from_prompt()?,
        i18n::EDIT_DEPL_TOOLKIT => self.deploy_toolkit = inquire::Text::new(
          &format!("{} {}:", i18n::DEPL_TOOLKIT, i18n::OR_HIT_ESC)
        ).prompt_skippable()?,
        i18n::EDIT_PROJECT_VARS => self.variables.edit_from_prompt()?,
        i18n::EDIT_ARTIFACTS => self.artifacts.edit_from_prompt()?,
        i18n::EDIT_AF_INPLACE => self.inplace_artifacts_into_project_root.edit_from_prompt(&mut self.artifacts)?,
        i18n::EDIT_PROJECT_PIPELINES => self.pipelines.edit_from_prompt(globals)?,
        i18n::EDIT_PROJECT_REASSIGN => for pipeline in &mut self.pipelines {
          for action in &mut pipeline.actions {
            *action = action.prompt_setup_for_project(&self.langs, &self.deploy_toolkit, &self.targets, &self.variables, &self.artifacts)?;
          }
        },
        _ => {},
      }
    }
    
    Ok(())
  }
  
  pub(crate) fn select_default_pipeline(&mut self) -> anyhow::Result<()> {
    match self.pipelines.len() {
      0 => {
        println!("{}", i18n::PROJECT_NO_PIPELINES);
      },
      1 => {
        let pipeline = self.pipelines.first_mut().unwrap();
        pipeline.default = Some(true);
      },
      _ => {
        let mut cmap = hmap!();
        let mut cs = vec![];
        
        self.pipelines.iter_mut().for_each(|c| {
          let s = format!("{} `{}`", i18n::PIPELINE, c.title);
          
          cmap.insert(s.clone(), c);
          cs.push(s);
        });
        
        if let Some(pipe) = inquire::Select::new(&format!("{} {}:", i18n::EDIT_DEFAULT_PROMPT, i18n::HIT_ESC), cs).prompt_skippable()? {
          for (key, val) in cmap.iter_mut() {
            if key.as_str().eq(pipe.as_str()) { val.default = Some(true); }
            else { val.default = Some(false); }
          }
        }
      },
    }
    
    Ok(())
  }
}

pub(crate) fn edit_project(
  globals: &mut DeployerGlobalConfig,
  config: &mut DeployerProjectOptions,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("{}", i18n::CFG_INVALID); }
  
  config.edit_project_from_prompt(globals)?;
  Ok(())
}
