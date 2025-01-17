//! Custom command module.
//! 
//! Custom command is such an entity that allows you to perform actions with
//! shell commands. Shell is `/bin/bash` by default, but you can specify it
//! with `DEPLOYER_SH_PATH` environment variable.
//! 
//! Also custom command supports several features and placeholders.
//! E.g., you can specify a command like `docker load -i <img>`, and after
//! this say that `<img>` is a placeholder, and in real Pipeline you just
//! replace `<img>` by concrete value (see `Variable` struct).

use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::i18n;
use crate::entities::environment::BuildEnvironment;
use crate::entities::info::ShortName;
use crate::entities::remote_host::RemoteHost;
use crate::entities::traits::Execute;
use crate::entities::variables::Variable;

/// Custom command.
#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, Clone)]
pub(crate) struct CustomCommand {
  /// Shell command that should be executed.
  pub(crate) bash_c: String,
  
  /// Command placeholders. You can specify several placeholders in single command.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) placeholders: Option<Vec<String>>,
  /// Variables list to replace placeholders.
  /// 
  /// If you have many variables to perform one command with (e.g., have to execute
  /// `cargo build --bin project1` and `--bin project2`), you can make two `Vec` inside
  /// single:
  /// 
  /// ```rust,ignore
  /// replacements: Some(vec![
  ///   // this is for the first execution
  ///   vec![("<pr>", Variable::new_plain("pr1", "project1"))],
  ///   // this is for the second execution
  ///   vec![("<pr>", Variable::new_plain("pr2", "project2"))],
  /// ])
  /// ```
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) replacements: Option<Vec<Vec<(String, Variable)>>>,
  
  /// Flag to ignore command fails.
  /// 
  /// Any status code your command returns which isn't equal zero means fail. But this
  /// flag allows to avoid Pipeline early exit, if needed. Error will be ignored.
  pub(crate) ignore_fails: bool,
  
  /// Flag to show output if the command was finished successfully.
  /// 
  /// E.g., if you don't wanna see `cargo build` output when code is built successfully,
  /// you can set this flag to `true`.
  pub(crate) show_success_output: bool,
  
  /// Flag to show command on screen (during Pipeline execution).
  /// 
  /// Allows to hide constructed command (from `bash_c` and replaced variables) to avoid
  /// leaking secrets (keys, tokens, paths to sensitive files, etc.).
  pub(crate) show_bash_c: bool,
  
  /// Flag to run this command only once on repeateable builds.
  /// 
  /// If set, this command will be performed only when Deployer starts fresh build
  /// (e.g., when no build cache is present, or `build -f` command flag is given).
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) only_when_fresh: Option<bool>,
  
  /// List with remote's short names.
  /// 
  /// If is specified and isn't empty, command will be performed only on given remote hosts.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) remote_exec: Option<Vec<ShortName>>,
}

impl Execute for CustomCommand {
  /// Runs given command one time or more, replacing the placeholders with given values.
  fn execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    if self.remote_exec.as_ref().is_some_and(|rs| !rs.is_empty()) { return self.remote_execute(env); }
    
    let mut output = vec![];
    
    if !env.new_build && self.only_when_fresh.is_some_and(|v| v) {
      if *crate::rw::VERBOSE.wait() {
        output.push(i18n::CMD_SKIP_DUE_TO_NOT_FRESH.to_string());
      }
      return Ok((true, output))
    }
    
    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => path,
      Err(_) => "/bin/bash".to_string(),
    };
    
    let mut cmds = vec![];
    if self.placeholders.is_some() && let Some(replacements) = &self.replacements {
      for every_start in replacements {
        let mut bash_c = self.bash_c.to_owned();
        
        for (from, to) in every_start { bash_c = bash_c.replace(from, to.get_value()?.as_str()); }
        cmds.push(bash_c);
      }
    } else {
      cmds.push(self.bash_c.to_owned());
    }
    
    for bash_c in &cmds {
      let bash_c_info = format!(r#"{} -c "{}""#, shell, bash_c).green();
      
      let mut cmd = std::process::Command::new(&shell);
      cmd.current_dir(env.build_dir).arg("-c").arg(bash_c);
      
      if !env.no_pipe { cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()); }
      
      let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("Can't execute command due to: {}", e))?;
      
      let success = if env.no_pipe {
        let res = child.wait().map_err(|e| anyhow::anyhow!("Can't wait for exit status due to: {}", e))?;
        res.success()
      } else {
        let command_output = child.wait_with_output().map_err(|e| anyhow::anyhow!("Can't wait for output due to: {}", e))?;
        
        let stdout_strs = String::from_utf8_lossy_owned(command_output.stdout);
        let stderr_strs = String::from_utf8_lossy_owned(command_output.stderr);
        output.extend_from_slice(&compose_output(
          bash_c_info.to_string(),
          stdout_strs,
          stderr_strs,
          command_output.status.success(),
          self.show_success_output,
          self.show_bash_c,
        ));
        
        command_output.status.success()
      };
      
      if !self.ignore_fails && !success {
        return Ok((false, output))
      }
    }
    
    Ok((true, output))
  }
}

