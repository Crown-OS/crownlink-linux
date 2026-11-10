use serde::{Deserialize, Serialize};

use super::{Direction, VideoLimits};

/// How to show one app of a device on this computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CastOptions {
    pub limits: VideoLimits,
}

/// Which way audio plays: `ToPeer` plays this computer's audio on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AudioOptions {
    pub direction: Direction,
}

/// Which end a headset should be connected to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeadsetOwner {
    Computer,
    Device,
}

/// A bonded Bluetooth audio sink.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Headset {
    /// Uppercase `AA:BB:CC:DD:EE:FF`.
    pub address: String,
    pub name: String,
    pub connected: bool,
}

/// The headsets one end could hand over.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AudioRoutes {
    pub headsets: Vec<Headset>,
}
