//! Run module.
//!
//! Deployer's run process both complicated and flexible enough.
//! The main functions is `run` and `execute_pipeline`.

use anyhow::bail;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::exit;
use uuid::Uuid;

use crate::actions::Action;
use crate::cmd::RunArgs;
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions, Placement};
#[cfg(feature = "containered")]
use crate::containered::execute_pipeline_containered;
use crate::entities::daemons::Daemons;
use crate::entities::environment::RunEnvironment;
use crate::entities::info::ShortName;
use crate::entities::remote_host::RemoteHost;
use crate::entities::requirements::{Satisfy, SatisfyErr};
use crate::entities::traits::{Execute, Merge};
use crate::i18n;
use crate::pipelines::DescribedPipeline;
use crate::remote::{sync_artifacts_from_remote, sync_from_remote, sync_to_remote};
use crate::rw::{build_log, copy_all, generate_build_log_filepath, log, symlink, write};
use crate::storage::use_from_storage;
use crate::utils::get_current_working_dir;
use crate::{ARTIFACTS_DIR, BUILD_CACHE_LIST, CACHE_DIR};
use crate::{hmap, hset};

/// List of all Pipelines runs at this host.
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Runs {
  pub projects: Vec<ProjectRuns>,
}

impl Merge for Runs {
  fn merge(&self, other: Self) -> anyhow::Result<Self> {
    let mut projects = hmap!();
    for p in &self.projects {
      projects.insert(p.name.to_owned(), vec![]);
    }
    for p in &other.projects {
      projects.insert(p.name.to_owned(), vec![]);
    }

    for p in &other.projects {
      let project = projects.get_mut(&p.name).unwrap();
      for run in &p.runs {
        if !project.contains(&run) {
          project.push(run)
        };
      }
    }
    for p in &self.projects {
      if other
        .projects
        .iter()
        .find(|v| v.name.as_str().eq(p.name.as_str()))
        .is_some_and(|v| v.runs.is_empty())
      {
        // Considering that project is cleared, continuing
        continue;
      }
      let project = projects.get_mut(&p.name).unwrap();
      for run in &p.runs {
        if !project.contains(&run) {
          project.push(run)
        };
      }
    }

    let mut runs = Runs { projects: vec![] };
    projects.iter().for_each(|(k, v)| {
      let pr_runs = v.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>();
      runs.projects.push(ProjectRuns {
        name: k.to_owned(),
        runs: pr_runs,
      });
    });

    Ok(runs)
  }
}

/// Runs of chosen project.
#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct ProjectRuns {
  /// Project name (see `DeployerProjectOptions::project_name`).
  pub name: String,
  /// List of run folders.
  pub runs: Vec<Run>,
}

/// Run information.
#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct Run {
  /// If is set, this folder will be used only for Pipelines with this exclusive tag.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exclusive_tag: Option<String>,
  /// Build path.
  pub folder: PathBuf,
}

/// Places Pipeline artifacts in the `artifacts` folder inside project's directory.
///
/// `panic_when_not_found` is set to `false` on all function's usages now.
pub fn place_artifacts(
  config: &DeployerProjectOptions,
  env: &RunEnvironment,
  panic_when_not_found: bool,
) -> anyhow::Result<()> {
  let mut ignore = vec![PathBuf::from(ARTIFACTS_DIR)];
  ignore.extend(config.cache_files.iter().cloned());

  for Placement { from, to } in &config.place_artifacts_into_project_root {
    let artifact_path = env.run_dir.join(from);
    if !std::fs::exists(artifact_path.clone())? {
      if panic_when_not_found {
        panic!("{}: {:?}!", i18n::ARTIFACT_ENPLACE_FAIL, artifact_path);
      }
    } else if artifact_path.as_path().is_dir() || artifact_path.as_path().is_file() {
      copy_all(
        artifact_path.as_path(),
        artifact_path.as_path(),
        env.artifacts_dir.join(to).as_path(),
        &ignore,
      )?;
    }
  }

  Ok(())
}

/// Creates the artifacts folder, if it isn't exist.
pub(crate) fn prepare_artifacts_folder(current_dir: &std::path::Path) -> anyhow::Result<PathBuf> {
  let artifacts_dir = current_dir.join(ARTIFACTS_DIR);
  std::fs::create_dir_all(artifacts_dir.as_path())
    .unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", artifacts_dir));

  Ok(artifacts_dir)
}

