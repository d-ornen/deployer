use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::ToSocketAddrs;

use crate::entities::info::ShortName;

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct RemoteHost {
  pub(crate) short_name: ShortName,
  pub(crate) ip: IpAddr,
  pub(crate) port: u16,
  pub(crate) username: String,
  pub(crate) ssh_private_key_file: PathBuf,
}

impl RemoteHost {
  pub(crate) fn check(&self) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
      let mut session = Session::connect(&self.ssh_private_key_file, &self.username, (self.ip, self.port)).await?;
      let (status, out) = session.call("which deployer").await?;
      session.close().await?;
      if status == 0 && !out.is_empty() && out.last().unwrap().contains("deployer") { Ok(()) }
      else { bail!("Remote host doesn't contains `deployer` executable in PATH.") }
    })
  }
  
  pub(crate) fn exec(&self, bash_c: &str, rt: &tokio::runtime::Runtime) -> anyhow::Result<(bool, Vec<String>)> {
    rt.block_on(async {
      let mut session = Session::connect(&self.ssh_private_key_file, &self.username, (self.ip, self.port)).await?;
      let (status, out) = session.call(bash_c).await?;
      session.close().await?;
      match status {
        0 => Ok((true, out)),
        _ => Ok((false, out)),
      }
    })
  }
}

struct Client {}
impl russh::client::Handler for Client { type Error = anyhow::Error; }

struct Session {
  session: russh::client::Handle<Client>,
}

impl Session {
  async fn connect<P: AsRef<Path>, A: ToSocketAddrs>(key_path: P, user: impl Into<String>, addrs: A) -> anyhow::Result<Self> {
    use russh::keys::{load_secret_key, key::PrivateKeyWithHashAlg};
    
    let key_pair = load_secret_key(key_path, None)?;
    let config = russh::client::Config {
      inactivity_timeout: Some(Duration::from_secs(5)),
      preferred: russh::Preferred {
        kex: Cow::Owned(vec![russh::kex::DH_G14_SHA256]),
        ..Default::default()
      },
      ..<_>::default()
    };

    let config = Arc::new(config);
    let sh = Client {};

    let mut session = russh::client::connect(config, addrs, sh).await?;
    let auth_res = session.authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key_pair), None)?).await?;

    if !auth_res { bail!("Authentication failed: {auth_res:?}"); }

    Ok(Self { session })
  }

  async fn call(&mut self, command: &str) -> anyhow::Result<(u32, Vec<String>)> {
    use russh::ChannelMsg;
    
    let mut channel = self.session.channel_open_session().await?;
    channel.exec(true, command).await?;

    let mut status = None;
    let mut out = vec![];
    let mut out_buf = vec![];

    loop {
      let Some(msg) = channel.wait().await else { break; };
      match msg {
        ChannelMsg::Data { ref data } => { out_buf.extend_from_slice(data); },
        ChannelMsg::ExitStatus { exit_status } => { status = Some(exit_status); },
        _ => {},
      }
    }
    
    out.extend_from_slice(String::from_utf8_lossy_owned(out_buf).split('\n').map(|s| s.to_string()).collect::<Vec<_>>().as_slice());
    Ok((status.expect("Remote program did not exit cleanly."), out))
  }

  async fn close(&mut self) -> anyhow::Result<()> {
    self.session.disconnect(russh::Disconnect::ByApplication, "deployer", "English").await?;
    Ok(())
  }
}
