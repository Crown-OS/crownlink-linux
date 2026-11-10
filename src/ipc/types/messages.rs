use serde::{Deserialize, Serialize};

/// Someone in a conversation. `name` and `contact_key` are only known when the device also
/// shares its contacts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Participant {
    pub address: String,
    pub name: Option<String>,
    pub contact_key: Option<String>,
}

/// One conversation of a device, as a list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Conversation {
    pub thread_id: u64,
    pub participants: Vec<Participant>,
    /// The real number of participants, which `participants` may fall short of.
    pub participant_count: u16,
    pub snippet: String,
    pub last_unix_ms: u64,
    pub unread: u32,
    pub has_attachments: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationPage {
    pub items: Vec<Conversation>,
    /// The cursor of the next page; `None` at the end.
    pub next: Option<String>,
}

/// Which box a message sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageFolder {
    Inbox,
    Sent,
    Draft,
    Outbox,
    Failed,
    Queued,
    /// A box this computer does not know; read it as received or sent by the message's sender.
    Other(u8),
}

/// How far a sent message got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeliveryStatus {
    None,
    Pending,
    Sent,
    Delivered,
    Failed,
    Other(u8),
}

/// A part of a message that is not its text, downloaded on demand with `fetch_blob`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Attachment {
    pub key: String,
    pub mime: String,
    pub name: Option<String>,
    pub len: u64,
    pub thumbnail_key: Option<String>,
    /// `0` when unknown.
    pub width: u16,
    /// `0` when unknown.
    pub height: u16,
}

/// One SMS or MMS.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageItem {
    pub thread_id: u64,
    pub message_id: String,
    pub folder: MessageFolder,
    /// `None` when the device itself sent it.
    pub sender: Option<String>,
    pub unix_ms: u64,
    pub text: Option<String>,
    /// The text was cut short; the whole of it is a blob.
    pub text_truncated: bool,
    pub read: bool,
    pub status: DeliveryStatus,
    pub attachments: Vec<Attachment>,
    /// Attachments the device left out to keep the item small.
    pub attachments_omitted: u16,
    pub sim_slot: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessagePage {
    pub thread_id: u64,
    pub items: Vec<MessageItem>,
    pub next: Option<String>,
}

/// A message arrived, changed or went away on a device.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageChange {
    Added(MessageItem),
    Updated(MessageItem),
    Deleted { thread_id: u64, message_id: String },
}

/// A message to send from a device, into `thread_id` or to `recipients`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageDraft {
    pub thread_id: Option<u64>,
    pub recipients: Vec<String>,
    pub text: String,
    /// Local file paths, streamed to the device after the request.
    pub attachments: Vec<String>,
    pub sim_slot: Option<u8>,
}
