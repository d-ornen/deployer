use anyhow::bail;
use colored::Colorize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::exit;

use crate::cmd::{CatRemoteArgs, NewRemoteArgs};
use crate::configs::DeployerGlobalConfig;
use crate::entities::info::ShortName;
use crate::entities::remote_host::RemoteHost;
use crate::hmap;
use crate::i18n;

pub(crate) fn list_remote(globals: &DeployerGlobalConfig) {
  println!("{}", i18n::KNOWN_HOSTS);
  
  let mut hosts = globals.remote_hosts.values().collect::<Vec<_>>();
  hosts.sort_by_key(|r| r.short_name.as_str());
  
  for host in hosts {
    println!("• {}: `{}`", i18n::HOST, host.short_name.as_str().green().italic());
  }
}

pub(crate) fn new_remote(
  globals: &mut DeployerGlobalConfig,
  args: NewRemoteArgs,
) -> anyhow::Result<RemoteHost> {
  let remote = RemoteHost::new_with_args_from_prompt(args)?;
  globals.remote_hosts.insert(remote.short_name.clone(), remote.clone());
  
  Ok(remote)
}

pub(crate) fn cat_remote(
  globals: &DeployerGlobalConfig,
  args: CatRemoteArgs,
) -> anyhow::Result<()> {
  let remote = match globals.remote_hosts.get(&ShortName::new(args.remote_host_short_info)?) {
    None => exit(1),
    Some(remote) => remote,
  };
  
  println!("{}: {}", i18n::HOST_SHORT_NAME, remote.short_name.as_str());
  println!("{}: {}", i18n::HOST_IP, remote.ip);
  println!("{}: {}", i18n::HOST_PORT, remote.port);
  println!("{}: {}", i18n::HOST_USERNAME, remote.username);
  
  Ok(())
}

pub(crate) fn edit_remote(
  globals: &mut DeployerGlobalConfig,
  args: CatRemoteArgs,
) -> anyhow::Result<()> {
  let remote = match globals.remote_hosts.get_mut(&ShortName::new(&args.remote_host_short_info)?) {
    None => exit(1),
    Some(remote) => remote,
  };
  
  remote.edit_from_prompt()?;
  
  Ok(())
}

pub(crate) fn remove_remote(globals: &mut DeployerGlobalConfig) -> anyhow::Result<()> {
  use inquire::{Select, Confirm};
  
  if globals.remote_hosts.is_empty() {
    println!("{}", i18n::NO_HOSTS);
    return Ok(())
  }
  
  let (remote, keys) = {
    let mut h = hmap!();
    let mut k = vec![];
    
    for key in globals.remote_hosts.keys() {
      let host = globals.remote_hosts.get(key).unwrap();
      let new_key = format!("• {}: `{}`", i18n::HOST, host.short_name.as_str());
      h.insert(new_key.clone(), host);
      k.push(new_key);
    }
    
    k.sort();
    
    (h, k)
  };
  
  let host = Select::new(i18n::REMOTE_REGISTRY_CHOOSE_TO_REMOVE, keys).prompt()?;
  let host = *remote.get(&host).unwrap();
  let short_name = host.short_name.clone();
  
  if !Confirm::new(i18n::ARE_YOU_SURE).prompt()? { return Ok(()) }
  
  globals.remote_hosts.remove(&short_name);
  
  Ok(())
}

pub(crate) fn sync_to_remote(
  build_dir: &Path,
  remote: &RemoteHost,
  ignore: &HashSet<PathBuf>,
) -> anyhow::Result<PathBuf> {
  let ignore = ignore
    .iter()
    .map(|p| format!("--exclude='{}'", p.to_string_lossy()))
    .collect::<Vec<_>>()
    .join(" ");
  
  let mut remote_build_folder = PathBuf::from("~");
  remote_build_folder.push(".cache");
  remote_build_folder.push(crate::CACHE_DIR);
  
  let build_pathbuf = build_dir.to_path_buf();
  let folder_name = build_pathbuf.file_name().unwrap().to_string_lossy();
  remote_build_folder.push(folder_name.as_str());
  
  let bash_c = format!(
    r#"rsync -avz {} --rsh='ssh -p{}' . "{}@{}:{}""#,
    ignore,
    remote.port,
    remote.username,
    remote.ip,
    remote_build_folder.to_string_lossy(),
  );
  
  let shell = match std::env::var("DEPLOYER_SH_PATH") {
    Ok(path) => path,
    Err(_) => "/bin/bash".to_string(),
  };
  
  let mut cmd = std::process::Command::new(&shell);
  cmd.current_dir(build_dir).arg("-c").arg(bash_c).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
  let res = cmd.spawn()?.wait_with_output()?;
  if !res.status.success() {
    let stdout_strs = String::from_utf8_lossy_owned(res.stdout);
    let stderr_strs = String::from_utf8_lossy_owned(res.stderr);
    bail!("{}{}", stdout_strs, stderr_strs)
  }
  
  Ok(remote_build_folder)
}

pub(crate) fn sync_from_remote(
  build_dir: &Path,
  artifacts_dir: &Path,
  remote: &RemoteHost,
) -> anyhow::Result<()> {
  let mut remote_build_folder = PathBuf::from("~");
  remote_build_folder.push(".cache");
  remote_build_folder.push(crate::CACHE_DIR);
  
  let build_pathbuf = build_dir.to_path_buf();
  let folder_name = build_pathbuf.file_name().unwrap().to_string_lossy();
  remote_build_folder.push(folder_name.as_str());
  remote_build_folder.push(crate::ARTIFACTS_DIR);
  
  let mut artifacts_pathbuf = artifacts_dir.to_path_buf();
  artifacts_pathbuf.push(remote.short_name.as_str());
  std::fs::create_dir_all(&artifacts_pathbuf)?;
  
  let bash_c = format!(
    r#"rsync -avz --rsh='ssh -p{}' "{}@{}:{}" {:?}"#,
    remote.port,
    remote.username,
    remote.ip,
    remote_build_folder.to_string_lossy(),
    artifacts_pathbuf,
  );
  
  let shell = match std::env::var("DEPLOYER_SH_PATH") {
    Ok(path) => path,
    Err(_) => "/bin/bash".to_string(),
  };
  
  let mut cmd = std::process::Command::new(&shell);
  cmd.current_dir(build_dir).arg("-c").arg(bash_c).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
  let res = cmd.spawn()?.wait_with_output()?;
  if !res.status.success() {
    let stdout_strs = String::from_utf8_lossy_owned(res.stdout);
    let stderr_strs = String::from_utf8_lossy_owned(res.stderr);
    bail!("{}{}", stdout_strs, stderr_strs)
  }
  
  Ok(())
}
