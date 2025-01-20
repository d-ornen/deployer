//! Build module.
//! 
//! Deployer's build process both complicated and flexible enough.
//! The main functions is `build` and `execute_pipeline`.

use anyhow::bail;
use colored::Colorize;
use fs_extra::dir::get_size;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::exit;
use uuid::Uuid;

use crate::actions::Action;
use crate::{CACHE_DIR, ARTIFACTS_DIR, BUILD_CACHE_LIST};
use crate::entities::auto_version::AutoVersionExtractFromRule;
use crate::entities::environment::BuildEnvironment;
use crate::entities::info::{ContentInfo, ShortName};
use crate::entities::remote_host::RemoteHost;
use crate::entities::requirements::{Requirement, Satisfy, SatisfyErr};
use crate::entities::traits::Execute;
use crate::cmd::{BuildArgs, CleanArgs};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::i18n;
use crate::pipelines::DescribedPipeline;
use crate::remote::{sync_to_remote, sync_from_remote, sync_artifacts_from_remote};
use crate::rw::{copy_all, write, symlink, log, generate_build_log_filepath, build_log};
use crate::storage::{use_from_storage, add_to_storage};
use crate::utils::get_current_working_dir;

/// List of all builds at this host.
#[derive(Deserialize, Serialize, Default)]
pub struct Builds {
  pub projects: Vec<ProjectBuilds>,
}

/// Builds of chosen project.
#[derive(Deserialize, Serialize, Clone)]
pub struct ProjectBuilds {
  /// Project name (see `DeployerProjectOptions::project_name`).
  pub name: String,
  /// List of build folders.
  pub builds: Vec<Build>,
}

/// Build information.
#[derive(Deserialize, Serialize, Clone)]
pub struct Build {
  /// If is set, this folder will be used only for Pipelines with this exclusive tag.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exclusive_tag: Option<String>,
  /// Build path.
  pub folder: PathBuf,
}

impl Build {
  /// Checks the exclusive tag on both build folder and given Pipeline.
  pub fn works_with(&self, pipeline: &DescribedPipeline) -> bool {
    self.exclusive_tag.as_ref().is_some_and(|a| pipeline.exclusive_exec_tag.as_ref().is_some_and(|b| a.as_str().eq(b.as_str()))) ||
    (self.exclusive_tag.is_none() && pipeline.exclusive_exec_tag.is_none())
  }
}

/// Places Pipeline artifacts in the `artifacts` folder inside project's directory.
/// 
/// `panic_when_not_found` is set to `false` on all function's usages now.
pub fn place_artifacts(
  config: &DeployerProjectOptions,
  env: BuildEnvironment,
  panic_when_not_found: bool,
) -> anyhow::Result<()> {
  let mut ignore = vec![PathBuf::from(ARTIFACTS_DIR)];
  ignore.extend(config.cache_files.iter().cloned());
  
  for (from, to) in &config.place_artifacts_into_project_root {
    let artifact_path = env.build_dir.join(from);
    if !std::fs::exists(artifact_path.clone())? {
      if panic_when_not_found { panic!("{}: {:?}!", i18n::ARTIFACT_ENPLACE_FAIL, artifact_path); }
    } else if artifact_path.as_path().is_dir() || artifact_path.as_path().is_file() {
      copy_all(artifact_path.as_path(), env.artifacts_dir.join(to).as_path(), &ignore)?;
    }
  }
  
  Ok(())
}

