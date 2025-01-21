//! CMD module.
//!
//! Defines the Deployer's command-line interface.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Build and deploy your services as fast as you can.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
  /// Command
  #[command(subcommand)]
  pub r#type: DeployerExecType,
  /// Verbose
  #[arg(short)]
  pub verbose: bool,
  /// Specify cache folder
  #[arg(long)]
  pub cache_folder: Option<String>,
  /// Specify config folder
  #[arg(long)]
  pub config_folder: Option<String>,
  /// Specify data folder
  #[arg(long)]
  pub storage_folder: Option<String>,
}

#[derive(Subcommand)]
pub enum DeployerExecType {
  /// List inner Deployer's registries
  #[command(subcommand)]
  Ls(ListType),
  /// Create new inner Deployer's object
  #[command(subcommand)]
  New(NewType),
  /// Print info about inner Deployer's object
  #[command(subcommand)]
  Cat(CatType),
  /// Edit inner Deployer's object
  #[command(subcommand)]
  Edit(EditType),
  /// Remove the inner Deployer's object
  #[command(subcommand)]
  Rm(RemoveType),

  /// Init the deployable project
  Init(InitArgs),
  /// Add Deployer's Pipeline to the project
  With(WithPipelineArgs),
  /// Build the project
  Build(BuildArgs),
  /// Clean the project's builds
  Clean(CleanArgs),

  #[cfg(feature = "tests")]
  Tests,

  /// Read the docs.
  Docs,
}

#[derive(Subcommand)]
pub enum ListType {
  /// List available Actions
  Actions,
  /// List available Pipelines
  Pipelines,
  /// List available content in Deployer's storage
  Content,
  /// List available remote hosts
  Remote,
}

#[derive(Subcommand)]
pub enum RemoveType {
  /// Remove an Action
  Action,
  /// Remove a Pipeline
  Pipeline,
  /// Remove content
  Content,
  /// Remove remote host
  Remote,
}

#[derive(Subcommand)]
pub enum CatType {
  /// Prints an Action
  Action(CatActionArgs),
  /// Prints a Pipeline
  Pipeline(CatPipelineArgs),
  /// Prints all Pipelines used by current Project
  Project(CatProjectArgs),
  /// Prints all information about remote host
  Remote(CatRemoteArgs),
}

#[derive(Subcommand)]
pub enum EditType {
  /// Edits an Action
  Action(CatActionArgs),
  /// Edits a Pipeline
  Pipeline(CatPipelineArgs),
  /// Edits Project settings
  Project,
  /// Edits an information about remote hosts
  Remote(CatRemoteArgs),
}

#[derive(Args)]
pub struct CatActionArgs {
  pub action_short_info_and_version: String,
}

#[derive(Args)]
pub struct CatPipelineArgs {
  pub pipeline_short_info_and_version: String,
}

#[derive(Args)]
pub struct CatProjectArgs {
  #[arg(short('n'), long)]
  pub cat_all_shell_commands: bool,
}

#[derive(Args)]
pub struct CatRemoteArgs {
  pub remote_host_short_info: String,
}

#[derive(Subcommand)]
pub enum NewType {
  /// Add new Action to Deployer's registry
  Action(NewActionArgs),
  /// Add new Pipeline to Deployer's registry
  Pipeline(NewPipelineArgs),
  /// Add new Content to Deployer's storage
  Content,
  /// Add new remote host to Deployer's registry
  Remote(NewRemoteArgs),
}

#[derive(Args)]
pub struct NewActionArgs {
  /// From description in JSON
  #[arg(short, long)]
  pub from: Option<String>,
}

pub type NewPipelineArgs = NewActionArgs;

#[derive(Args)]
pub struct NewRemoteArgs {
  #[arg(short, long)]
  pub short_name: Option<String>,
  #[arg(short, long)]
  pub ip: Option<std::net::IpAddr>,
  #[arg(short, long)]
  pub port: Option<u16>,
  #[arg(short, long)]
  pub username: Option<String>,
  #[arg(short, long)]
  pub ssh_key_path: Option<PathBuf>,
}

#[derive(Args)]
pub struct InitArgs {}

#[derive(Args)]
pub struct WithPipelineArgs {
  /// {short-name}@{version}
  pub tag: Option<String>,
  /// {short-name}
  #[arg(short, long)]
  pub r#as: Option<String>,
}

#[derive(Args)]
pub struct CleanArgs {
  /// Clean current project artifacts
  #[arg(short, long)]
  pub include_artifacts: bool,
}

#[derive(Args)]
pub struct BuildArgs {
  /// {short-name} or {short-name1},{short-name2},..
  #[arg(required = false, value_delimiter(','))]
  pub pipeline_tags: Vec<String>,

  /// Build in current folder
  #[arg(short('j'), long)]
  pub current: bool,
  /// Build in specified folder
  #[arg(short('o'), long)]
  pub build_at: Option<PathBuf>,

  /// Fresh build
  #[arg(short('f'), long)]
  pub fresh: bool,
  /// With symlinking cache
  #[arg(short('c'), long)]
  pub link_cache: bool,
  /// With copying cache
  #[arg(short('C'), long)]
  pub copy_cache: bool,

  /// Build as remote host (as worker)
  #[arg(short('r'), long)]
  pub remote_build_folder: Option<PathBuf>,

  /// Build remotely on specified hosts (as boss-node)
  #[arg(short('R'), long, value_delimiter(','))]
  pub remote_host_short_names: Vec<String>,

  /// Force disable output from Actions
  #[arg(short('s'), long)]
  pub silent: bool,
  /// Don't pipe I/O channels
  #[arg(short('t'), long)]
  pub no_pipe: bool,
}
