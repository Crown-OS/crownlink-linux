//! Application messages: requests, pages and pushes the node routes without reading.
//!
//! Logs name only the message type, the peer and the request id, never a body: these carry
//! messages, contacts, credentials and locations.

use llts_signaling::device::{DeviceId, FeatureSet};
use llts_signaling::message::{
    MessageType, RequestDone, RequestError, RequestFailed, RequestId, Signal,
};

use super::event_loop::PeerLoop;
use super::pending::{Gated, Waiting};
use crate::features::links::IncomingLink;
use crate::ipc::proto::Collections;
use crate::ipc::server::DaemonEvent;
use crate::peer::convert::{
    ipc_call_counters, ipc_call_log_page, ipc_contact_counters, ipc_contact_page,
    ipc_conversation_page, ipc_id, ipc_message_change, ipc_message_counters, ipc_message_page,
    ipc_voicemail_page, request_failure_reason,
};

/// What this computer answers a phone-data request from a peer: it has no SMS, call log or
/// address book of its own.
const NO_PHONE_DATA: &str = "a computer keeps no messages, call history or contacts";

fn unanswered(kind: MessageType, peer: DeviceId, request: RequestId) {
    tracing::debug!(?kind, %peer, request = request.0, "an answer to no pending request");
}

impl PeerLoop {
    pub(super) fn on_application(&mut self, peer: DeviceId, message: &[u8]) {
        let signal = match Signal::decode(message) {
            Ok(signal) => signal,
            Err(error) => {
                tracing::debug!(%peer, %error, "dropping a malformed application message");
                return;
            }
        };
        if let Err(error) = signal.validate() {
            tracing::debug!(
                kind = ?signal.message_type(),
                %peer,
                %error,
                "dropping an application message that breaks its caps"
            );
            return;
        }
        match signal {
            Signal::RequestFailedV1(failed) => self.on_request_failed(peer, &failed),
            Signal::RequestDoneV1(done) => self.on_request_done(peer, done),
            Signal::ConversationPageV1(page) => match self.requests.take_if(
                peer,
                page.request,
                |waiting| matches!(waiting, Waiting::Conversations(_)),
            ) {
                Some(Waiting::Conversations(reply)) => reply.reply(ipc_conversation_page(&page)),
                _ => unanswered(MessageType::ConversationPageV1, peer, page.request),
            },
            Signal::MessagePageV1(page) => match self.requests.take_if(
                peer,
                page.request,
                |waiting| {
                    matches!(waiting, Waiting::Messages { thread_id, .. } if *thread_id == page.thread_id)
                },
            ) {
                Some(Waiting::Messages { reply, .. }) => reply.reply(ipc_message_page(&page)),
                _ => unanswered(MessageType::MessagePageV1, peer, page.request),
            },
            Signal::CallLogPageV1(page) => match self.requests.take_if(
                peer,
                page.request,
                |waiting| matches!(waiting, Waiting::CallLog(_)),
            ) {
                Some(Waiting::CallLog(reply)) => reply.reply(ipc_call_log_page(&page)),
                _ => unanswered(MessageType::CallLogPageV1, peer, page.request),
            },
            Signal::VoicemailPageV1(page) => match self.requests.take_if(
                peer,
                page.request,
                |waiting| matches!(waiting, Waiting::Voicemails(_)),
            ) {
                Some(Waiting::Voicemails(reply)) => reply.reply(ipc_voicemail_page(&page)),
                _ => unanswered(MessageType::VoicemailPageV1, peer, page.request),
            },
            Signal::ContactPageV1(page) => match self.requests.take_if(
                peer,
                page.request,
                |waiting| matches!(waiting, Waiting::Contacts(_)),
            ) {
                Some(Waiting::Contacts(reply)) => reply.reply(ipc_contact_page(&page)),
                _ => unanswered(MessageType::ContactPageV1, peer, page.request),
            },
            Signal::MessageEventV1(event) => self.announce(DaemonEvent::MessageChanged {
                id: ipc_id(peer),
                change: ipc_message_change(&event.change),
            }),
            Signal::MessageCountersV1(counters) => self.update_collections(peer, |collections| {
                collections.messages = Some(ipc_message_counters(counters));
            }),
            Signal::CallCountersV1(counters) => self.update_collections(peer, |collections| {
                collections.calls = Some(ipc_call_counters(counters));
            }),
            Signal::ContactCountersV1(counters) => self.update_collections(peer, |collections| {
                collections.contacts = Some(ipc_contact_counters(counters));
            }),
            Signal::OpenUriV1(open) => self.on_open_uri(peer, open.uri),
            Signal::ListConversationsV1(list) => self.refuse_phone_data(peer, list.request),
            Signal::ListMessagesV1(list) => self.refuse_phone_data(peer, list.request),
            Signal::SendMessageV1(send) => self.refuse_phone_data(peer, send.request),
            Signal::MarkReadV1(mark) => self.refuse_phone_data(peer, mark.request),
            Signal::ListCallLogV1(list) => self.refuse_phone_data(peer, list.request),
            Signal::ListVoicemailV1(list) => self.refuse_phone_data(peer, list.request),
            Signal::ListContactsV1(list) => self.refuse_phone_data(peer, list.request),
            other => tracing::debug!(
                kind = ?other.message_type(),
                %peer,
                "no feature of this build handles the application message"
            ),
        }
    }