/// Gets a compatible or creates a new run folder to perform Pipeline Actions.
///
/// You can specify next `deployer run` command flags:
///
/// - `run_at` to specify concrete run folder
/// - `fresh` to create new run folder
/// - `copy_cache` to copy cache files from project's folder
/// - `link_cache` to create symlinks to cache files from project's folder
pub(crate) fn prepare_run_folder(
  config: &DeployerProjectOptions,
  runs: &mut Runs,
  exclusive_exec_tag: &Option<String>,
  current_dir: &std::path::Path,
  cache_dir: &Path,
  args: &RunArgs,
) -> anyhow::Result<(PathBuf, bool)> {
  let run_path = if let Some(run_at) = args.run_at.as_ref() {
    run_at.to_owned()
  } else {
    let mut run_path = PathBuf::new();
    run_path.push(cache_dir);
    run_path.push(CACHE_DIR);

    let mut project_runs = match runs
      .projects
      .iter()
      .position(|p| p.name.as_str().eq(config.project_name.as_str()))
    {
      None => ProjectRuns {
        name: config.project_name.to_owned(),
        runs: vec![],
      },
      Some(project_runs) => {
        let copy = runs.projects.get(project_runs).unwrap().clone();
        runs.projects.remove(project_runs);
        copy
      }
    };

    let folder = match project_runs.runs.iter().rev().find(|b| {
      b.exclusive_tag
        .as_ref()
        .is_some_and(|a| exclusive_exec_tag.as_ref().is_some_and(|b| a.as_str().eq(b.as_str())))
        || (b.exclusive_tag.is_none() && exclusive_exec_tag.is_none())
    }) {
      Some(b_stats) if !args.fresh => b_stats.folder.to_owned(),
      _ => {
        let uuid = format!("deploy-build-{}", Uuid::new_v4());
        let folder = run_path.join(uuid);
        let b_stats = Run {
          exclusive_tag: exclusive_exec_tag.clone(),
          folder: folder.to_owned(),
        };
        project_runs.runs.push(b_stats);
        folder.to_owned()
      }
    };

    runs.projects.push(project_runs);

    folder
  };

  let fresh = !run_path.exists() || args.fresh;
  std::fs::create_dir_all(run_path.as_path()).unwrap_or_else(|_| panic!("Can't create `{:?}` folder!", run_path));

  let mut ignore = vec![
    PathBuf::from(ARTIFACTS_DIR),
    PathBuf::from(run_path.file_name().unwrap()),
  ];
  ignore.extend(config.cache_files.iter().cloned());

  let cwd = get_current_working_dir().unwrap();
  copy_all(&cwd, &cwd, run_path.as_path(), &ignore)?;
  write(cache_dir, BUILD_CACHE_LIST, &runs);

  if args.link_cache {
    for cache_item in &config.cache_files {
      symlink(current_dir.join(cache_item), run_path.join(cache_item));
      log(format!("-> {:?}", cache_item));
    }
  }

  if args.copy_cache {
    for cache_item in &config.cache_files {
      let cache_item_path = current_dir.join(cache_item);
      copy_all(&cache_item_path, &cache_item_path, run_path.join(cache_item), &[""])?;
      log(format!("-> {:?}", cache_item));
    }
  }

  Ok((run_path, fresh))
}

