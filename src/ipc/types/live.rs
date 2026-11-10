use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LiveProgress {
    pub value: u32,
    pub max: u32,
    pub indeterminate: bool,
}

/// A running clock to animate locally, counting up from or down to `base_unix_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chronometer {
    pub base_unix_ms: u64,
    pub counts_down: bool,
}

/// A button of a live activity, or an inline reply when `accepts_text` is set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LiveAction {
    pub id: String,
    pub label: String,
    pub accepts_text: bool,
}

/// One ongoing activity of a device: a timer, a delivery, a route.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LiveActivity {
    pub id: String,
    pub package: String,
    /// The app's label.
    pub app: String,
    pub title: String,
    pub text: String,
    pub short_text: Option<String>,
    /// The platform's category verbatim, such as `navigation` or `stopwatch`.
    pub category: Option<String>,
    pub progress: Option<LiveProgress>,
    pub chronometer: Option<Chronometer>,
    /// The device ranks it above its other activities.
    pub promoted: bool,
    pub color_argb: Option<u32>,
    pub actions: Vec<LiveAction>,
}
