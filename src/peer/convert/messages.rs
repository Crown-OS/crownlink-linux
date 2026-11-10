use llts_signaling::message as wire;

use crate::ipc::proto::{
    Attachment, Conversation, ConversationPage, DeliveryStatus, MessageChange, MessageFolder,
    MessageItem, MessagePage, Participant,
};

const fn ipc_folder(folder: wire::MessageFolder) -> MessageFolder {
    match folder {
        wire::MessageFolder::INBOX => MessageFolder::Inbox,
        wire::MessageFolder::SENT => MessageFolder::Sent,
        wire::MessageFolder::DRAFT => MessageFolder::Draft,
        wire::MessageFolder::OUTBOX => MessageFolder::Outbox,
        wire::MessageFolder::FAILED => MessageFolder::Failed,
        wire::MessageFolder::QUEUED => MessageFolder::Queued,
        wire::MessageFolder(other) => MessageFolder::Other(other),
    }
}

const fn ipc_status(status: wire::DeliveryStatus) -> DeliveryStatus {
    match status {
        wire::DeliveryStatus::NONE => DeliveryStatus::None,
        wire::DeliveryStatus::PENDING => DeliveryStatus::Pending,
        wire::DeliveryStatus::SENT => DeliveryStatus::Sent,
        wire::DeliveryStatus::DELIVERED => DeliveryStatus::Delivered,
        wire::DeliveryStatus::FAILED => DeliveryStatus::Failed,
        wire::DeliveryStatus(other) => DeliveryStatus::Other(other),
    }
}

fn ipc_participant(participant: &wire::Participant<'_>) -> Participant {
    Participant {
        address: participant.address.to_owned(),
        name: participant.name.map(str::to_owned),
        contact_key: participant.contact_key.map(str::to_owned),
    }
}

fn ipc_conversation(conversation: &wire::Conversation<'_>) -> Conversation {
    Conversation {
        thread_id: conversation.thread_id,
        participants: conversation
            .participants
            .iter()
            .map(ipc_participant)
            .collect(),
        participant_count: conversation.participant_count,
        snippet: conversation.snippet.to_owned(),
        last_unix_ms: conversation.last_unix_ms,
        unread: conversation.unread,
        has_attachments: conversation.has_attachments,
    }
}

pub(crate) fn ipc_conversation_page(page: &wire::ConversationPage<'_>) -> ConversationPage {
    ConversationPage {
        items: page.items.iter().map(ipc_conversation).collect(),
        next: page.next.map(str::to_owned),
    }
}

fn ipc_attachment(attachment: &wire::Attachment<'_>) -> Attachment {
    Attachment {
        key: attachment.key.to_owned(),
        mime: attachment.mime.to_owned(),
        name: attachment.name.map(str::to_owned),
        len: attachment.len,
        thumbnail_key: attachment.thumbnail_key.map(str::to_owned),
        width: attachment.width,
        height: attachment.height,
    }
}

fn ipc_message(item: &wire::MessageItem<'_>) -> MessageItem {
    MessageItem {
        thread_id: item.thread_id,
        message_id: item.message_id.to_owned(),
        folder: ipc_folder(item.folder),
        sender: item.sender.map(str::to_owned),
        unix_ms: item.unix_ms,
        text: item.text.map(str::to_owned),
        text_truncated: item.text_truncated,
        read: item.read,
        status: ipc_status(item.status),
        attachments: item.attachments.iter().map(ipc_attachment).collect(),
        attachments_omitted: item.attachments_omitted,
        sim_slot: item.sim_slot,
    }
}

pub(crate) fn ipc_message_page(page: &wire::MessagePage<'_>) -> MessagePage {
    MessagePage {
        thread_id: page.thread_id,
        items: page.items.iter().map(ipc_message).collect(),
        next: page.next.map(str::to_owned),
    }
}

