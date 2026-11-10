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
        <proto::ring_device as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::ring_device>()?;
            PeerRequest::RingDevice {
                id: args.id,
                reply: call.responder(router),
            }
        }
        <proto::stop_ringing as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::stop_ringing>()?;
            PeerRequest::StopRinging {
                id: args.id,
                reply: call.responder(router),
            }
        }
        <proto::locate as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::locate>()?;
            PeerRequest::Locate {
                id: args.id,
                reply: call.responder(router),
            }
        }
        <proto::local_ring as MethodMsg>::SELECTOR => {
            call.args::<proto::local_ring>()?;
            call.reply(server, encode(&router.state.local_ring()));
            return Ok(Routed::Handled);
        }
        _ => return Ok(Routed::Elsewhere),
    };
    router.forward(request);
    Ok(Routed::Handled)
}
