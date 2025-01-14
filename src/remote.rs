use colored::Colorize;
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