/// Creates the artifacts folder, if it isn't exist.
fn prepare_artifacts_folder(
  current_dir: &std::path::Path,
) -> anyhow::Result<PathBuf> {
  let artifacts_dir = current_dir.join(ARTIFACTS_DIR);
  std::fs::create_dir_all(artifacts_dir.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", artifacts_dir));
  
  Ok(artifacts_dir)
}

/// Gets a compatible or creates a new build folder to perform Pipeline Actions.
/// 
/// You can specify next `deployer build` command flags:
/// 
/// - `build_at` to specify concrete build folder
/// - `fresh` to create new build folder
/// - `copy_cache` to copy cache files from project's folder
/// - `link_cache` to create symlinks to cache files from project's folder
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
        let b_stats = Build { exclusive_tag: selected_pipeline.exclusive_exec_tag.clone(), folder: folder.to_owned(), };
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

/// Decides where and how to build the project.
/// 
/// You can specify next `deployer build` command flags:
/// 
/// - `remote_build_folder` to run Deployer as worker node
/// - `remote_host_short_names` to run Deployer as controller
/// - `current` to build at project's folder instead of build folder
/// - `silent` to build without any out to display
/// - `no_pipe` to build without I/O redirection from Actions' commands
/// - `build_at` to specify concrete build folder
/// - `fresh` to create new build folder
/// - `copy_cache` to copy cache files from project's folder
/// - `link_cache` to create symlinks to cache files from project's folder
pub fn build(
  config: &mut DeployerProjectOptions,
  globals: &DeployerGlobalConfig,
  builds: &mut Builds,
  cache_dir: &Path,
  config_dir: &Path,
  storage_dir: &Path,
  args: &BuildArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() && args.remote_build_folder.is_none() { panic!("{}", i18n::CFG_INVALID); }
  check_args_on_conflicts(args)?;
  
  if let Some(build_dir) = &args.remote_build_folder && !args.pipeline_tags.is_empty() {
    return build_as_worker(build_dir, cache_dir, config_dir, storage_dir, &globals.remote_hosts, args)
  }
  
  let curr_dir = std::env::current_dir().expect("Can't get current dir!");
  let artifacts_dir = prepare_artifacts_folder(&curr_dir)?;
  
  if !args.remote_host_short_names.is_empty() {
    return build_as_controller(config, builds, &artifacts_dir, &curr_dir, cache_dir, &globals.remote_hosts, args)
  }

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
        config_dir,
        storage_dir,
        project_dir: Some(&curr_dir),
        artifacts_dir: &artifacts_dir,
        new_build,
        silent_build: args.silent,
        no_pipe: args.no_pipe,
        ignore: &config.cache_files,
        remotes: &globals.remote_hosts,
      };
      
      execute_pipeline(config, env, pipeline)?;
      place_artifacts(config, env, false)?;
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
          config_dir,
          storage_dir,
          project_dir: Some(&curr_dir),
          artifacts_dir: &artifacts_dir,
          new_build,
          silent_build: args.silent,
          no_pipe: args.no_pipe,
          ignore: &config.cache_files,
          remotes: &globals.remote_hosts,
        };
        
        execute_pipeline(config, env, pipeline)?;
        place_artifacts(config, env, false)?;
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

/// Builds project as worker node (e.g., without project directory itself
/// and with given build folder path from controller's node).
pub fn build_as_worker(
  build_dir: &Path,
  cache_dir: &Path,
  config_dir: &Path,
  storage_dir: &Path,
  remotes: &HashMap<ShortName, RemoteHost>,
  args: &BuildArgs,
) -> anyhow::Result<()> {
  let config = crate::rw::read::<DeployerProjectOptions>(build_dir, crate::PROJECT_CONF);
  let artifacts_dir = prepare_artifacts_folder(build_dir)?;
  
  for pipeline_tag in &args.pipeline_tags {
    if let Some(pipeline) = &config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
      let env = BuildEnvironment {
        build_dir,
        cache_dir,
        config_dir,
        storage_dir,
        project_dir: None,
        artifacts_dir: &artifacts_dir,
        new_build: true,
        silent_build: false,
        no_pipe: false,
        ignore: &config.cache_files,
        remotes,
      };
      
      execute_pipeline(&config, env, pipeline)?;
      place_artifacts(&config, env, false)?;
    } else {
      panic!(
        "There is no such Pipeline `{}` set up for this project. Maybe, you've forgotten set up this Pipeline for project via `{}`?",
        pipeline_tag.green(),
        "deployer with {pipeline-short-name-and-ver}".green(),
      );
    }
  }
  
  Ok(())
}

