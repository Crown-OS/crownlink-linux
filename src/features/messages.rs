//! Sending a message from a device: the checks a draft passes on this computer before the
//! request leaves, so a draft the device would refuse fails at once with a reason.

use llts_signaling::message::limits::{MAX_PARTICIPANTS, MAX_SEND_TEXT_BYTES};
use llts_signaling::message::{RequestId, SendMessage};

use crate::ipc::proto::{MessageCounters, MessageDraft};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DraftError {
    #[error("a message needs a conversation or at least one recipient")]
    NoRecipients,
    #[error("a recipient is empty")]
    BlankRecipient,
    #[error("a message can go to at most {MAX_PARTICIPANTS} recipients, not {0}")]
    TooManyRecipients(usize),
    #[error("a message has no text")]
    Empty,
    #[error("a {0}-byte text exceeds the {MAX_SEND_TEXT_BYTES}-byte limit")]
    TextTooLong(usize),
    #[error(
        "attachments need the llts blob lane, which this build of crownlink and the device's \
         llts library do not have yet"
    )]
    AttachmentsUnavailable,
    #[error("the device cannot send text messages")]
    CannotSend,
}

/// Checks `draft` against the wire caps and what the device said it can do (`None` until its
/// counters arrived).
///
/// # Errors
///
/// Returns the first rule the draft breaks.
pub fn check_draft(
    draft: &MessageDraft,
    counters: Option<MessageCounters>,
) -> Result<(), DraftError> {
    if draft.thread_id.is_none() && draft.recipients.is_empty() {
        return Err(DraftError::NoRecipients);
    }
    if draft.recipients.len() > MAX_PARTICIPANTS {
        return Err(DraftError::TooManyRecipients(draft.recipients.len()));
    }
    if draft
        .recipients
        .iter()
        .any(|recipient| recipient.trim().is_empty())
    {
        return Err(DraftError::BlankRecipient);
    }
    if !draft.attachments.is_empty() {
        return Err(DraftError::AttachmentsUnavailable);
    }
    if draft.text.trim().is_empty() {
        return Err(DraftError::Empty);
    }
    if draft.text.len() > MAX_SEND_TEXT_BYTES {
        return Err(DraftError::TextTooLong(draft.text.len()));
    }
    if counters.is_some_and(|counters| !counters.can(MessageCounters::SEND_SMS)) {
        return Err(DraftError::CannotSend);
    }
    Ok(())
}

/// The request that sends a checked `draft`.
pub fn send_request(request: RequestId, draft: &MessageDraft) -> SendMessage<'_> {
    SendMessage {
        request,
        thread_id: draft.thread_id,
        recipients: draft.recipients.iter().map(String::as_str).collect(),
        text: &draft.text,
        attachments: Vec::new(),
        sim_slot: draft.sim_slot,
    }
}

#[cfg(test)]
mod tests {
    use llts_signaling::message::Validate;

    use super::*;

    fn draft() -> MessageDraft {
        MessageDraft {
            thread_id: None,
            recipients: vec!["+15550100".to_owned()],
            text: "On my way".to_owned(),
            attachments: Vec::new(),
            sim_slot: None,
        }
    }

    const SMS_ONLY: MessageCounters = MessageCounters {
        revision: 1,
        unread: 0,
        capabilities: MessageCounters::SEND_SMS,
        mms_max_bytes: 0,
    };

    #[test]
    fn a_plain_text_passes_and_encodes_within_the_caps() {
        let draft = draft();
        assert_eq!(check_draft(&draft, Some(SMS_ONLY)), Ok(()));
        assert_eq!(check_draft(&draft, None), Ok(()));
        let request = send_request(RequestId(7), &draft);
        assert!(request.validate().is_ok());
        assert_eq!(request.recipients, ["+15550100"]);
    }

    #[test]
    fn a_reply_into_a_thread_needs_no_recipients() {
        let reply = MessageDraft {
            thread_id: Some(4),
            recipients: Vec::new(),
            ..draft()
        };
        assert_eq!(check_draft(&reply, None), Ok(()));
    }

    #[test]
    fn broken_drafts_fail_with_their_reason() {
        let cases = [
            (
                MessageDraft {
                    recipients: Vec::new(),
                    ..draft()
                },
                DraftError::NoRecipients,
            ),
            (
                MessageDraft {
                    recipients: vec![" ".to_owned()],
                    ..draft()
                },
                DraftError::BlankRecipient,
            ),
            (
                MessageDraft {
                    recipients: vec!["1".to_owned(); MAX_PARTICIPANTS + 1],
                    ..draft()
                },
                DraftError::TooManyRecipients(MAX_PARTICIPANTS + 1),
            ),
            (
                MessageDraft {
                    text: String::new(),
                    ..draft()
                },
                DraftError::Empty,
            ),
            (
                MessageDraft {
                    text: "x".repeat(MAX_SEND_TEXT_BYTES + 1),
                    ..draft()
                },
                DraftError::TextTooLong(MAX_SEND_TEXT_BYTES + 1),
            ),
            (
                MessageDraft {
                    attachments: vec!["/tmp/photo.jpg".to_owned()],
                    ..draft()
                },
                DraftError::AttachmentsUnavailable,
            ),
        ];
        for (draft, error) in cases {
            assert_eq!(check_draft(&draft, None), Err(error));
        }
    }

    #[test]
    fn a_device_that_cannot_send_refuses_at_once() {
        let mute = MessageCounters {
            capabilities: MessageCounters::MARK_READ,
            ..SMS_ONLY
        };
        assert_eq!(
            check_draft(&draft(), Some(mute)),
            Err(DraftError::CannotSend)
        );
    }
}
