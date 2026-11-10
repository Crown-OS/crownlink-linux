use serde::{Deserialize, Serialize};

/// What kind of call a log entry records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallKind {
    Incoming,
    Outgoing,
    Missed,
    Voicemail,
    Rejected,
    Blocked,
    AnsweredElsewhere,
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallLogEntry {
    pub entry_id: String,
    pub number: String,
    /// The name the device cached when the call happened.
    pub name: Option<String>,
    /// Only known when the device also shares its contacts.
    pub contact_key: Option<String>,
    pub kind: CallKind,
    pub start_unix_ms: u64,
    pub duration_s: u32,
    pub is_new: bool,
    pub sim_slot: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallLogPage {
    pub items: Vec<CallLogEntry>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Voicemail {
    pub voicemail_id: String,
    pub number: String,
    pub name: Option<String>,
    pub received_unix_ms: u64,
    pub duration_s: u32,
    pub heard: bool,
    pub transcript: Option<String>,
    pub transcript_truncated: bool,
    /// The recording, for `fetch_blob`; `None` when the device cannot read it.
    pub audio_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoicemailPage {
    pub items: Vec<Voicemail>,
    pub next: Option<String>,
}