/// Builds project as controller (e.g., sends build folder to worker node
/// and starts Deployer on this node remotely).
/// 
/// After remote build Deployer automatically copies all built artifacts
/// to this host from the remote.
pub fn build_as_controller(
  config: &DeployerProjectOptions,
  builds: &mut Builds,
  artifacts_dir: &Path,
  current_dir: &Path,
  cache_dir: &Path,
  remotes: &HashMap<ShortName, RemoteHost>,
  args: &BuildArgs,
) -> anyhow::Result<()> {
  let mut remote = vec![];
  for short_name in &args.remote_host_short_names {
    if let Some(host) = remotes.get(&ShortName::new(short_name)?) {
      remote.push(host.clone());
    }
  }
  if remote.is_empty() { bail!(i18n::NO_SUCH_HOSTS); }
  
  for pipeline_tag in &args.pipeline_tags {
    if let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
      let (build_path, _) = prepare_build_folder(config, builds, pipeline, current_dir, cache_dir, args)?;
      
      for host in remote.iter() {
        if !args.silent { println!("{} `{}`...", i18n::START_BUILD_AT_REMOTE, host.short_name.as_str().green()); }
        let now = std::time::Instant::now();
        let generated_remote = sync_to_remote(&build_path, host, &config.cache_files)?;
        match host.call_deployer_to_build(&generated_remote, pipeline.title.as_str()) {
          Err(e) => println!("{}", e),
          Ok((status, out)) => {
            if !args.silent { for line in out { println!("{}", line) } }
            if !status { exit(1); }
          }
        }
        sync_artifacts_from_remote(&generated_remote, artifacts_dir, host)?;
        if !args.silent {
          println!("{} `{}` ({}).", i18n::BUILT_AT_REMOTE, host.short_name.as_str().green(), format!("{:.2?}", now.elapsed()).green());
        }
      }
    } else {
      panic!(
        "There is no such Pipeline `{}` set up for this project. Maybe, you've forgotten set up this Pipeline for project via `{}`?",
        pipeline_tag.green(),
        "deployer with {pipeline-short-name-and-ver}".green(),
      );
    }
  }
  
  Ok(())
}

