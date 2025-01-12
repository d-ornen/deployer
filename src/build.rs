use colored::Colorize;
use fs_extra::dir::get_size;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::actions::Action;
use crate::{CACHE_DIR, ARTIFACTS_DIR, BUILD_CACHE_LIST};
use crate::entities::auto_version::AutoVersionExtractFromRule;
use crate::entities::environment::BuildEnvironment;
use crate::entities::info::ContentInfo;
use crate::entities::traits::Execute;
use crate::cmd::{BuildArgs, CleanArgs};
use crate::configs::DeployerProjectOptions;
use crate::i18n;
use crate::pipelines::DescribedPipeline;
use crate::rw::{copy_all, write, symlink, log, generate_build_log_filepath, build_log};
use crate::storage::{use_from_storage, add_to_storage};
use crate::utils::get_current_working_dir;

/// Список всех билдов в системе
#[derive(Deserialize, Serialize, Default)]
pub(crate) struct Builds {
  pub(crate) projects: Vec<ProjectBuilds>,
}

/// Билды определённого проекта
#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct ProjectBuilds {
  pub(crate) name: String,
  pub(crate) builds: Vec<BuildStats>,
}

/// Информация о сборке
#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct BuildStats {
  /// Этот билд будет использоваться только для определённых пайплайнов
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) exclusive_tag: Option<String>,
  /// Путь сборки
  pub(crate) folder: PathBuf,
}

impl BuildStats {
  pub(crate) fn works_with(&self, pipeline: &DescribedPipeline) -> bool {
    self.exclusive_tag.as_ref().is_some_and(|a| pipeline.exclusive_exec_tag.as_ref().is_some_and(|b| a.as_str().eq(b.as_str()))) ||
    (self.exclusive_tag.is_none() && pipeline.exclusive_exec_tag.is_none())
  }
}

pub(crate) fn enplace_artifacts(
  config: &DeployerProjectOptions,
  env: BuildEnvironment,
  panic_when_not_found: bool,
) -> anyhow::Result<()> {
  let mut ignore = vec![PathBuf::from(ARTIFACTS_DIR)];
  ignore.extend(config.cache_files.iter().cloned());
  
  for (from, to) in &config.inplace_artifacts_into_project_root {
    let artifact_path = env.build_dir.join(from);
    if !std::fs::exists(artifact_path.clone())? {
      if panic_when_not_found { panic!("{}: {:?}!", i18n::ARTIFACT_ENPLACE_FAIL, artifact_path); }
    } else if artifact_path.as_path().is_dir() || artifact_path.as_path().is_file() {
      copy_all(artifact_path.as_path(), env.artifacts_dir.join(to).as_path(), &ignore)?;
    }
  }
  
  Ok(())
}

