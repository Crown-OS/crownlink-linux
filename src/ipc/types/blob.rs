use serde::{Deserialize, Serialize};

/// A blob a device sent, as a file in crownlink's cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobFile {
    pub path: String,
    pub mime: String,
    pub name: Option<String>,
    pub len: u64,
}
