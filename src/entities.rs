//! Entities module.

pub mod traits;

pub mod info;
pub mod targets;
pub mod custom_command;
pub mod variables;
pub mod programming_languages;
pub mod environment;
pub mod auto_version;
pub mod requirements;
#[cfg(feature = "tui")]
pub mod path_type;
pub mod remote_host;
