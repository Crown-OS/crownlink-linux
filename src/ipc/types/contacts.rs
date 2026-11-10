use serde::{Deserialize, Serialize};

/// What a contact's number is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhoneKind {
    Home,
    Mobile,
    Work,
    Main,
    /// Any other kind; show the number's `label` when it has one.
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContactPhone {
    pub number: String,
    pub kind: PhoneKind,
    /// The user's own label for a custom kind.
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Contact {
    /// The device's stable key for the contact.
    pub contact_key: String,
    pub name: String,
    pub starred: bool,
    pub phones: Vec<ContactPhone>,
    pub emails: Vec<String>,
    pub organization: Option<String>,
    /// The photo, for `fetch_blob`.
    pub photo_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContactPage {
    pub items: Vec<Contact>,
    pub next: Option<String>,
}