impl CustomCommand {
  /// Runs given command remotely on one or more remote hosts.
  pub(crate) fn remote_execute(&self, env: BuildEnvironment) -> anyhow::Result<(bool, Vec<String>)> {
    let hosts = self.remote_exec.as_ref().unwrap();
    let mut output = vec![];
    
    let shell = match std::env::var("DEPLOYER_SH_PATH") {
      Ok(path) => path,
      Err(_) => "/bin/bash".to_string(),
    };
    
    if !env.new_build && self.only_when_fresh.is_some_and(|v| v) {
      if *crate::rw::VERBOSE.wait() {
        output.push(i18n::CMD_SKIP_DUE_TO_NOT_FRESH.to_string());
      }
      return Ok((true, output))
    }
    
    let rt = tokio::runtime::Runtime::new()?;
    let globals = crate::rw::read::<crate::configs::DeployerGlobalConfig>(&env.config_dir, crate::GLOBAL_CONF);
    
    let mut cmds = vec![];
    if self.placeholders.is_some() && let Some(replacements) = &self.replacements {
      for every_start in replacements {
        let mut bash_c = self.bash_c.to_owned();
        
        for (from, to) in every_start { bash_c = bash_c.replace(from, to.get_value()?.as_str()); }
        cmds.push(bash_c);
      }
    } else {
      cmds.push(self.bash_c.to_owned());
    }
    
    for hostname in hosts {
      output.push(format!("{}: `{}`", i18n::REMOTE_EXEC, hostname.as_str().green()));
      let remote = match globals.remote_hosts.get(hostname) {
        None => {
          output.push(i18n::NO_SUCH_REMOTE.to_string());
          if !self.ignore_fails {
            return Ok((false, output))
          }
          continue
        },
        Some(remote) => remote,
      };
      
      let mut session = remote.open_session(&rt)?;
      for bash_c in &cmds {
        let bash_c_info = format!(r#"{} -c "{}""#, shell, bash_c).green();
        let (s, out) = remote.exec(bash_c, &mut session, &rt)?;
        
        output.extend_from_slice(&compose_output(
          bash_c_info.to_string(),
          out,
          String::new(),
          s,
          self.show_success_output,
          self.show_bash_c,
        ));
        
        if !self.ignore_fails && !s {
          RemoteHost::close_session(&mut session, &rt)?;
          return Ok((false, output))
        }
      }
      RemoteHost::close_session(&mut session, &rt)?;
    }
    
    Ok((true, output))
  }
}

/// Composes output from given `stdout` and `stderr` to Deployer's out.
fn compose_output(
  bash_c_info: String,
  stdout: String,
  stderr: String,
  success: bool,
  show_success_output: bool,
  show_bash_c: bool,
) -> Vec<String> {
  let mut output = vec![];
  
  if success && !show_success_output { return output }
  
  if !stdout.trim().is_empty() || !stderr.trim().is_empty() {
    if show_bash_c {
      output.push(format!("{} `{}`:", i18n::EXECUTING, bash_c_info));
    } else {
      output.push(i18n::EXECUTING_HIDDEN.to_string());
    }
  }
  if !stdout.trim().is_empty() {
    let total = stdout.chars().filter(|c| *c == '\n').count();
    
    for (i, line) in stdout.split('\n').enumerate() {
      if i == total && line.trim().is_empty() { break }
      output.push(format!(">>> {}", line));
    }
  }
  if !stderr.trim().is_empty() {
    let total = stderr.chars().filter(|c| *c == '\n').count();
    if total != 0 && !success { output.push(format!("{}", i18n::ERRORS.red().bold())); }
    
    for (i, line) in stderr.split('\n').enumerate() {
      if i == total && line.trim().is_empty() { break }
      output.push(format!(">>> {}", line));
    }
  }
  
  if
    let Ok(num) = std::env::var("DEPLOYER_TRIM_ERR_OUT_LINES") &&
    let Ok(num) = num.parse::<usize>() &&
    num <= output.len() &&
    !success
  {
    output[(output.len()-1-num)..(output.len()-1)].to_vec()
  } else {
    output
  }
}
