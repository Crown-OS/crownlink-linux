//! Asking this computer's user to approve something a device asked for, as a notification with
//! buttons from whichever `org.freedesktop.Notifications` server runs (crownotify on CrownOS).
//!
//! Anything but a press of one of the offered buttons is a refusal: dismissing the notification,
//! pressing its body, the timeout, or no notification server at all.

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;
use zbus::zvariant::Value;

use crate::util::markup::escape;

const APP_NAME: &str = "CrownLink";
const APP_ICON: &str = "phone";
/// `urgency=critical` is the one urgency Do Not Disturb never holds back.
const CRITICAL_URGENCY: u8 = 2;
/// The notification stays until it is answered; the prompt's own timeout ends it.
const NEVER_EXPIRES: i32 = 0;
/// A server listing this reads bodies as markup; summaries are always plain text.
const BODY_MARKUP: &str = "body-markup";
pub const DEFAULT_PROMPT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("session bus: {0}")]
    Bus(#[from] zbus::Error),
}

/// One button of a prompt: the key it answers with and the label the user reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptAction {
    pub key: &'static str,
    pub label: &'static str,
}

/// A question for the user. `summary` and `body` are plain text, shown as written.
#[derive(Debug, Clone, Copy)]
pub struct Prompt<'a> {
    pub summary: &'a str,
    pub body: &'a str,
    pub actions: &'a [PromptAction],
    pub timeout: Duration,
}

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments, reason = "the D-Bus method's signature")]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    fn get_capabilities(&self) -> zbus::Result<Vec<String>>;

    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
}

/// Posts prompts and waits for their answers.
#[derive(Debug, Clone)]
pub struct Prompts {
    notifications: NotificationsProxy<'static>,
}

impl Prompts {
    /// # Errors
    ///
    /// Fails when the session bus is unreachable.
    pub async fn connect() -> Result<Self, PromptError> {
        let connection = zbus::Connection::session().await?;
        Ok(Self {
            notifications: NotificationsProxy::new(&connection).await?,
        })
    }

    /// The key of the button the user pressed, or `None` for every kind of refusal.
    ///
    /// # Errors
    ///
    /// Fails when no notification server takes the prompt, which callers treat as a refusal.
    pub async fn ask(&self, prompt: &Prompt<'_>) -> Result<Option<&'static str>, PromptError> {
        let mut invoked = self.notifications.receive_action_invoked().await?;
        let mut closed = self.notifications.receive_notification_closed().await?;
        let actions: Vec<&str> = prompt
            .actions
            .iter()
            .flat_map(|action| [action.key, action.label])
            .collect();
        let hints = HashMap::from([
            ("urgency", Value::U8(CRITICAL_URGENCY)),
            ("resident", Value::Bool(true)),
        ]);
        let body = self.plain_body(prompt.body).await;
        let id = self
            .notifications
            .notify(
                APP_NAME,
                0,
                APP_ICON,
                prompt.summary,
                &body,
                &actions,
                hints,
                NEVER_EXPIRES,
            )
            .await?;
        let answer = async {
            loop {
                tokio::select! {
                    Some(signal) = invoked.next() => {
                        let Ok(args) = signal.args() else { continue };
                        if args.id == id {
                            return prompt
                                .actions
                                .iter()
                                .find(|action| action.key == args.action_key)
                                .map(|action| action.key);
                        }
                    }
                    Some(signal) = closed.next() => {
                        if signal.args().is_ok_and(|args| args.id == id) {
                            return None;
                        }
                    }
                    else => return None,
                }
            }
        };
        match tokio::time::timeout(prompt.timeout, answer).await {
            Ok(answer) => {
                if answer.is_some() {
                    let _ = self.notifications.close_notification(id).await;
                }
                Ok(answer)
            }
            Err(_) => {
                let _ = self.notifications.close_notification(id).await;
                Ok(None)
            }
        }
    }

    /// `body` as the server will show it as written.
    async fn plain_body<'a>(&self, body: &'a str) -> Cow<'a, str> {
        let markup = self
            .notifications
            .get_capabilities()
            .await
            .is_ok_and(|capabilities| capabilities.iter().any(|name| name == BODY_MARKUP));
        if markup {
            escape(body)
        } else {
            Cow::Borrowed(body)
        }
    }
}
