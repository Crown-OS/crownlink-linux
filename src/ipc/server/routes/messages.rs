use crownos_ipc::{MethodMsg, Server};

use super::{Call, Routed};
use crate::ipc::proto;
use crate::ipc::server::{Malformed, Router};
use crate::peer::PeerRequest;

pub(super) fn route(
    router: &mut Router,
    _server: &mut Server,
    call: &Call<'_>,
) -> Result<Routed, Malformed> {
    let request = match call.selector {
        <proto::conversations as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::conversations>()?;
            PeerRequest::Conversations {
                id: args.id,
                page: args.page,
                reply: call.responder(router),
            }
        }
        <proto::messages as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::messages>()?;
            PeerRequest::Messages {
                id: args.id,
                thread_id: args.thread_id,
                page: args.page,
                reply: call.responder(router),
            }
        }
        <proto::send_message as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::send_message>()?;
            PeerRequest::SendMessage {
                id: args.id,
                draft: args.draft,
                reply: call.responder(router),
            }
        }
        <proto::mark_read as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::mark_read>()?;
            PeerRequest::MarkRead {
                id: args.id,
                thread_id: args.thread_id,
                reply: call.responder(router),
            }
        }
        <proto::fetch_blob as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::fetch_blob>()?;
            PeerRequest::FetchBlob {
                id: args.id,
                feature: args.feature,
                key: args.key,
                reply: call.responder(router),
            }
        }
        _ => return Ok(Routed::Elsewhere),
    };
    router.forward(request);
    Ok(Routed::Handled)
}
