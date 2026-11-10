use serde::{Deserialize, Serialize};

/// Where a device was at its last position fix.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeviceLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: u32,
    pub altitude_m: Option<i32>,
    pub fix_unix_ms: u64,
}
