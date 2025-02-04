//! Deployer
//!
//! Deployer is a relative simple, yet powerful localhost CI/CD instrument. It allows you to:
//!
//! - have your own actions and pipelines repositories (`Actions Registry` and `Pipelines Registry`) in a single JSON file
//! - create actions and pipelines from TUI or JSON configuration files
//! - configure actions for specific project
//! - satisfy requirements for your system to run pipelines
//! - check compatibility over actions and projects
//! - run actions and pipelines at remote hosts (you need to setup your remote with SSH key and install `deployer`)
//! - use variables for commands from `env`-files and HashiCorp Vault KV2-storage
//! - run pipelines with different cache requirements in different build folders
//! - store common content in Deployer's storage, add and patch additional files for build on the fly
//! - and share your project build/deploy settings very quickly and without any dependencies.
//!
//! For further reading, check `README.md`, `DOCS.en.md` and `DOCS.ru.md`.

#![feature(let_chains, if_let_guard, once_wait, string_from_utf8_lossy_owned, str_as_str)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

#[cfg(feature = "tests")]
mod tests;

mod cmd;
mod configs;
mod rw;
#[cfg(feature = "tui")]
mod tui;
mod utils;

#[cfg(feature = "containered")]
mod containered;
mod run;

mod remote;
mod storage;

mod actions;
mod entities;
mod pipelines;
#[cfg(feature = "tui")]
mod project;

mod i18n;

use std::path::PathBuf;

use crate::actions::{cat_action, edit_action, list_actions, new_action, remove_action};
use crate::cmd::{CatType, Cli, DeployerExecType, EditType, ListType, NewType, RemoveType};
use crate::configs::{DeployerGlobalConfig, DeployerProjectOptions};
use crate::pipelines::{
  assign_pipeline_to_project, cat_pipeline, cat_project_pipelines, edit_pipeline, list_pipelines, new_pipeline,
  remove_pipeline,
};
use crate::project::{edit_project, init_project};
use crate::remote::{cat_remote, edit_remote, list_remote, new_remote, remove_remote};
use crate::run::Runs;
use crate::rw::{VERBOSE, read, read_or_migrate, write, write_merge};
use crate::storage::{list_content, new_content, remove_content};
use crate::tui::docs;
use crate::utils::get_current_working_dir;

#[cfg(feature = "tests")]
use crate::tests::tests;

use crate::run::{clean_runs, run};

use clap::Parser;
use dirs::{cache_dir, config_dir, data_local_dir};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static PROJECT_CONF: &str = "deploy-config.json";
static HIDDEN_PROJECT_CONF: &str = ".deploy-config.json";
static GLOBAL_CONF: &str = "deploy-global.json";
static BUILD_CACHE_LIST: &str = "deploy-builds.json";

pub static CACHE_DIR: &str = "deploy-cache";
pub static LOGS_DIR: &str = "logs";
pub static STORAGE_DIR: &str = "deployer";

pub static ARTIFACTS_DIR: &str = "artifacts";

pub static CTRLC_HANDLER: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex<Option<std::process::Child>>>> =
  std::sync::LazyLock::new(|| std::sync::Arc::new(std::sync::Mutex::new(None)));

#[cfg(not(unix))]
compile_error!("`deployer` can't work with non-Unix systems.");

