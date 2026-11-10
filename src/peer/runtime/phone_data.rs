//! Asking a phone for its messages, call history and contacts, sending messages from it, and
//! opening links on it.

use llts_signaling::device::DeviceId;
use llts_signaling::message::{
    ListCallLog, ListContacts, ListConversations, ListMessages, ListVoicemail, MarkRead, OpenUri,
    Signal,
};

use super::event_loop::PeerLoop;
use super::pending::{Waiting, CHANGE_TIMEOUT, PAGE_TIMEOUT};
use crate::features::messages::{check_draft, send_request};
use crate::ipc::proto::Feature;
use crate::ipc::server::Responder;
use crate::peer::convert::{signaling_id, signaling_page};
use crate::peer::PeerRequest;
use crate::util::uri_scheme::check;

/// Why a blob download cannot start: the chunked, acknowledged blob lane is not in llts yet.
fn blob_lane_missing(feature: Feature) -> String {
    format!(
        "downloading {feature:?} items needs the llts blob lane, which this build of crownlink \
         and the device's llts library do not have yet"
    )
}

impl PeerLoop {
    /// Serves the requests of the Messages, call history, contacts and links features.
    pub(super) fn on_phone_data_request(&mut self, request: PeerRequest) {
        match request {
            PeerRequest::Conversations { id, page, reply } => self.ask(
                signaling_id(id),
                Waiting::Conversations(reply),
                PAGE_TIMEOUT,
                |request| {
                    Signal::ListConversationsV1(ListConversations {
                        request,
                        page: signaling_page(&page),
                    })
                },
            ),
            PeerRequest::Messages {
                id,
                thread_id,
                page,
                reply,
            } => self.ask(
                signaling_id(id),
                Waiting::Messages { thread_id, reply },
                PAGE_TIMEOUT,
                |request| {
                    Signal::ListMessagesV1(ListMessages {
                        request,
                        thread_id,
                        page: signaling_page(&page),
                    })
                },
            ),
            PeerRequest::SendMessage { id, draft, reply } => {
                let peer = signaling_id(id);
                match check_draft(&draft, self.collections.get(&peer).messages) {
                    Ok(()) => self.ask(
                        peer,
                        Waiting::SendMessage(reply),
                        CHANGE_TIMEOUT,
                        |request| Signal::SendMessageV1(send_request(request, &draft)),
                    ),
                    Err(error) => reply.fail(error.to_string()),
                }
            }
            PeerRequest::MarkRead {
                id,
                thread_id,
                reply,
            } => self.ask(
                signaling_id(id),
                Waiting::MarkRead(reply),
                CHANGE_TIMEOUT,
                |request| Signal::MarkReadV1(MarkRead { request, thread_id }),
            ),
            PeerRequest::CallLog { id, page, reply } => self.ask(
                signaling_id(id),
                Waiting::CallLog(reply),
                PAGE_TIMEOUT,
                |request| {
                    Signal::ListCallLogV1(ListCallLog {
                        request,
                        page: signaling_page(&page),
                    })
                },
            ),
            PeerRequest::Voicemails { id, page, reply } => self.ask(
                signaling_id(id),
                Waiting::Voicemails(reply),
                PAGE_TIMEOUT,
                |request| {
                    Signal::ListVoicemailV1(ListVoicemail {
                        request,
                        page: signaling_page(&page),
                    })
                },
            ),
            PeerRequest::Contacts { id, page, reply } => self.ask(
                signaling_id(id),
                Waiting::Contacts(reply),
                PAGE_TIMEOUT,
                |request| {
                    Signal::ListContactsV1(ListContacts {
                        request,
                        page: signaling_page(&page),
                    })
                },
            ),
            PeerRequest::FetchBlob { feature, reply, .. } => reply.fail(blob_lane_missing(feature)),
            PeerRequest::OpenOnDevice { id, uri, reply } => {
                self.open_on_device(signaling_id(id), &uri, reply);
            }
            other => Self::refuse_unimplemented(other),
        }
    }

    fn open_on_device(&mut self, peer: DeviceId, uri: &str, reply: Responder<()>) {
        let outcome = check(uri)
            .map_err(|error| error.to_string())
            .and_then(|uri| self.send_signal(peer, &Signal::OpenUriV1(OpenUri { uri: &uri })));
        Self::respond(reply, outcome);
    }
}
