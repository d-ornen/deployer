use ssh2::Session;

#[derive(Debug)]
pub struct SshClient {
  session: Session,
}

impl SshClient {
  pub fn new() -> Self {
    SshClient { session: Session::new().unwrap() }
  }

  pub fn connect_with_password(&mut self, host: &str, username: &str, password: &str) -> Result<(), Box<dyn Error>> {
    let tcp = TcpStream::connect(host)?;
    self.session.set_tcp_stream(tcp);
    self.session.handshake()?;
    self.session.userauth_password(username, password)?;
    
    if !self.session.authenticated() { return Err("Authentication failed".into()); }
    Ok(())
  }

  pub fn connect_with_key(&mut self, host: &str, username: &str, key_path: &Path) -> Result<(), Box<dyn Error>> {
    let tcp = TcpStream::connect(host)?;
    self.session.set_tcp_stream(tcp);
    self.session.handshake()?;
    self.session.userauth_pubkey_file(
      username,
      None,
      key_path,
      None,
    )?;
    
    if !self.session.authenticated() { return Err("Authentication failed".into()); }
    Ok(())
  }

  pub fn execute_command(&self, command: &str) -> Result<String, Box<dyn Error>> {
    let mut channel = self.session.channel_session()?;
    channel.exec(command)?;
    
    let mut output = String::new();
    channel.read_to_string(&mut output)?;
    
    channel.wait_close()?;
    let exit_status = channel.exit_status()?;
    
    if exit_status != 0 { return Err(format!("Command failed with status {}", exit_status).into()); }
    Ok(output)
  }
}

pub fn generate_ssh_key(key_path: &Path, key_type: &str, bits: u32) -> Result<(), Box<dyn Error>> {
  // Create .ssh directory if it doesn't exist
  if let Some(parent) = key_path.parent() { fs::create_dir_all(parent)?; }

  // Generate key pair using ssh-keygen
  let output = Command::new("ssh-keygen")
    .arg("-t")
    .arg(key_type)
    .arg("-b")
    .arg(bits.to_string())
    .arg("-f")
    .arg(key_path)
    .arg("-N")
    .arg("")
    .output()?;

  if !output.status.success() {
    return Err(format!("Failed to generate SSH key: {}", String::from_utf8_lossy(&output.stderr)).into());
  }

  Ok(())
}

pub fn copy_public_key(client: &mut SshClient, pub_key_path: &Path) -> Result<(), Box<dyn Error>> {
  // Read public key content
  let mut pub_key = String::new();
  File::open(pub_key_path)?.read_to_string(&mut pub_key)?;
  pub_key = pub_key.trim().to_string();

  // Create .ssh directory and set permissions
  let commands = vec![
    "mkdir -p ~/.ssh",
    "chmod 700 ~/.ssh",
    &format!("echo '{}' >> ~/.ssh/authorized_keys", pub_key),
    "chmod 600 ~/.ssh/authorized_keys",
  ];

  for cmd in commands {
    client.execute_command(cmd)?;
  }

  Ok(())
}
