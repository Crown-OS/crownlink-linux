//! Application messages: requests, pages and pushes the node routes without reading.
//!
//! Logs name only the message type and the peer, never a body: these carry messages,
//! contacts, credentials and locations.

use llts_signaling::device::DeviceId;
use llts_signaling::message::Signal;

use super::event_loop::PeerLoop;

impl PeerLoop {
    pub(super) fn on_application(&self, peer: DeviceId, message: &[u8]) {
        match Signal::decode(message) {
            Ok(signal) => match signal.validate() {
                Ok(()) => tracing::debug!(
                    kind = ?signal.message_type(),
                    %peer,
                    "no feature of this build handles the application message"
                ),
                Err(error) => tracing::debug!(
                    kind = ?signal.message_type(),
                    %peer,
                    %error,
                    "dropping an application message that breaks its caps"
                ),
            },
            Err(error) => {
                tracing::debug!(%peer, %error, "dropping a malformed application message")
            }
        }
    }
}
