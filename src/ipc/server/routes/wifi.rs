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
        <proto::share_wifi as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::share_wifi>()?;
            PeerRequest::ShareWifi {
                id: args.id,
                ssid: args.ssid,
                reply: call.responder(router),
            }
        }
        <proto::request_wifi as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::request_wifi>()?;
            PeerRequest::RequestWifi {
                id: args.id,
                ssid: args.ssid,
                reply: call.responder(router),
            }
        }
        _ => return Ok(Routed::Elsewhere),
    };
    router.forward(request);
    Ok(Routed::Handled)
}
