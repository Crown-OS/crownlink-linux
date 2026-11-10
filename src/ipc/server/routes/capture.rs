use crownos_ipc::{MethodMsg, Server};

use super::{unavailable, Call, Routed};
use crate::ipc::proto;
use crate::ipc::server::{Malformed, Router};
use crate::peer::PeerRequest;

pub(super) fn route(
    router: &mut Router,
    server: &mut Server,
    call: &Call<'_>,
) -> Result<Routed, Malformed> {
    match call.selector {
        <proto::capture as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::capture>()?;
            router.forward(PeerRequest::Capture {
                id: args.id,
                request: args.request,
                reply: call.responder(router),
            });
        }
        <proto::capture_status as MethodMsg>::SELECTOR => {
            call.args::<proto::capture_status>()?;
            call.reply(server, Err(unavailable("capture tracking")));
        }
        <proto::transfers as MethodMsg>::SELECTOR => {
            call.args::<proto::transfers>()?;
            call.reply(server, Err(unavailable("transfer tracking")));
        }
        _ => return Ok(Routed::Elsewhere),
    }
    Ok(Routed::Handled)
}
