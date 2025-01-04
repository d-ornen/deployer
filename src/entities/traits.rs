use crate::entities::environment::BuildEnvironment;

pub(crate) trait Edit {
  fn edit_from_prompt(&mut self) -> anyhow::Result<()>;
  fn reorder(&mut self) -> anyhow::Result<()> { Ok(()) }
  fn add_item(&mut self) -> anyhow::Result<()> { Ok(()) }
  fn remove_item(&mut self) -> anyhow::Result<()> { Ok(()) }
}

pub(crate) trait EditExtended<T> {
  fn edit_from_prompt(&mut self, opts: &mut T) -> anyhow::Result<()>;
  fn reorder(&mut self, _opts: &mut T) -> anyhow::Result<()> { Ok(()) }
  fn add_item(&mut self, _opts: &mut T) -> anyhow::Result<()> { Ok(()) }
  fn remove_item(&mut self, _opts: &mut T) -> anyhow::Result<()> { Ok(()) }
}

pub(crate) trait Execute {
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)>;
}
