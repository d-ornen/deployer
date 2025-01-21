#![feature(let_chains, if_let_guard, once_wait, string_from_utf8_lossy_owned, str_as_str)]
#![deny(warnings, clippy::todo, clippy::unimplemented)]

pub mod cmd;
pub mod configs;
pub mod rw;
#[cfg(feature = "tui")]
pub mod tui;
pub mod utils;

pub mod build;

pub mod remote;
pub mod storage;

pub mod actions;
pub mod entities;
pub mod pipelines;

pub mod i18n;

pub static PROJECT_CONF: &str = "deploy-config.json";
pub static HIDDEN_PROJECT_CONF: &str = ".deploy-config.json";
pub static GLOBAL_CONF: &str = "deploy-global.json";
pub static BUILD_CACHE_LIST: &str = "deploy-builds.json";

pub static CACHE_DIR: &str = "deploy-cache";
pub static LOGS_DIR: &str = "logs";
pub static STORAGE_DIR: &str = "deployer";

pub static ARTIFACTS_DIR: &str = "artifacts";

pub static CTRLC_HANDLER: std::sync::LazyLock<std::sync::Arc<std::sync::Mutex<Option<std::process::Child>>>> =
  std::sync::LazyLock::new(|| std::sync::Arc::new(std::sync::Mutex::new(None)));

pub fn install_ctrlc_handler() -> anyhow::Result<()> {
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
  .map_err(|_| anyhow::anyhow!("Error setting Ctrl-C handler"))
}