fn prepare_artifacts_folder(
  current_dir: &std::path::Path,
) -> anyhow::Result<PathBuf> {
  let artifacts_dir = current_dir.join(ARTIFACTS_DIR);
  std::fs::create_dir_all(artifacts_dir.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", artifacts_dir));
  
  Ok(artifacts_dir)
}

fn prepare_build_folder(
  config: &DeployerProjectOptions,
  builds: &mut Builds,
  selected_pipeline: &DescribedPipeline,
  current_dir: &std::path::Path,
  cache_dir: &Path,
  args: &BuildArgs,
) -> anyhow::Result<(PathBuf, bool)> {
  let build_path = if let Some(build_at) = args.build_at.as_ref() {
    build_at.to_owned()
  } else {
    let mut build_path = PathBuf::new();
    build_path.push(cache_dir);
    build_path.push(CACHE_DIR);
    
    let mut project_builds = match builds.projects.iter().position(|p| p.name.as_str().eq(config.project_name.as_str())) {
      None => ProjectBuilds { name: config.project_name.to_owned(), builds: vec![] },
      Some(project_builds) => {
        let copy = builds.projects.get(project_builds).unwrap().clone();
        builds.projects.remove(project_builds);
        copy
      },
    };
    
    let folder = match project_builds.builds.iter().rev().find(|b| b.works_with(selected_pipeline)) {
      Some(b_stats) if !args.fresh => {
        b_stats.folder.to_owned()
      },
      _ => {
        let uuid = format!("deploy-build-{}", Uuid::new_v4());
        let folder = build_path.join(uuid);
        let b_stats = BuildStats { exclusive_tag: selected_pipeline.exclusive_exec_tag.clone(), folder: folder.to_owned(), };
        project_builds.builds.push(b_stats);
        folder.to_owned()
      },
    };
    
    builds.projects.push(project_builds);
    
    folder
  };
  
  let fresh = !build_path.exists() || args.fresh;
  std::fs::create_dir_all(build_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", build_path));
  
  let mut ignore = vec![PathBuf::from(ARTIFACTS_DIR), PathBuf::from(build_path.file_name().unwrap())];
  ignore.extend(config.cache_files.iter().cloned());
  
  copy_all(get_current_working_dir().unwrap(), build_path.as_path(), &ignore)?;
  write(cache_dir, BUILD_CACHE_LIST, &builds);
  
  if args.link_cache {
    for cache_item in &config.cache_files {
      symlink(current_dir.join(cache_item), build_path.join(cache_item));
      log(format!("-> {:?}", cache_item));
    }
  }
  
  if args.copy_cache {
    for cache_item in &config.cache_files {
      copy_all(
        current_dir.join(cache_item),
        build_path.join(cache_item),
        &[""]
      )?;
      log(format!("-> {:?}", cache_item));
    }
  }
  
  Ok((build_path, fresh))
}

pub(crate) fn build(
  config: &mut DeployerProjectOptions,
  builds: &mut Builds,
  cache_dir: &Path,
  storage_dir: &Path,
  args: &BuildArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() { panic!("{}", i18n::CFG_INVALID); }
  
  if args.link_cache && args.copy_cache { panic!(
    "Select only one option from `{}` and `{}`. See help via `{}`.", "c".green(), "C".green(), "deployer build -h".green()
  ); }
  if (args.fresh || args.link_cache || args.copy_cache || args.build_at.is_some()) && args.current { panic!(
    "Select either `{}` or `{}`/{}`/`{}`/`{}` options. See help via `{}`.",
    "o".green(),
    "j".green(),
    "f".green(),
    "c".green(),
    "C".green(),
    "deployer build -h".green(),
  ); }
  if args.silent && args.no_pipe { panic!(
    "Select only one option from `{}` and `{}`. See help via `{}`.", "s".green(), "t".green(), "deployer build -h".green()
  ); }

  let curr_dir = std::env::current_dir().expect("Can't get current dir!");
  let artifacts_dir = prepare_artifacts_folder(&curr_dir)?;

  if args.pipeline_tags.is_empty() {
    if config.pipelines.is_empty() {
      panic!("The pipelines' list is empty! Check the config file for errors.");
    }

    let cntr = config.pipelines.iter().filter(|p| p.default.is_some_and(|v| v)).count();
    if cntr == 0 { panic!("There is no default Pipelines! Please, specify at least one to execute."); }

    for pipeline in config.pipelines.iter().filter(|p| p.default.is_some_and(|v| v)) {
      let (build_path, new_build) = if args.current {
        (curr_dir.clone(), false)
      } else {
        prepare_build_folder(config, builds, pipeline, &curr_dir, cache_dir, args)?
      };

      let env = BuildEnvironment {
        build_dir: &build_path,
        cache_dir,
        storage_dir,
        artifacts_dir: &artifacts_dir,
        new_build,
        silent_build: args.silent,
        no_pipe: args.no_pipe,
      };
      
      execute_pipeline(config, env, pipeline)?;
      
      enplace_artifacts(config, env, true)?;
    }
  } else {
    for pipeline_tag in &args.pipeline_tags {
      if let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
        let (build_path, new_build) = if args.current {
          (curr_dir.clone(), false)
        } else {
          prepare_build_folder(config, builds, pipeline, &curr_dir, cache_dir, args)?
        };
        
        let env = BuildEnvironment {
          build_dir: &build_path,
          cache_dir,
          storage_dir,
          artifacts_dir: &artifacts_dir,
          new_build,
          silent_build: args.silent,
          no_pipe: args.no_pipe,
        };
        
        execute_pipeline(config, env, pipeline)?;
        
        enplace_artifacts(config, env, true)?;
      } else {
        panic!(
          "There is no such Pipeline `{}` set up for this project. Maybe, you've forgotten set up this Pipeline for project via `{}`?",
          pipeline_tag.green(),
          "deployer with {pipeline-short-name-and-ver}".green(),
        );
      }
    }
  }
  
  Ok(())
}

pub(crate) fn execute_pipeline(
  config: &DeployerProjectOptions,
  env: BuildEnvironment,
  pipeline: &DescribedPipeline,
) -> anyhow::Result<()> {
  use std::io::{stdout, Write};
  use std::time::Instant;
  
  let log_file = generate_build_log_filepath(
    &config.project_name,
    &pipeline.title,
    env.cache_dir,
  );
  
  if !env.silent_build { println!("{}", i18n::STARTING_PIPELINE.replace("{}", &pipeline.title)); }
  build_log(&log_file, &[format!("Starting the `{}` Pipeline...", pipeline.title)])?;
  
  let canonicalized = env.build_dir.canonicalize()?;
  let canonicalized = canonicalized.to_str().expect("Can't convert `Path` to string!");
  if !env.silent_build { println!("{}: {}", i18n::BUILD_PATH, canonicalized); }
  build_log(&log_file, &[format!("{}: {}", i18n::BUILD_PATH, canonicalized)])?;
  
  let mut cntr = 1usize;
  let total = pipeline.actions.len();
  for action in &pipeline.actions {
    if !env.silent_build {
      if !env.no_pipe {
        print!("[{}/{}] {} `{}`...", cntr, total, i18n::STARTING_ACTION, action.title.blue().italic());
      } else {
        println!("[{}/{}] {} `{}`...", cntr, total, i18n::STARTING_ACTION, action.title.blue().italic());
      }
      build_log(&log_file, &[format!("[{}/{}] {} `{}`...", cntr, total, i18n::STARTING_ACTION, action.title)])?;
    }
    stdout().flush()?;
    let now = Instant::now();
    
    let (status, output) = match &action.action {
      Action::Custom(cmd) => cmd.execute(env)?,
      Action::Check(check) => check.execute(env)?,
      Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => a.execute(env)?,
      Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => a.execute(env)?,
      Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => a.execute(env)?,
      Action::Observe(o_action) => o_action.execute(env)?,
      Action::Patch(patch) => patch.execute(env)?,
      Action::ForceArtifactsEnplace => {
        enplace_artifacts(config, env, false)?;
        
        let mut modified_env = env;
        let artifacts_dir = modified_env.build_dir.to_path_buf().join(ARTIFACTS_DIR);
        modified_env.artifacts_dir = &artifacts_dir;
        enplace_artifacts(config, modified_env, false)?;
        
        (true, vec![i18n::ARTIFACTS_ENPLACED.into()])
      },
      Action::Interrupt => {
        println!();
        inquire::Confirm::new(i18n::INTERRUPT).with_default(true).prompt()?;
        (true, vec![])
      },
      Action::UseFromStorage(content_info) => {
        match use_from_storage(env.storage_dir, env.build_dir, content_info) {
          Ok(_) => (true, vec![]),
          Err(e) => (false, vec![e.to_string()]),
        }
      },
      Action::AddToStorage(rules) => {
        match &rules.auto_version_rule {
          AutoVersionExtractFromRule::CmdStdout(cmd) => {
            let (succ, out) = cmd.execute(env)?;
            if !succ || out.is_empty() { (false, out) }
            else {
              let version = out.last().unwrap().trim().to_owned();
              let info = ContentInfo::new(rules.short_name.as_str(), version.as_str())?;
              if let Err(e) = add_to_storage(env.storage_dir, env.artifacts_dir, &info) { (false, vec![e.to_string()]) }
              else { (true, vec![]) }
            }
          },
          AutoVersionExtractFromRule::PlainFile(path) => {
            let mut file = std::fs::File::open(path)?;
            let version = { let mut ver = String::new(); file.read_to_string(&mut ver)?; ver.trim().to_string() };
            let info = ContentInfo::new(rules.short_name.as_str(), version.as_str())?;
            if let Err(e) = add_to_storage(env.storage_dir, env.artifacts_dir, &info) { (false, vec![e.to_string()]) }
            else { (true, vec![]) }
          },
        }
      },
    };
    
    let status_str = match status {
      true => i18n::DONE.to_string(),
      false => i18n::GOT_ERROR.red().bold().to_string(),
    };
    
    let elapsed = now.elapsed();
    if !env.no_pipe { build_log(&log_file, &output)?; }
    build_log(&log_file, &[
      format!(
        "[{}/{}] {} -{} ({:.2?}).",
        cntr,
        total,
        i18n::STARTING_ACTION.replace("{}", &action.title),
        if status { i18n::DONE } else { i18n::GOT_ERROR },
        elapsed,
      ),
    ])?;
    
    if !env.silent_build {
      if !env.no_pipe {
        println!("{} ({}).", status_str, format!("{:.2?}", elapsed).green());
        for line in output { println!("{}", line); }
      } else {
        println!("[{}/{}] {} -{} ({}).", cntr, total, i18n::STARTING_ACTION.replace("{}", &action.title.blue().italic()), status_str, format!("{:.2?}", elapsed).green());
      }
    }
    
    cntr += 1;
    
    if !status { return Ok(()) }
  }
  
  Ok(())
}

pub(crate) fn clean_builds(
  config: &DeployerProjectOptions,
  builds: &mut Builds,
  cache_dir: &Path,
  args: &CleanArgs,
) -> anyhow::Result<()> {
  let mut path = PathBuf::new();
  path.push(cache_dir);
  path.push(CACHE_DIR);
  
  let mut total: u64 = 0;
  
  if let Some(project_builds) = builds.projects.iter_mut().find(|p| p.name.as_str().eq(config.project_name.as_str())) {
    for folder in project_builds.builds.iter().map(|b| b.folder.clone()) {
      total += get_size(&folder)?;
      let _ = std::fs::remove_dir_all(folder);
    }
    project_builds.builds.clear();
  }
  
  if args.include_artifacts {
    let curr_dir = std::env::current_dir()?;
    let artifacts_dir = curr_dir.join(ARTIFACTS_DIR);
    if artifacts_dir.as_path().exists() {
      total += get_size(&artifacts_dir)?;
      let _ = std::fs::remove_dir_all(artifacts_dir);
    }
  }
  
  println!("{}: {}", i18n::CLEANED, format_size(total));
  
  Ok(())
}

fn format_size(size: u64) -> String {
  const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
  let mut size = size as f64;
  let mut unit_index = 0;

  while size >= 1024.0 && unit_index < UNITS.len() - 1 {
    size /= 1024.0;
    unit_index += 1;
  }

  if unit_index == 0 { format!("{} {}", size as u64, UNITS[unit_index]) }
  else { format!("{:.1} {}", size, UNITS[unit_index]) }
}
