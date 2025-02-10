use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::cell::RefCell;
use std::process::Child;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct Daemons(Rc<RefCell<Vec<Daemon>>>);
pub struct Daemon(Option<Child>);

impl Daemons {
  pub fn new() -> Self {
    Self(Rc::new(RefCell::new(Vec::new())))
  }

  pub fn add_daemon(&self, child: Child) {
    self.0.borrow_mut().push(Daemon(Some(child)));
  }
}

impl Drop for Daemon {
  fn drop(&mut self) {
    let mut child = self.0.take().unwrap();
    if let Ok(None) = child.try_wait() {
      let _ = signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGTERM);
      let mut cntr = 0u16;
      while let Ok(None) = child.try_wait()
        && cntr < 10
      {
        std::thread::sleep(std::time::Duration::from_secs(1));
        cntr += 1;
      }
      let _ = child.kill();
    }
    drop(child);
  }
}