pub(crate) fn ipc_message_change(change: &wire::MessageChange<'_>) -> MessageChange {
    match change {
        wire::MessageChange::Added(item) => MessageChange::Added(ipc_message(item)),
        wire::MessageChange::Updated(item) => MessageChange::Updated(ipc_message(item)),
        wire::MessageChange::Deleted {
            thread_id,
            message_id,
        } => MessageChange::Deleted {
            thread_id: *thread_id,
            message_id: (*message_id).to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire_item(folder: u8, status: u8) -> wire::MessageItem<'static> {
        wire::MessageItem {
            thread_id: 9,
            message_id: "mms/12",
            folder: wire::MessageFolder(folder),
            sender: Some("+15550100"),
            unix_ms: 1_700_000_000_000,
            text: Some("photo"),
            text_truncated: false,
            read: true,
            status: wire::DeliveryStatus(status),
            attachments: vec![wire::Attachment {
                key: "mms-part/3/1700000000000",
                mime: "image/jpeg",
                name: Some("a.jpg"),
                len: 2_048,
                thumbnail_key: Some("mms-thumb/3/1700000000000/256"),
                width: 640,
                height: 480,
            }],
            attachments_omitted: 2,
            sim_slot: Some(1),
        }
    }

    #[test]
    fn a_message_page_keeps_every_field() {
        let page = wire::MessagePage {
            request: wire::RequestId(1),
            thread_id: 9,
            items: vec![wire_item(1, 0)],
            next: Some("1700000000000:m:12"),
        };
        let expected = MessagePage {
            thread_id: 9,
            items: vec![MessageItem {
                thread_id: 9,
                message_id: "mms/12".into(),
                folder: MessageFolder::Inbox,
                sender: Some("+15550100".into()),
                unix_ms: 1_700_000_000_000,
                text: Some("photo".into()),
                text_truncated: false,
                read: true,
                status: DeliveryStatus::None,
                attachments: vec![Attachment {
                    key: "mms-part/3/1700000000000".into(),
                    mime: "image/jpeg".into(),
                    name: Some("a.jpg".into()),
                    len: 2_048,
                    thumbnail_key: Some("mms-thumb/3/1700000000000/256".into()),
                    width: 640,
                    height: 480,
                }],
                attachments_omitted: 2,
                sim_slot: Some(1),
            }],
            next: Some("1700000000000:m:12".into()),
        };
        assert_eq!(ipc_message_page(&page), expected);
    }

    #[test]
    fn every_folder_and_status_maps_and_unknown_ones_survive() {
        let folders = [
            MessageFolder::Inbox,
            MessageFolder::Sent,
            MessageFolder::Draft,
            MessageFolder::Outbox,
            MessageFolder::Failed,
            MessageFolder::Queued,
        ];
        for (raw, folder) in (1..).zip(folders) {
            assert_eq!(ipc_folder(wire::MessageFolder(raw)), folder);
        }
        assert_eq!(
            ipc_folder(wire::MessageFolder(42)),
            MessageFolder::Other(42)
        );
        let statuses = [
            DeliveryStatus::None,
            DeliveryStatus::Pending,
            DeliveryStatus::Sent,
            DeliveryStatus::Delivered,
            DeliveryStatus::Failed,
        ];
        for (raw, status) in (0..).zip(statuses) {
            assert_eq!(ipc_status(wire::DeliveryStatus(raw)), status);
        }
        assert_eq!(
            ipc_status(wire::DeliveryStatus(9)),
            DeliveryStatus::Other(9)
        );
    }

    #[test]
    fn conversations_and_changes_cross_over() {
        let page = wire::ConversationPage {
            request: wire::RequestId(2),
            items: vec![wire::Conversation {
                thread_id: 4,
                participants: vec![wire::Participant {
                    address: "+15550100",
                    name: None,
                    contact_key: None,
                }],
                participant_count: 3,
                snippet: "See you",
                last_unix_ms: 5,
                unread: 1,
                has_attachments: true,
            }],
            next: None,
        };
        let converted = ipc_conversation_page(&page);
        assert_eq!(converted.next, None);
        assert_eq!(
            converted.items,
            [Conversation {
                thread_id: 4,
                participants: vec![Participant {
                    address: "+15550100".into(),
                    name: None,
                    contact_key: None,
                }],
                participant_count: 3,
                snippet: "See you".into(),
                last_unix_ms: 5,
                unread: 1,
                has_attachments: true,
            }]
        );
        assert_eq!(
            ipc_message_change(&wire::MessageChange::Deleted {
                thread_id: 4,
                message_id: "sms/1",
            }),
            MessageChange::Deleted {
                thread_id: 4,
                message_id: "sms/1".into(),
            }
        );
        assert!(matches!(
            ipc_message_change(&wire::MessageChange::Updated(wire_item(2, 3))),
            MessageChange::Updated(MessageItem {
                folder: MessageFolder::Sent,
                status: DeliveryStatus::Delivered,
                ..
            })
        ));
    }
}
