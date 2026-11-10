use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FocusMode {
    Off,
    DoNotDisturb,
    Named(String),
}

/// A device's Do Not Disturb or focus mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FocusInfo {
    pub mode: FocusMode,
    /// When it ends on its own; `None` until turned off.
    pub until_unix_ms: Option<u64>,
}
