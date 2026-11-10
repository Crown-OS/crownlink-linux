//! The request arms of the `crownlink` service, one module per group of methods.

mod capture;
mod cast;
mod devices;
mod find;
mod focus;
mod links;
mod live;
mod messages;
mod phone;
mod wifi;

use crownos_ipc::{Message, MethodMsg, PeerId, RemoteError, Server};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::responder::DeferredReply;
use super::{send_deferred, Malformed, Responder, Router};

/// One request on its way to the group that serves it.
struct Call<'a> {
    peer: PeerId,
    req_id: u32,
    selector: u64,
    payload: &'a [u8],
}

impl Call<'_> {
    fn args<M: MethodMsg<Args = M> + DeserializeOwned>(&self) -> Result<M, Malformed> {
        Ok(postcard::from_bytes(self.payload)?)
    }

    fn reply(&self, server: &mut Server, outcome: Result<Vec<u8>, RemoteError>) {
        send_deferred(
            server,
            DeferredReply {
                peer: self.peer,
                req_id: self.req_id,
                outcome,
            },
        );
    }

    fn responder<R: Serialize>(&self, router: &Router) -> Responder<R> {
        router.responder(self.peer, self.req_id)
    }
}

/// Whether a group recognised the request's selector.
enum Routed {
    Handled,
    Elsewhere,
}

type Route = fn(&mut Router, &mut Server, &Call<'_>) -> Result<Routed, Malformed>;

const ROUTES: [Route; 10] = [
    devices::route,
    messages::route,
    phone::route,
    capture::route,
    cast::route,
    focus::route,
    live::route,
    wifi::route,
    find::route,
    links::route,
];

fn unavailable(what: &str) -> RemoteError {
    RemoteError::handler(format!(
        "{what} is not available in this build of crownlink"
    ))
}

impl Router {
    pub(super) fn request(
        &mut self,
        server: &mut Server,
        peer: PeerId,
        message: &Message,
    ) -> Result<(), Malformed> {
        let call = Call {
            peer,
            req_id: message.req_id,
            selector: message.selector,
            payload: &message.payload,
        };
        for route in ROUTES {
            if let Routed::Handled = route(self, server, &call)? {
                return Ok(());
            }
        }
        call.reply(server, Err(RemoteError::unknown_method(call.selector)));
        Ok(())
    }
}
