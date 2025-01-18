//! AddToStorage Action.
//! 
//! Based on automatic rule to extract version from project, allows you to
//! load your artifacts as content to Deployer's storage.

use serde::{Deserialize, Serialize};

use crate::entities::auto_version::AutoVersionExtractFromRule;

/// AddToStorage Action.
#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct AddToStorageAction {
  /// Content's short name. Used to load to and sync from storage.
  pub short_name: String,
  /// Automatic rule to extract version from project.
  pub auto_version_rule: AutoVersionExtractFromRule,
}
