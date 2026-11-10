//! The methods that predate continuity: devices, pairing, features, streams, calls, state.

use crownos_ipc::{MethodMsg, Server};

use super::{Call, Routed};
use crate::ipc::proto;
use crate::ipc::server::{encode, Malformed, Router};
use crate::peer::{PeerNotice, PeerRequest};

pub(super) fn route(
    router: &mut Router,
    server: &mut Server,
    call: &Call<'_>,
) -> Result<Routed, Malformed> {
    use proto::*;
    match call.selector {
        <devices as MethodMsg>::SELECTOR => {
            call.args::<devices>()?;
            call.reply(server, encode(&router.state.devices()));
        }
        <features as MethodMsg>::SELECTOR => {
            let args = call.args::<features>()?;
            call.reply(
                server,
                router.state.features(args.id).and_then(|set| encode(&set)),
            );
        }
        <state as MethodMsg>::SELECTOR => {
            let args = call.args::<state>()?;
            call.reply(
                server,
                router
                    .state
                    .state(args.id, args.topic)
                    .and_then(|doc| encode(&doc)),
            );
        }
        <set_feature as MethodMsg>::SELECTOR => {
            let args = call.args::<set_feature>()?;
            match router
                .state
                .set_feature(args.id, args.feature, args.enabled)
            {
                Ok((features, event)) => {
                    call.reply(server, encode(&()));
                    let _ = event.emit(server);
                    router.notify(PeerNotice::FeaturesChanged {
                        id: args.id,
                        features,
                    });
                }
                Err(error) => call.reply(server, Err(error)),
            }
        }
        <pairing_begin as MethodMsg>::SELECTOR => {
            call.args::<pairing_begin>()?;
            router.forward(PeerRequest::PairingBegin(call.responder(router)));
        }
        <pairing_confirm as MethodMsg>::SELECTOR => {
            let args = call.args::<pairing_confirm>()?;
            router.forward(PeerRequest::PairingConfirm {
                id: args.id,
                accept: args.accept,
                reply: call.responder(router),
            });
        }
        <forget as MethodMsg>::SELECTOR => {
            let args = call.args::<forget>()?;
            router.forward(PeerRequest::Forget {
                id: args.id,
                reply: call.responder(router),
            });
        }
        <send_file as MethodMsg>::SELECTOR => {
            let args = call.args::<send_file>()?;
            router.forward(PeerRequest::SendFile {
                id: args.id,
                path: args.path,
                reply: call.responder(router),
            });
        }
        <start_mirror as MethodMsg>::SELECTOR => {
            let args = call.args::<start_mirror>()?;
            router.forward(PeerRequest::StartMirror {
                id: args.id,
                options: args.options,
                reply: call.responder(router),
            });
        }
        <start_camera as MethodMsg>::SELECTOR => {
            let args = call.args::<start_camera>()?;
            router.forward(PeerRequest::StartCamera {
                id: args.id,
                options: args.options,
                reply: call.responder(router),
            });
        }
        <start_mic as MethodMsg>::SELECTOR => {
            let args = call.args::<start_mic>()?;
            router.forward(PeerRequest::StartMic {
                id: args.id,
                reply: call.responder(router),
            });
        }
        <start_monitor as MethodMsg>::SELECTOR => {
            let args = call.args::<start_monitor>()?;
            router.forward(PeerRequest::StartMonitor {
                id: args.id,
                options: args.options,
                reply: call.responder(router),
            });
        }
        <stop as MethodMsg>::SELECTOR => {
            let args = call.args::<stop>()?;
            router.forward(PeerRequest::Stop {
                id: args.id,
                feature: args.feature,
                reply: call.responder(router),
            });
        }
        <set_unicursor_edge as MethodMsg>::SELECTOR => {
            let args = call.args::<set_unicursor_edge>()?;
            router.forward(PeerRequest::SetUnicursorEdge {
                id: args.id,
                edge: args.edge,
                reply: call.responder(router),
            });
        }
        <pickup_call as MethodMsg>::SELECTOR => {
            let args = call.args::<pickup_call>()?;
            router.forward(PeerRequest::PickupCall {
                call_id: args.call_id,
                reply: call.responder(router),
            });
        }
        <decline_call as MethodMsg>::SELECTOR => {
            let args = call.args::<decline_call>()?;
            router.forward(PeerRequest::DeclineCall {
                call_id: args.call_id,
                reply: call.responder(router),
            });
        }
        <hangup_call as MethodMsg>::SELECTOR => {
            let args = call.args::<hangup_call>()?;
            router.forward(PeerRequest::HangupCall {
                call_id: args.call_id,
                reply: call.responder(router),
            });
        }
        <dial as MethodMsg>::SELECTOR => {
            let args = call.args::<dial>()?;
            router.forward(PeerRequest::Dial {
                id: args.id,
                number: args.number,
                reply: call.responder(router),
            });
        }
        <set_hotspot as MethodMsg>::SELECTOR => {
            let args = call.args::<set_hotspot>()?;
            router.forward(PeerRequest::SetHotspot {
                id: args.id,
                enabled: args.enabled,
                reply: call.responder(router),
            });
        }
        <pair_with_qr as MethodMsg>::SELECTOR => {
            let args = call.args::<pair_with_qr>()?;
            router.forward(PeerRequest::PairWithQr {
                qr: args.qr,
                reply: call.responder(router),
            });
        }
        _ => return Ok(Routed::Elsewhere),
    }
    Ok(Routed::Handled)
}
