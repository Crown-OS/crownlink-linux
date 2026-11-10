//! Sending application messages, and requests whose answers come back later.

use std::time::Duration;

use llts_node::NodeError;
use llts_signaling::device::DeviceId;
use llts_signaling::message::{RequestCancel, RequestId, Signal};

use super::event_loop::PeerLoop;
use super::pending::Waiting;
use crate::util::error_chain::error_chain;

/// What a refused send means to the caller.
fn send_failure(error: &NodeError) -> String {
    match error {
        NodeError::FeatureUnavailable { feature, .. } => {
            format!("{feature:?} is not shared with the device")
        }
        NodeError::NotConnected(_) => "the device is not connected".to_owned(),
        NodeError::UnknownPeer(_) => "the device is not paired".to_owned(),
        other => error_chain(other),
    }
}

impl PeerLoop {
    /// Checks `signal` against its caps and hands it to the node for `peer`. Safe to call
    /// while the loop settles: it does not reset the failures of the command in progress.
    pub(super) fn send_signal(
        &mut self,
        peer: DeviceId,
        signal: &Signal<'_>,
    ) -> Result<(), String> {
        signal.validate().map_err(|error| error_chain(&error))?;
        let message = signal
            .encode(Vec::new())
            .map_err(|error| error_chain(&error))?;
        let now = self.clock.now();
        self.node
            .send_application(now, peer, &message)
            .map_err(|error| send_failure(&error))
    }

    /// Sends the request `build` makes under a fresh id and parks `waiting` until the answer,
    /// a refusal, `timeout` or the feature stops being shared.
    pub(super) fn ask<'a>(
        &mut self,
        peer: DeviceId,
        waiting: Waiting,
        timeout: Duration,
        build: impl FnOnce(RequestId) -> Signal<'a>,
    ) {
        let request = self.requests.allocate(peer);
        match self.send_signal(peer, &build(request)) {
            Ok(()) => {
                let deadline = self.clock.now().instant.saturating_add(timeout);
                self.requests.insert(peer, request, waiting, deadline);
            }
            Err(reason) => waiting.fail(reason),
        }
    }

    /// Fails every request whose deadline passed and tells each device to stop working on it.
    pub(super) fn expire_requests(&mut self) {
        let now = self.clock.now().instant;
        for abandoned in self.requests.expire(now) {
            abandoned.waiting.fail("the device did not answer in time");
            let cancel = Signal::RequestCancelV1(RequestCancel {
                request: abandoned.request,
            });
            if let Err(reason) = self.send_signal(abandoned.peer, &cancel) {
                tracing::debug!(
                    peer = %abandoned.peer,
                    request = abandoned.request.0,
                    reason,
                    "cannot cancel an expired request"
                );
            }
        }
    }
}
