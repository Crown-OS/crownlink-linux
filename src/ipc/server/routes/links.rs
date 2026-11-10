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
    if call.selector != <proto::open_on_device as MethodMsg>::SELECTOR {
        return Ok(Routed::Elsewhere);
    }
    let args = call.args::<proto::open_on_device>()?;
    router.forward(PeerRequest::OpenOnDevice {
        id: args.id,
        uri: args.uri,
        reply: call.responder(router),
    });
    Ok(Routed::Handled)
}
