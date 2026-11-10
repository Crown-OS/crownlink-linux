use serde::{Deserialize, Serialize};

use super::{DeviceId, Direction};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransferState {
    Running,
    Finished { path: String },
    Failed(String),
}

/// A file or blob moving between this computer and a device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Transfer {
    pub id: u32,
    pub device: DeviceId,
    pub name: String,
    pub direction: Direction,
    pub done: u64,
    pub total: u64,
    pub state: TransferState,
}
