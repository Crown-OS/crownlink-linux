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
    match call.selector {
        <proto::set_focus as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::set_focus>()?;
            router.forward(PeerRequest::SetFocus {
                focus: args.focus,
                reply: call.responder(router),
            });
        }
        <proto::focus as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::focus>()?;
            let focus = router.state.focus(args.id);
            call.reply(server, focus.and_then(|focus| encode(&focus)));
        }
        _ => return Ok(Routed::Elsewhere),
    }
    Ok(Routed::Handled)
}
