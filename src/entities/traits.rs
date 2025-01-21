//! Traits module.
//! 
//! Contains common Deployer traits.

use crate::entities::environment::BuildEnvironment;

/// Allows to edit given entity via TUI.
#[cfg(feature = "tui")]
pub trait Edit {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()>;
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  fn add_item(&mut self) -> anyhow::Result<()> { Ok(()) }
  fn remove_item(&mut self) -> anyhow::Result<()> { Ok(()) }
}

/// Allows to edit given entity via TUI, with additional `opts`.
/// 
/// For example, you need to edit Pipeline, and you want to create a new
/// Action for this Pipeline. But you also wanna add this Action to Registry;
/// this is why you can implement `EditExtended<DeployerGlobalConfig>` and
/// add created Action to the Registry.
#[cfg(feature = "tui")]
pub trait EditExtended<T> {
  fn edit_from_prompt(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn reorder(&mut self, _opts: &mut T) -> anyhow::Result<()> { Ok(()) }
  fn add_item(&mut self, _opts: &mut T) -> anyhow::Result<()> { Ok(()) }
  fn remove_item(&mut self, _opts: &mut T) -> anyhow::Result<()> { Ok(()) }
}

/// Executes given entity with build environment.
pub trait Execute {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)>;
  fn execute_observer(&self, #[allow(unused_variables)] env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> { Ok((true, vec![])) }
}
