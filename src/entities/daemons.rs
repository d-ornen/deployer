use std::cell::RefCell;
use std::process::Child;
use std::rc::Rc;

pub type Daemons = Rc<RefCell<Vec<Child>>>;

pub fn new_daemons_container() -> Daemons {
  Rc::new(RefCell::new(Vec::new()))
}

pub fn shutdown_daemons(daemons: &Daemons) -> anyhow::Result<()> {
  let mut daemons = daemons.borrow_mut();
  for daemon in daemons.iter_mut().rev() {
    let status = daemon.wait()?;
    if !status.success() {
      anyhow::bail!("Daemon was finished unsuccessfully.")
    }
  }
  Ok(())
}
