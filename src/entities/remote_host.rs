//! Remote host module.
//! 
//! Any remote host struct is a set of properties needed to connect and authenticate by `ssh`.

use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::ToSocketAddrs;

use crate::entities::custom_command::compose_output;
use crate::entities::info::ShortName;
use crate::i18n;

/// Remote host.
#[derive(Deserialize, Serialize, Clone)]
pub struct RemoteHost {
  /// Short name (remote host identifier inside Deployer's Registry).
  pub short_name: ShortName,
  /// IP address of SSH server.
  pub ip: IpAddr,
  /// Port of SSH server.
  pub port: u16,
  /// Username under which you plan to perform operations on the host.
  pub username: String,
  /// Path to private SSH key file.
  pub ssh_private_key_file: PathBuf,
}

impl RemoteHost {
  /// Checks the remote host connectivity, authorization and Deployer installation existence.
  pub fn check(&self) -> anyhow::Result<()> {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
    
    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => path,
      Err(_) => "/bin/bash".to_string(),
    };
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
      let mut session = Session::connect(&self.ssh_private_key_file, &self.username, (self.ip, self.port)).await?;
      let out = session.call(&format!(r#"{} -c "~/.cargo/bin/deployer -V""#, shell)).await?;
      session.close().await?;
      if out.is_empty() || !out.contains(PKG_NAME) {
        bail!(i18n::REMOTE_NO_DEPLOYER)
      } else {
        if !out.contains(&format!("{} {}", PKG_NAME, PKG_VERSION)) {
          println!(r#"{} (out: "{}")"#, i18n::REMOTE_CONSIDER_UPGRADE, out.trim());
        }
        Ok(())
      }
    })
  }
  
  /// Starts Deployer's Pipeline execution on the remote host with given remote build folder.
  pub fn call_deployer_to_build(&self, remote_build_dir: &Path, pipeline: &str) -> anyhow::Result<(bool, Vec<String>)> {
    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => path,
      Err(_) => "/bin/bash".to_string(),
    };
    
    let rt = tokio::runtime::Runtime::new()?;
    let cmd = format!(
      r#"{} -c "~/.cargo/bin/deployer build -r {} {} && echo $?""#,
      shell,
      remote_build_dir.to_string_lossy().as_str(),
      pipeline,
    );
    rt.block_on(async {
      let mut session = Session::connect(&self.ssh_private_key_file, &self.username, (self.ip, self.port)).await?;
      let out = session.call(cmd.as_str()).await?;
      session.close().await?;
      let success = out.ends_with("\n0\n");
      let mut out = compose_output(cmd, out, String::new(), success, true, true);
      out.pop();
      Ok((success, out))
    })
  }
  
  /// Opens the session with given runtime.
  pub fn open_session(&self, rt: &tokio::runtime::Runtime) -> anyhow::Result<Session> {
    rt.block_on(Session::connect(&self.ssh_private_key_file, &self.username, (self.ip, self.port)))
  }
  
  /// Closes the session with given runtime.
  pub fn close_session(session: &mut Session, rt: &tokio::runtime::Runtime) -> anyhow::Result<()> {
    rt.block_on(session.close())
  }
  
  /// Executes single shell command with given session and runtime.
  pub fn exec(&self, bash_c: &str, session: &mut Session, rt: &tokio::runtime::Runtime) -> anyhow::Result<(bool, String)> {
    rt.block_on(async {
      let out = session.call(bash_c).await?;
      let success = out.ends_with("\n0\n");
      Ok((success, out))
    })
  }
}

struct Client {}

#[async_trait::async_trait]
impl russh::client::Handler for Client {
  type Error = anyhow::Error;
  
  /// WARNING: allows any server keys without authorization.
  /// 
  /// May lead to any security consequences, but simplifies remote host setup.
  async fn check_server_key(&mut self, _: &russh::keys::ssh_key::PublicKey) -> Result<bool, Self::Error> {
    Ok(true)
  }
}

pub struct Session {
  session: russh::client::Handle<Client>,
}

impl Session {
  async fn connect<P: AsRef<Path>, A: ToSocketAddrs>(key_path: P, user: impl Into<String>, addrs: A) -> anyhow::Result<Self> {
    use russh::keys::{load_secret_key, key::PrivateKeyWithHashAlg};
    
    let key_pair = load_secret_key(key_path, None)?;
    let config = russh::client::Config {
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

  async fn call(&mut self, command: &str) -> anyhow::Result<String> {
    use russh::ChannelMsg;
    
    let mut channel = self.session.channel_open_session().await?;
    channel.exec(true, command).await?;
    
    let mut out_buf = vec![];
    while let Some(msg) = channel.wait().await {
      if let ChannelMsg::Data { ref data } = msg { out_buf.extend_from_slice(data); }
    }
    
    Ok(String::from_utf8_lossy_owned(out_buf))
  }

  async fn close(&mut self) -> anyhow::Result<()> {
    self.session.disconnect(russh::Disconnect::ByApplication, "deployer", "English").await?;
    Ok(())
  }
}
