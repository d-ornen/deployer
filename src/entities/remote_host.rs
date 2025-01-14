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
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    
    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => path,
      Err(_) => "/bin/bash".to_string(),
    };
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
      let mut session = Session::connect(&self.ssh_private_key_file, &self.username, (self.ip, self.port)).await?;
      let (s, o) = session.call(&format!(r#"{} -c "~/.cargo/bin/deployer -V""#, shell)).await?;
      session.close().await?;
      if s != 0 || o.is_empty() || !o.contains(&format!("{} {}", PKG_NAME, PKG_VERSION)) {
        bail!(r#"Deployer version on remote host didn't match with this Deployer version (out: "{}")"#, o.trim()) }
      else { Ok(()) }
    })
  }
  
  pub(crate) fn open_session(&self, rt: &tokio::runtime::Runtime) -> anyhow::Result<Session> {
    rt.block_on(Session::connect(&self.ssh_private_key_file, &self.username, (self.ip, self.port)))
  }
  
  pub(crate) fn close_session(session: &mut Session, rt: &tokio::runtime::Runtime) -> anyhow::Result<()> {
    rt.block_on(session.close())
  }
  
  pub(crate) fn exec(&self, bash_c: &str, session: &mut Session, rt: &tokio::runtime::Runtime) -> anyhow::Result<(bool, String)> {
    rt.block_on(async {
      let (status, out) = session.call(bash_c).await?;
      match status {
        0 => Ok((true, out)),
        _ => Ok((false, out)),
      }
    })
  }
}

struct Client {}

#[async_trait::async_trait]
impl russh::client::Handler for Client {
  type Error = anyhow::Error;
  
  async fn check_server_key(&mut self, _: &russh::keys::ssh_key::PublicKey) -> Result<bool, Self::Error> {
    Ok(true)
  }
}

pub(crate) struct Session {
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

    let mut session = match russh::client::connect(config, addrs, sh).await {
      Ok(s) => s,
      Err(e) => bail!("Client connect failed: {e:?}"),
    };
    let auth_res = session.authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key_pair), Some(russh::keys::HashAlg::Sha256))?).await?;

    if !auth_res { bail!("Authentication failed: {auth_res:?}"); }

    Ok(Self { session })
  }

  async fn call(&mut self, command: &str) -> anyhow::Result<(u32, String)> {
    use russh::ChannelMsg;
    
    let mut channel = self.session.channel_open_session().await?;
    channel.exec(true, command).await?;

    let mut status = None;
    let mut out_buf = vec![];

    loop {
      let Some(msg) = channel.wait().await else { break; };
      match msg {
        ChannelMsg::Data { ref data } => { out_buf.extend_from_slice(data); },
        ChannelMsg::ExitStatus { exit_status } => { status = Some(exit_status); },
        _ => {},
      }
    }
    
    let out = String::from_utf8_lossy_owned(out_buf);
    Ok((status.expect("Remote program did not exit cleanly."), out))
  }

  async fn close(&mut self) -> anyhow::Result<()> {
    self.session.disconnect(russh::Disconnect::ByApplication, "deployer", "English").await?;
    Ok(())
  }
}