fn main() {
  std::panic::set_hook(Box::new(|e| {
    let err = e.to_string();
    if err.contains("called `Result::unwrap()` on an `Err` value: ") {
      eprintln!(
        "{}",
        err
          .split("called `Result::unwrap()` on an `Err` value: ")
          .last()
          .unwrap()
      );
    } else {
      eprintln!("{}", err.split('\n').next_back().unwrap());
    }
    std::process::exit(1);
  }));

  ctrlc::set_handler(move || {
    let mut guard = CTRLC_HANDLER.lock().unwrap();
    if let Some(child) = guard.as_mut() {
      if let Err(e) = child.kill() {
        eprintln!("Failed to kill process: {}", e);
      }
      *guard = None;
    } else {
      println!("\nInterrupted");
      std::process::exit(0);
    }
  })
  .expect("Error setting Ctrl-C handler");

  let args = Cli::parse();

  if args.verbose {
    if let DeployerExecType::Run(run_args) = &args.r#type
      && run_args.silent
    {
      VERBOSE.set(false).unwrap();
    } else {
      VERBOSE.set(true).unwrap();
    }
  } else {
    VERBOSE.set(false).unwrap();
  }

  let cache_folder = if let Some(cache_folder) = &args.cache_folder {
    let cf = PathBuf::from(cache_folder);
    if cf.is_absolute() {
      cf
    } else {
      let path = PathBuf::new();
      path.join(cf)
    }
  } else {
    cache_dir().expect("Can't get `cache` directory's location automatically, please specify one.")
  };
  let config_folder = if let Some(config_folder) = &args.config_folder {
    let cf = PathBuf::from(config_folder);
    if cf.is_absolute() {
      cf
    } else {
      let path = PathBuf::new();
      path.join(cf)
    }
  } else {
    config_dir().expect("Can't get `config` directory's location automatically, please specify one.")
  };
  let storage_folder = if let Some(storage_folder) = &args.storage_folder {
    let sf = PathBuf::from(storage_folder);
    if sf.is_absolute() {
      sf
    } else {
      let path = PathBuf::new();
      path.join(sf)
    }
  } else {
    data_local_dir().expect("Can't get `storage` directory's location automatically, please specify one.")
  };

  let mut globals = read_or_migrate::<DeployerGlobalConfig>(&config_folder, GLOBAL_CONF);
  DeployerGlobalConfig::make_sure_contain_defaults(&mut globals.actions_registry);
  let mut config = read_or_migrate::<DeployerProjectOptions>(&get_current_working_dir().unwrap(), PROJECT_CONF);
  if config == Default::default() {
    config = read_or_migrate::<DeployerProjectOptions>(&get_current_working_dir().unwrap(), HIDDEN_PROJECT_CONF);
  }
  let mut runs = read::<Runs>(&cache_folder, BUILD_CACHE_LIST);

  match args.r#type {
    DeployerExecType::Ls(ListType::Actions) => list_actions(&globals),
    DeployerExecType::New(NewType::Action(args)) => {
      let _ = new_action(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }
    DeployerExecType::Cat(CatType::Action(args)) => cat_action(&globals, &args).unwrap(),
    DeployerExecType::Edit(EditType::Action(args)) => {
      edit_action(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }
    DeployerExecType::Rm(RemoveType::Action) => {
      remove_action(&mut globals).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }

    DeployerExecType::Ls(ListType::Pipelines) => list_pipelines(&globals).unwrap(),
    DeployerExecType::New(NewType::Pipeline(args)) => {
      new_pipeline(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }
    DeployerExecType::Cat(CatType::Pipeline(args)) => cat_pipeline(&globals, &args).unwrap(),
    DeployerExecType::Edit(EditType::Pipeline(args)) => {
      edit_pipeline(&mut globals, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }
    DeployerExecType::Rm(RemoveType::Pipeline) => {
      remove_pipeline(&mut globals).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }

    DeployerExecType::Ls(ListType::Content) => list_content(&storage_folder).unwrap(),
    DeployerExecType::New(NewType::Content) => new_content(&storage_folder).unwrap(),
    DeployerExecType::Rm(RemoveType::Content) => remove_content(&storage_folder).unwrap(),

    DeployerExecType::Ls(ListType::Remote) => list_remote(&globals),
    DeployerExecType::New(NewType::Remote(args)) => {
      let _ = new_remote(&mut globals, args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }
    DeployerExecType::Cat(CatType::Remote(args)) => cat_remote(&globals, args).unwrap(),
    DeployerExecType::Edit(EditType::Remote(args)) => {
      edit_remote(&mut globals, args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }
    DeployerExecType::Rm(RemoveType::Remote) => {
      remove_remote(&mut globals).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
    }

    DeployerExecType::Init(args) => {
      init_project(&mut globals, &mut config, &args).unwrap();
      write(get_current_working_dir().unwrap(), PROJECT_CONF, &config);
    }
    DeployerExecType::With(args) => {
      assign_pipeline_to_project(&mut globals, &mut config, &args).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
      write(get_current_working_dir().unwrap(), PROJECT_CONF, &config);
    }
    DeployerExecType::Cat(CatType::Project(args)) => cat_project_pipelines(&config, args).unwrap(),
    DeployerExecType::Edit(EditType::Project) => {
      edit_project(&mut globals, &mut config).unwrap();
      write(&config_folder, GLOBAL_CONF, &globals);
      write(get_current_working_dir().unwrap(), PROJECT_CONF, &config);
    }
    DeployerExecType::Run(args) => {
      run(
        &mut config,
        &globals,
        &mut runs,
        &cache_folder,
        &config_folder,
        &storage_folder,
        &args,
      )
      .unwrap();
      write_merge(&cache_folder, BUILD_CACHE_LIST, &runs);
    }
    DeployerExecType::Clean(args) => {
      clean_runs(&config, &mut runs, &cache_folder, &args).unwrap();
      write(&cache_folder, BUILD_CACHE_LIST, &runs);
    }

    DeployerExecType::Docs => docs::read_docs().unwrap(),

    #[cfg(feature = "tests")]
    DeployerExecType::Tests => tests().unwrap(),
  }
}
