//! Specific PathType module.

/// Path type.
///
/// This struct is used only to enable or disable `safe_path` checks inside `add` & `edit` TUIs.
pub enum PathType {
  Absolute,
  Relative,
}