/// Decides where and how to run the project.
///
/// You can specify next `deployer run` command flags:
///
/// - `remote_folder` to run Deployer as worker node
/// - `remote_host_short_names` to run Deployer as controller
/// - `current` to run at project's folder instead of run folder
/// - `silent` to run without any out to display
/// - `no_pipe` to run without I/O redirection from Actions' commands
/// - `run_at` to specify concrete run folder
/// - `fresh` to create new run folder
/// - `copy_cache` to copy cache files from project's folder
/// - `link_cache` to create symlinks to cache files from project's folder
pub fn run(
  config: &mut DeployerProjectOptions,
  globals: &DeployerGlobalConfig,
  runs: &mut Runs,
  cache_dir: &Path,
  config_dir: &Path,
  storage_dir: &Path,
  args: &RunArgs,
) -> anyhow::Result<()> {
  if *config == Default::default() && args.remote_folder.is_none() {
    panic!("{}", i18n::CFG_INVALID);
  }
  check_args_on_conflicts(args)?;

  if let Some(run_dir) = &args.remote_folder
    && !args.pipeline_tags.is_empty()
  {
    return run_as_worker(
      config,
      run_dir,
      cache_dir,
      config_dir,
      storage_dir,
      &globals.remote_hosts,
      args,
    );
  }

  let curr_dir = std::env::current_dir().expect("Can't get current dir!");
  let artifacts_dir = prepare_artifacts_folder(&curr_dir)?;

  if !args.remote_host_short_names.is_empty() {
    return run_as_controller(
      config,
      runs,
      &artifacts_dir,
      &curr_dir,
      cache_dir,
      &globals.remote_hosts,
      args,
    );
  }

  if args.pipeline_tags.is_empty() {
    if config.pipelines.is_empty() {
      panic!("The pipelines' list is empty! Check the config file for errors.");
    }

    let cntr = config.pipelines.iter().filter(|p| p.default.is_some_and(|v| v)).count();
    if cntr == 0 {
      panic!("There is no default Pipelines! Please, specify at least one to execute.");
    }

    for pipeline in config.pipelines.iter().filter(|p| p.default.is_some_and(|v| v)) {
      let (run_path, new_build) = if args.current {
        (curr_dir.clone(), false)
      } else {
        prepare_run_folder(config, runs, &pipeline.exclusive_exec_tag, &curr_dir, cache_dir, args)?
      };

      let env = RunEnvironment {
        run_dir: &run_path,
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
        #[cfg(feature = "containered")]
        containered: args.containered,
        daemons: Daemons::new(),
      };

      execute_pipeline(config, &env, pipeline)?;
      place_artifacts(config, &env, false)?;
    }
  } else {
    for pipeline_tag in &args.pipeline_tags {
      if let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
        let (run_path, new_build) = if args.current {
          (curr_dir.clone(), false)
        } else {
          prepare_run_folder(config, runs, &pipeline.exclusive_exec_tag, &curr_dir, cache_dir, args)?
        };

        let env = RunEnvironment {
          run_dir: &run_path,
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
          #[cfg(feature = "containered")]
          containered: args.containered,
          daemons: Daemons::new(),
        };

        execute_pipeline(config, &env, pipeline)?;
        place_artifacts(config, &env, false)?;
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

/// Runs project as worker node (e.g., without project directory itself
/// and with given run folder path from controller's node).
pub fn run_as_worker(
  config: &mut DeployerProjectOptions,
  run_dir: &Path,
  cache_dir: &Path,
  config_dir: &Path,
  storage_dir: &Path,
  remotes: &HashMap<ShortName, RemoteHost>,
  args: &RunArgs,
) -> anyhow::Result<()> {
  let artifacts_dir = prepare_artifacts_folder(run_dir)?;

  for pipeline_tag in &args.pipeline_tags {
    if let Some(pipeline) = &config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
      let env = RunEnvironment {
        run_dir,
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
        #[cfg(feature = "containered")]
        containered: args.containered,
        daemons: Daemons::new(),
      };

      execute_pipeline(config, &env, pipeline)?;
      place_artifacts(config, &env, false)?;
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

/// Runs project as controller (e.g., sends run folder to worker node
/// and starts Deployer on this node remotely).
///
/// After remote run Deployer automatically copies all built artifacts
/// to this host from the remote.
pub fn run_as_controller(
  config: &DeployerProjectOptions,
  runs: &mut Runs,
  artifacts_dir: &Path,
  current_dir: &Path,
  cache_dir: &Path,
  remotes: &HashMap<ShortName, RemoteHost>,
  args: &RunArgs,
) -> anyhow::Result<()> {
  let mut remote = vec![];
  for short_name in &args.remote_host_short_names {
    if let Some(host) = remotes.get(&ShortName::new(short_name)?) {
      remote.push(host.clone());
    }
  }
  if remote.is_empty() {
    bail!(i18n::NO_SUCH_HOSTS);
  }

  for pipeline_tag in &args.pipeline_tags {
    if let Some(pipeline) = config.pipelines.iter().find(|p| p.title.as_str().eq(pipeline_tag)) {
      let (build_path, _) =
        prepare_run_folder(config, runs, &pipeline.exclusive_exec_tag, current_dir, cache_dir, args)?;

      for host in remote.iter() {
        if !args.silent {
          println!(
            "{} `{}`...",
            i18n::START_BUILD_AT_REMOTE,
            host.short_name.as_str().green()
          );
        }
        let now = std::time::Instant::now();
        let generated_remote = sync_to_remote(&build_path, host, &config.cache_files)?;
        match host.call_deployer_to_build(&generated_remote, pipeline.title.as_str()) {
          Err(e) => println!("{}", e),
          Ok((status, out)) => {
            if !args.silent {
              for line in out {
                println!("{}", line)
              }
            }
            if !status {
              exit(1);
            }
          }
        }
        sync_artifacts_from_remote(&generated_remote, artifacts_dir, host)?;
        if !args.silent {
          println!(
            "{} `{}` ({}).",
            i18n::BUILT_AT_REMOTE,
            host.short_name.as_str().green(),
            format!("{:.2?}", now.elapsed()).green()
          );
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
  env: &RunEnvironment,
  pipeline: &DescribedPipeline,
) -> anyhow::Result<()> {
  use std::io::{Write, stdout};
  use std::time::{Duration, Instant};

  #[cfg(feature = "containered")]
  if pipeline.containered_opts.is_some() && !env.containered {
    let env = RunEnvironment {
      silent_build: false,
      daemons: env.daemons.clone(),
      ..(*env)
    };
    return execute_pipeline_containered(config, &env, pipeline);
  }

  let mut total_time = Duration::from_secs(0);
  let log_file = generate_build_log_filepath(&config.project_name, &pipeline.title, env.cache_dir);

  if !env.silent_build {
    println!("{}", i18n::STARTING_PIPELINE.replace("{}", &pipeline.title));
  }
  build_log(&log_file, &[format!("Starting the `{}` Pipeline...", pipeline.title)])?;

  let canonicalized = env.run_dir.canonicalize()?;
  let canonicalized = canonicalized.to_str().expect("Can't convert `Path` to string!");
  if !env.silent_build {
    println!("{}: {}", i18n::BUILD_PATH, canonicalized);
  }
  build_log(&log_file, &[format!("{}: {}", i18n::BUILD_PATH, canonicalized)])?;

  let now = Instant::now();

  #[allow(clippy::mutable_key_type)]
  let mut requirements = hset!();
  for action in &pipeline.actions {
    if let Some(action_reqs) = &action.requirements {
      for action_req in action_reqs {
        requirements.insert(action_req.clone());
      }
    }
  }
  if let Err(e) = requirements.satisfy(env) {
    match e {
      SatisfyErr::Exists(path) => println!("{}", i18n::REQ_NOT_SATISFIED.replace("{}", &path.to_string_lossy())),
      SatisfyErr::ExistsAny(paths) => {
        let paths = paths
          .iter()
          .map(|p| p.to_string_lossy().as_str().to_string())
          .collect::<Vec<_>>();
        println!("{}", i18n::REQ_NOT_SATISFIED.replace("{}", &paths.join("`, `")))
      }
      SatisfyErr::Check(output) => {
        println!("{}", i18n::REQ_CMD_NOT_SATISFIED);
        for line in output {
          println!("{}", line);
        }
      }
      SatisfyErr::Remote(e) => println!("{}", e),
    }
    return Ok(());
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
    let env = if action.exec_in_project_dir.is_some_and(|v| v)
      && let Some(project_dir) = env.project_dir
    {
      RunEnvironment {
        run_dir: project_dir,
        daemons: env.daemons.clone(),
        ..(*env)
      }
    } else {
      RunEnvironment {
        daemons: env.daemons.clone(),
        ..(*env)
      }
    };
    let observer = matches!(&action.action, Action::Observe(_));
    let sub_pipeline = matches!(&action.action, Action::SubPipeline(_));
    let no_pipe = observer | sub_pipeline | env.no_pipe;

    if !env.silent_build {
      if !no_pipe {
        print!(
          "[{}/{}] {} `{}`...",
          cntr,
          total,
          i18n::STARTING_ACTION,
          action.title.blue().italic()
        );
      } else {
        println!(
          "[{}/{}] {} `{}`...",
          cntr,
          total,
          i18n::STARTING_ACTION,
          action.title.blue().italic()
        );
      }
      build_log(
        &log_file,
        &[format!(
          "[{}/{}] {} `{}`...",
          cntr,
          total,
          i18n::STARTING_ACTION,
          action.title
        )],
      )?;
    }
    stdout().flush()?;
    let now = Instant::now();

    let (status, output) = match &action.action {
      Action::SyncToRemote { remote_host_name } => {
        if let Some(remote) = env.remotes.get(remote_host_name) {
          if let Err(e) = sync_to_remote(env.run_dir, remote, env.ignore) {
            (false, vec![e.to_string()])
          } else {
            (true, vec![])
          }
        } else {
          (false, vec![i18n::NO_SUCH_REMOTE.to_string()])
        }
      }
      Action::SyncFromRemote { remote_host_name } => {
        if let Some(remote) = env.remotes.get(remote_host_name) {
          if let Err(e) = sync_from_remote(env.run_dir, remote) {
            (false, vec![e.to_string()])
          } else {
            (true, vec![])
          }
        } else {
          (false, vec![i18n::NO_SUCH_REMOTE.to_string()])
        }
      }
      Action::Custom(cmd) => cmd.execute(&env)?,
      Action::Test(test) => test.execute(&env)?,
      Action::PreBuild(a) | Action::Build(a) | Action::PostBuild(a) => a.execute(&env)?,
      Action::Pack(a) | Action::Deliver(a) | Action::Install(a) => a.execute(&env)?,
      Action::ConfigureDeploy(a) | Action::Deploy(a) | Action::PostDeploy(a) => a.execute(&env)?,
      Action::Observe(o_action) => o_action.execute_observer(&env)?,
      Action::Patch(patch) => patch.execute(&env)?,
      Action::Interrupt => {
        println!();
        #[cfg(feature = "tui")]
        inquire::Confirm::new(i18n::INTERRUPT).with_default(true).prompt()?;
        #[cfg(not(feature = "tui"))]
        let _ = std::io::read_to_string(std::io::stdin())?;
        (true, vec![])
      }
      Action::UseFromStorage { content_info } => match use_from_storage(env.storage_dir, env.run_dir, content_info) {
        Ok(_) => (true, vec![]),
        Err(e) => (false, vec![e.to_string()]),
      },
      Action::AddToStorage(rules) => {
        place_artifacts(config, &env, false)?;
        rules.execute(&env)?
      }
      Action::SubPipeline(pipeline) => {
        execute_pipeline(config, &env, pipeline)?;
        (true, vec![])
      }
    };

    let status_str = match status {
      true => i18n::DONE.to_string(),
      false => i18n::GOT_ERROR.red().bold().to_string(),
    };

    let elapsed = now.elapsed();
    if !observer {
      total_time += elapsed;
    }
    if !no_pipe {
      build_log(&log_file, &output)?;
    }
    build_log(
      &log_file,
      &[format!(
        "[{}/{}] {} -{} ({:.2?}).",
        cntr,
        total,
        i18n::STARTING_ACTION.replace("{}", &action.title),
        if status { i18n::DONE } else { i18n::GOT_ERROR },
        elapsed,
      )],
    )?;

    if !env.silent_build {
      if !no_pipe {
        println!("{} ({}).", status_str, format!("{:.2?}", elapsed).green());
        for line in &output {
          println!("{}", line);
        }
      } else {
        println!(
          "[{}/{}] {} -{} ({}).",
          cntr,
          total,
          i18n::STARTING_ACTION.replace("{}", &action.title.blue().italic()),
          status_str,
          format!("{:.2?}", elapsed).green()
        );
      }
    }

    cntr += 1;

    if !status {
      exit(1)
    }
  }

  println!("{} {}.", i18n::DONE_IN, format!("{:.2?}", total_time).green());
  build_log(&log_file, &[format!("{} {:.2?}.", i18n::DONE_IN, total_time)])?;

  Ok(())
}

fn check_args_on_conflicts(args: &RunArgs) -> anyhow::Result<()> {
  if args.link_cache && args.copy_cache {
    panic!(
      "Select only one option from `{}` and `{}`. See help via `{}`.",
      "c".green(),
      "C".green(),
      "deployer run -h".green()
    );
  }
  if (args.fresh || args.link_cache || args.copy_cache || args.run_at.is_some()) && args.current {
    panic!(
      "Select either `{}` or `{}`/{}`/`{}`/`{}` options. See help via `{}`.",
      "o".green(),
      "j".green(),
      "f".green(),
      "c".green(),
      "C".green(),
      "deployer run -h".green(),
    );
  }
  if args.silent && args.no_pipe {
    panic!(
      "Select only one option from `{}` and `{}`. See help via `{}`.",
      "s".green(),
      "t".green(),
      "deployer run -h".green()
    );
  }
  if args.remote_folder.is_some() && args.pipeline_tags.is_empty() {
    panic!("You always should specify Pipeline tags for executing while remote builds.")
  }
  if !args.remote_host_short_names.is_empty()
    && (args.remote_folder.is_some()
      || args.pipeline_tags.is_empty()
      || args.link_cache
      || args.copy_cache
      || args.fresh
      || args.run_at.is_some()
      || args.current)
  {
    panic!(
      "If you specify remote hosts to run on, you should specify only Pipelines list, `{}`/`{}` flags and nothing more.",
      "silent".green(),
      "no_pipe".green(),
    )
  }

  Ok(())
}