/// Performs specified Pipeline with the project.
/// 
/// Pipeline execution steps:
/// 
/// 1. Prepare build log file.
/// 2. Collect all Pipeline Actions' requirements and satisfy them.
/// 3. Execute all Pipeline's Actions.
pub fn execute_pipeline(
  config: &DeployerProjectOptions,
  env: BuildEnvironment,
  pipeline: &DescribedPipeline,
) -> anyhow::Result<()> {
  use std::io::{stdout, Write};
  use std::time::{Instant, Duration};
  
  let mut total_time = Duration::from_secs(0);
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
  
  let now = Instant::now();
  
  #[allow(clippy::mutable_key_type)]
  let mut requirements = HashSet::<Requirement>::new();
  for action in &pipeline.actions {
    if let Some(action_reqs) = &action.requirements {
      for action_req in action_reqs { requirements.insert(action_req.clone()); }
    }
  }
  if let Err(e) = requirements.satisfy(env) {
    match e {
      SatisfyErr::Exists(path) => println!("{}", i18n::REQ_NOT_SATISFIED.replace("{}", &path.to_string_lossy())),
      SatisfyErr::ExistsAny(paths) => {
        let paths = paths.iter().map(|p| p.to_string_lossy().as_str().to_string()).collect::<Vec<_>>();
        println!("{}", i18n::REQ_NOT_SATISFIED.replace("{}", &paths.join("`, `")))
      },
      SatisfyErr::Check(output) => {
        println!("{}", i18n::REQ_CMD_NOT_SATISFIED);
        for line in output { println!("{}", line); }
      }
      SatisfyErr::Remote(e) => println!("{}", e),
    }
    return Ok(())
  }
  
  if !requirements.is_empty() {
    let elapsed = now.elapsed();
    println!("{} {}.", i18n::REQ_CHECKS_TOOK, format!("{:.2?}", elapsed).green());
    build_log(&log_file, &[format!("{} {:.2?}.", i18n::REQ_CHECKS_TOOK, elapsed)])?;
    total_time += elapsed;
  }
  
  let mut cntr = 1usize;
  let total = pipeline.actions.len();
  for action in &pipeline.actions {
    let env = if action.exec_in_project_dir.is_some_and(|v| v) && let Some(project_dir) = env.project_dir {
      BuildEnvironment {
        build_dir: project_dir,
        ..env
      }
    } else { env };
    
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
      Action::SyncToRemote(remote_name) => {
        if let Some(remote) = env.remotes.get(remote_name) {
          if let Err(e) = sync_to_remote(env.build_dir, remote, env.ignore) { (false, vec![e.to_string()]) } else { (true, vec![]) }
        } else {
          (false, vec![i18n::NO_SUCH_REMOTE.to_string()])
        }
      },
      Action::SyncFromRemote(remote_name) => {
        if let Some(remote) = env.remotes.get(remote_name) {
          if let Err(e) = sync_from_remote(env.build_dir, remote) { (false, vec![e.to_string()]) } else { (true, vec![]) }
        } else {
          (false, vec![i18n::NO_SUCH_REMOTE.to_string()])
        }
      },
      Action::Custom(cmd) => cmd.execute(env)?,
      Action::Check(check) => check.execute(env)?,
      Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) | Action::Test(a) => a.execute(env)?,
      Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => a.execute(env)?,
      Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => a.execute(env)?,
      Action::Observe(o_action) => o_action.execute(env)?,
      Action::Patch(patch) => patch.execute(env)?,
      Action::Interrupt => {
        println!();
        #[cfg(feature = "tui")]
        inquire::Confirm::new(i18n::INTERRUPT).with_default(true).prompt()?;
        #[cfg(not(feature = "tui"))]
        {
          let _ = std::io::read_to_string(std::io::stdin())?;
        }
        (true, vec![])
      },
      Action::UseFromStorage(content_info) => {
        match use_from_storage(env.storage_dir, env.build_dir, content_info) {
          Ok(_) => (true, vec![]),
          Err(e) => (false, vec![e.to_string()]),
        }
      },
      Action::AddToStorage(rules) => {
        place_artifacts(config, env, false)?;
        
        match &rules.auto_version_rule {
          AutoVersionExtractFromRule::CmdStdout(cmd) => {
            let (succ, out) = cmd.execute(env)?;
            if !succ || out.is_empty() { (false, out) }
            else {
              let version = out.last().unwrap().trim().replace(">>> ", "");
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
    total_time += elapsed;
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
        for line in &output { println!("{}", line); }
      } else {
        println!("[{}/{}] {} -{} ({}).", cntr, total, i18n::STARTING_ACTION.replace("{}", &action.title.blue().italic()), status_str, format!("{:.2?}", elapsed).green());
      }
    }
    
    cntr += 1;
    
    if !status { exit(1) }
  }
  
  println!("{} {}.", i18n::DONE_IN, format!("{:.2?}", total_time).green());
  build_log(&log_file, &[format!("{} {:.2?}.", i18n::DONE_IN, total_time)])?;
  
  Ok(())
}

/// Cleans all project builds.
/// 
/// You can also specify `include_artifacts` option (`deployer clean -i`)
/// to cleanup `artifacts` folder.
pub fn clean_builds(
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

fn check_args_on_conflicts(args: &BuildArgs) -> anyhow::Result<()> {
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
  if args.remote_build_folder.is_some() && args.pipeline_tags.is_empty() { panic!(
    "You always should specify Pipeline tags for executing while remote builds."
  ) }
  if !args.remote_host_short_names.is_empty() && (
    args.remote_build_folder.is_some() ||
    args.pipeline_tags.is_empty() ||
    args.link_cache ||
    args.copy_cache ||
    args.fresh ||
    args.build_at.is_some() ||
    args.current
  ) { panic!(
    "If you specify remote hosts to build on, you should specify only Pipelines list, `{}`/`{}` flags and nothing more.",
    "silent".green(),
    "no_pipe".green(),
  ) }
  
  Ok(())
}

/// Formats `u64` as file size (bytes).
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
