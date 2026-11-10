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
        <proto::live_activities as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::live_activities>()?;
            let activities = router.state.live_activities(args.id);
            call.reply(
                server,
                activities.and_then(|activities| encode(&activities)),
            );
        }
        <proto::live_activity_action as MethodMsg>::SELECTOR => {
            let args = call.args::<proto::live_activity_action>()?;
            router.forward(PeerRequest::LiveActivityAction {
                id: args.id,
                activity_id: args.activity_id,
                action_id: args.action_id,
                reply: call.responder(router),
            });
        }
        _ => return Ok(Routed::Elsewhere),
    }
    Ok(Routed::Handled)
}
