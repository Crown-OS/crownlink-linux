use serde::{Deserialize, Serialize};

/// A device's messages at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageCounters {
    /// Changes whenever any conversation does, so pages fetched before are stale.
    pub revision: u64,
    pub unread: u32,
    pub capabilities: u32,
    /// The carrier's MMS size limit; `0` when unknown.
    pub mms_max_bytes: u32,
}

impl MessageCounters {
    pub const SEND_SMS: u32 = 1 << 0;
    pub const SEND_MMS: u32 = 1 << 1;
    pub const MARK_READ: u32 = 1 << 2;

    #[must_use]
    pub const fn can(self, capability: u32) -> bool {
        self.capabilities & capability == capability
    }
}

/// A device's call log and voicemail at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallCounters {
    pub call_log_revision: u64,
    pub voicemail_revision: u64,
    pub missed: u32,
    pub unheard_voicemails: u32,
    pub capabilities: u32,
}

impl CallCounters {
    pub const VOICEMAIL_AUDIO: u32 = 1 << 0;
    pub const VOICEMAIL_TRANSCRIPTS: u32 = 1 << 1;

    #[must_use]
    pub const fn can(self, capability: u32) -> bool {
        self.capabilities & capability == capability
    }
}

/// A device's contacts at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContactCounters {
    pub revision: u64,
    pub count: u32,
    pub capabilities: u32,
}

impl ContactCounters {
    pub const PHOTOS: u32 = 1 << 0;

    #[must_use]
    pub const fn can(self, capability: u32) -> bool {
        self.capabilities & capability == capability
    }
}

/// What a device last said about each of its collections. A field is `None` until the device
/// published it and whenever its feature is not shared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Collections {
    pub messages: Option<MessageCounters>,
    pub calls: Option<CallCounters>,
    pub contacts: Option<ContactCounters>,
}
