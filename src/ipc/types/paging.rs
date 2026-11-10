use serde::{Deserialize, Serialize};

/// Where a page of a device's collection starts and how long it may be.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageRequest {
    /// The previous page's `next`; `None` asks for the first page.
    pub cursor: Option<String>,
    /// The device clamps it to its own page cap.
    pub limit: u16,
}
