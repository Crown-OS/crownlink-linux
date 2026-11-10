use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScanFormat {
    Pdf,
    Jpeg,
}

/// What the device's user is asked to capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptureKind {
    Photo,
    Scan(ScanFormat),
    Sketch,
}

/// Where a capture goes once it arrives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptureDestination {
    /// Into the window focused when the capture was asked for.
    Paste,
    /// Saved into this directory.
    Folder(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub kind: CaptureKind,
    pub destination: CaptureDestination,
}

/// Where a capture stands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptureStatus {
    Pending,
    Pasted {
        path: String,
        mime: String,
    },
    /// The target window lost focus, so the result waits on the clipboard.
    OnClipboard {
        path: String,
        mime: String,
    },
    Saved {
        path: String,
        mime: String,
    },
    Cancelled,
    Failed(String),
}
