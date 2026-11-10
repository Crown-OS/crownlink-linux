use crownos_ipc::{MethodMsg, Server};

use super::{unavailable, Call, Routed};
use crate::ipc::proto;
use crate::ipc::server::{encode, Malformed, Router};
use crate::peer::PeerRequest;

pub(super) fn route(
    router: &mut Router,
    server: &mut Server,
    call: &Call<'_>,
) -> Result<Routed, Malformed> {
    let request = match call.selector {
        <proto::start_cast as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::start_cast>()?;
            PeerRequest::StartCast {
                id: args.id,
                options: args.options,
                reply: call.responder(router),
            }
        }
        <proto::start_audio as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::start_audio>()?;
            PeerRequest::StartAudio {
                id: args.id,
                options: args.options,
                reply: call.responder(router),
            }
        }
        <proto::move_headset as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::move_headset>()?;
            PeerRequest::MoveHeadset {
                id: args.id,
                address: args.address,
                to: args.to,
                reply: call.responder(router),
            }
        }
        <proto::audio_routes as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::audio_routes>()?;
            let routes = router.state.audio_routes(args.id);
            call.reply(server, routes.and_then(|routes| encode(&routes)));
            return Ok(Routed::Handled);
        }
        <proto::local_audio_routes as MethodMsg>::SELECTOR => {
            call.args::<proto::local_audio_routes>()?;
            call.reply(
                server,
                Err(unavailable("handing this computer's headsets over")),
            );
            return Ok(Routed::Handled);
        }
        _ => return Ok(Routed::Elsewhere),
    };
    router.forward(request);
    Ok(Routed::Handled)
}
