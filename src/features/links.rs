//! Links a device asks this computer to open: checked against the same rules as outgoing ones,
//! shown to the user, and opened with `xdg-open` only when the user presses Open.

use std::process::{Command, Stdio};

use tokio::sync::mpsc;

use crate::desktop::{Prompt, PromptAction, PromptError, Prompts, DEFAULT_PROMPT_TIMEOUT};
use crate::util::uri_scheme::{check, UriError};

const OPENER: &str = "xdg-open";
const OPEN: PromptAction = PromptAction {
    key: "open",
    label: "Open",
};
const IGNORE: PromptAction = PromptAction {
    key: "ignore",
    label: "Ignore",
};
/// Links waiting for the user; more than this from a device are dropped, not queued.
pub const LINK_QUEUE_DEPTH: usize = 4;

/// A link a device asked to open, already checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingLink {
    pub device: String,
    uri: String,
}

impl IncomingLink {
    /// # Errors
    ///
    /// Fails for a link outside the allowed schemes or over the length cap.
    pub fn new(device: impl Into<String>, uri: &str) -> Result<Self, UriError> {
        Ok(Self {
            device: device.into(),
            uri: check(uri)?.into_owned(),
        })
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }
}

/// Asks about each incoming link in turn until `links` closes.
///
/// # Errors
///
/// Fails when the session bus is unreachable, so no link can be asked about.
pub async fn serve(mut links: mpsc::Receiver<IncomingLink>) -> Result<(), PromptError> {
    let prompts = Prompts::connect().await?;
    while let Some(link) = links.recv().await {
        let summary = format!("{} wants to open a link", link.device);
        let body = format!("Open {}?", link.uri);
        let prompt = Prompt {
            summary: &summary,
            body: &body,
            actions: &[OPEN, IGNORE],
            timeout: DEFAULT_PROMPT_TIMEOUT,
        };
        match prompts.ask(&prompt).await {
            Ok(Some(key)) if key == OPEN.key => open(link).await,
            Ok(_) => tracing::debug!(device = link.device, "the user did not open a link"),
            Err(error) => {
                tracing::warn!(%error, device = link.device, "cannot ask about a link; ignoring it");
            }
        }
    }
    Ok(())
}

async fn open(link: IncomingLink) {
    let device = link.device;
    let uri = link.uri;
    let status = tokio::task::spawn_blocking(move || {
        Command::new(OPENER)
            .arg(uri)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    })
    .await;
    match status {
        Ok(Ok(status)) if status.success() => tracing::info!(device, "opened a link"),
        Ok(Ok(status)) => tracing::warn!(device, %status, "{OPENER} could not open a link"),
        Ok(Err(error)) => tracing::warn!(device, %error, "cannot run {OPENER}"),
        Err(error) => tracing::warn!(device, %error, "the link opener stopped"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_allowed_links_are_accepted() {
        assert!(IncomingLink::new("Pixel", "file:///etc/passwd").is_err());
        assert_eq!(
            IncomingLink::new("Pixel", "mailto:a@b.c?attach=/x").map(|link| link.uri().to_owned()),
            Ok("mailto:a@b.c".to_owned())
        );
    }
}
