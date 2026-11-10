use crownos_ipc::{MethodMsg, Server};

use super::{Call, Routed};
use crate::ipc::proto;
use crate::ipc::server::{encode, Malformed, Router};
use crate::peer::PeerRequest;

pub(super) fn route(
    router: &mut Router,
    server: &mut Server,
    call: &Call<'_>,
) -> Result<Routed, Malformed> {
    let request = match call.selector {
        <proto::call_log as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::call_log>()?;
            PeerRequest::CallLog {
                id: args.id,
                page: args.page,
                reply: call.responder(router),
            }
        }
        <proto::voicemails as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::voicemails>()?;
            PeerRequest::Voicemails {
                id: args.id,
                page: args.page,
                reply: call.responder(router),
            }
        }
        <proto::contacts as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::contacts>()?;
            PeerRequest::Contacts {
                id: args.id,
                page: args.page,
                reply: call.responder(router),
            }
        }
        <proto::collections as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::collections>()?;
            let collections = router.state.collections(args.id);
            call.reply(
                server,
                collections.and_then(|collections| encode(&collections)),
            );
            return Ok(Routed::Handled);
        }
        _ => return Ok(Routed::Elsewhere),
    };
    router.forward(request);
    Ok(Routed::Handled)
}