    fn on_request_failed(&mut self, peer: DeviceId, failed: &RequestFailed<'_>) {
        match self.requests.take_if(peer, failed.request, |_| true) {
            Some(waiting) => waiting.fail(request_failure_reason(failed)),
            None => unanswered(MessageType::RequestFailedV1, peer, failed.request),
        }
    }

    fn on_request_done(&mut self, peer: DeviceId, done: RequestDone<'_>) {
        let waiting = self.requests.take_if(peer, done.request, |waiting| {
            matches!(waiting, Waiting::SendMessage(_) | Waiting::MarkRead(_))
        });
        match waiting {
            Some(Waiting::SendMessage(reply)) => reply.reply(
                done.reference
                    .map_or_else(|| format!("pending/{}", done.request.0), str::to_owned),
            ),
            Some(Waiting::MarkRead(reply)) => reply.reply(()),
            _ => unanswered(MessageType::RequestDoneV1, peer, done.request),
        }
    }

    fn update_collections(&mut self, peer: DeviceId, change: impl FnOnce(&mut Collections)) {
        if let Some(collections) = self.collections.update(peer, change) {
            self.announce(DaemonEvent::Collections {
                id: ipc_id(peer),
                collections,
            });
        }
    }

    fn refuse_phone_data(&mut self, peer: DeviceId, request: RequestId) {
        let refusal = Signal::RequestFailedV1(RequestFailed {
            request,
            error: RequestError::UNSUPPORTED,
            detail: NO_PHONE_DATA,
        });
        if let Err(reason) = self.send_signal(peer, &refusal) {
            tracing::debug!(%peer, request = request.0, reason, "cannot refuse a request");
        }
    }

    fn on_open_uri(&self, peer: DeviceId, uri: &str) {
        let device = self
            .node
            .device(&peer)
            .map(|device| device.name.to_owned())
            .unwrap_or_default();
        match IncomingLink::new(device, uri) {
            Ok(link) => {
                if let Err(error) = self.services.incoming_links.try_send(link) {
                    tracing::warn!(%peer, %error, "dropping a link the user cannot be asked about");
                }
            }
            Err(error) => tracing::debug!(%peer, %error, "refusing a link from a device"),
        }
    }

    /// Ends what a device can no longer be asked: requests of features it stopped sharing, and
    /// the counters of those features.
    pub(super) fn forget_unshared(&mut self) {
        let shared = |node: &llts_node::Node, peer: &DeviceId| {
            node.device(peer).map(|device| device.available)
        };
        for abandoned in self
            .requests
            .unshared(|peer| shared(&self.node, &peer).unwrap_or(FeatureSet::EMPTY))
        {
            let reason = match self.node.device(&abandoned.peer) {
                None => "the device was forgotten".to_owned(),
                Some(device) if !device.connected => "the device disconnected".to_owned(),
                Some(_) => format!(
                    "{:?} is no longer shared with the device",
                    abandoned.waiting.feature()
                ),
            };
            abandoned.waiting.fail(reason);
        }
        for (peer, collections) in self
            .collections
            .retain_shared(|peer| shared(&self.node, peer))
        {
            self.announce(DaemonEvent::Collections {
                id: ipc_id(peer),
                collections,
            });
        }
    }
}
