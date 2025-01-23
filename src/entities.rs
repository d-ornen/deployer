//! Entities module.

pub mod traits;

pub mod auto_version;
pub mod custom_command;
pub mod environment;
pub mod info;
#[cfg(feature = "tui")]
pub mod path_type;
pub mod programming_languages;
pub mod remote_host;
pub mod requirements;
pub mod targets;
pub mod variables;
