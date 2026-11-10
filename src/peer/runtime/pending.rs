//! Requests this computer sent a device and the IPC callers waiting for their answers.

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::time::Duration;

use llts_core::clock::Instant;
use llts_signaling::device::{DeviceId, Feature, FeatureSet};
use llts_signaling::message::RequestId;

use crate::ipc::proto::{CallLogPage, ContactPage, ConversationPage, MessagePage, VoicemailPage};
use crate::ipc::server::Responder;

/// A page that has not arrived in this long will not.
pub(super) const PAGE_TIMEOUT: Duration = Duration::from_secs(20);
/// Sending and marking read wait on the phone's provider and radio.
pub(super) const CHANGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Who is waiting for a request, and for what.
#[derive(Debug)]
pub(super) enum Waiting {
    Conversations(Responder<ConversationPage>),
    Messages {
        thread_id: u64,
        reply: Responder<MessagePage>,
    },
    CallLog(Responder<CallLogPage>),
    Voicemails(Responder<VoicemailPage>),
    Contacts(Responder<ContactPage>),
    SendMessage(Responder<String>),
    MarkRead(Responder<()>),
}

/// What a waiting caller belongs to, so a request ends when its feature stops being shared.
pub(super) trait Gated {
    fn feature(&self) -> Feature;
}

impl Gated for Waiting {
    fn feature(&self) -> Feature {
        match self {
            Self::Conversations(_)
            | Self::Messages { .. }
            | Self::SendMessage(_)
            | Self::MarkRead(_) => Feature::Messages,
            Self::CallLog(_) | Self::Voicemails(_) => Feature::CallHistory,
            Self::Contacts(_) => Feature::Contacts,
        }
    }
}

impl Waiting {
    pub(super) fn fail(self, reason: impl Into<String>) {
        match self {
            Self::Conversations(reply) => reply.fail(reason),
            Self::Messages { reply, .. } => reply.fail(reason),
            Self::CallLog(reply) => reply.fail(reason),
            Self::Voicemails(reply) => reply.fail(reason),
            Self::Contacts(reply) => reply.fail(reason),
            Self::SendMessage(reply) => reply.fail(reason),
            Self::MarkRead(reply) => reply.fail(reason),
        }
    }
}

#[derive(Debug)]
struct Pending<W> {
    waiting: W,
    deadline: Instant,
}

/// One request that ended without its answer.
#[derive(Debug)]
pub(super) struct Abandoned<W = Waiting> {
    pub(super) peer: DeviceId,
    pub(super) request: RequestId,
    pub(super) waiting: W,
}

/// Every unanswered request, keyed by the device and the id this computer gave it.
#[derive(Debug)]
pub(super) struct PendingRequests<W = Waiting> {
    next_ids: BTreeMap<DeviceId, u32>,
    pending: BTreeMap<(DeviceId, RequestId), Pending<W>>,
}

impl<W> Default for PendingRequests<W> {
    fn default() -> Self {
        Self {
            next_ids: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }
}

impl<W: Gated> PendingRequests<W> {
    /// A fresh id for a request to `peer`; ids wrap and skip ones still waiting.
    pub(super) fn allocate(&mut self, peer: DeviceId) -> RequestId {
        let next = self.next_ids.entry(peer).or_default();
        loop {
            let id = RequestId(*next);
            *next = next.wrapping_add(1);
            if !self.pending.contains_key(&(peer, id)) {
                return id;
            }
        }
    }

    pub(super) fn insert(
        &mut self,
        peer: DeviceId,
        request: RequestId,
        waiting: W,
        deadline: Instant,
    ) {
        self.pending
            .insert((peer, request), Pending { waiting, deadline });
    }

    /// The caller waiting for `request` from `peer`, if `answers` says the answer fits it.
    pub(super) fn take_if(
        &mut self,
        peer: DeviceId,
        request: RequestId,
        answers: impl FnOnce(&W) -> bool,
    ) -> Option<W> {
        match self.pending.entry((peer, request)) {
            Entry::Occupied(entry) if answers(&entry.get().waiting) => Some(entry.remove().waiting),
            _ => None,
        }
    }

    pub(super) fn next_deadline(&self) -> Option<Instant> {
        self.pending
            .values()
            .map(|pending| pending.deadline)
            .min_by_key(|deadline| deadline.as_micros())
    }

    /// Removes every request whose deadline passed by `now`.
    pub(super) fn expire(&mut self, now: Instant) -> Vec<Abandoned<W>> {
        self.remove_where(|_, pending| !pending.deadline.is_after(now))
    }

    /// Removes every request whose feature `shared` no longer allows for its device.
    pub(super) fn unshared(
        &mut self,
        shared: impl Fn(DeviceId) -> FeatureSet,
    ) -> Vec<Abandoned<W>> {
        self.remove_where(|peer, pending| !shared(peer).contains(pending.waiting.feature()))
    }

    fn remove_where(
        &mut self,
        remove: impl Fn(DeviceId, &Pending<W>) -> bool,
    ) -> Vec<Abandoned<W>> {
        let keys: Vec<_> = self
            .pending
            .iter()
            .filter(|((peer, _), pending)| remove(*peer, pending))
            .map(|(key, _)| *key)
            .collect();
        keys.into_iter()
            .filter_map(|(peer, request)| {
                self.pending
                    .remove(&(peer, request))
                    .map(|pending| Abandoned {
                        peer,
                        request,
                        waiting: pending.waiting,
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHONE: DeviceId = DeviceId([1; 32]);
    const TABLET: DeviceId = DeviceId([2; 32]);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Asked {
        Thread(u64),
        Calls,
        Contacts,
        Conversations,
    }

    impl Gated for Asked {
        fn feature(&self) -> Feature {
            match self {
                Self::Thread(_) | Self::Conversations => Feature::Messages,
                Self::Calls => Feature::CallHistory,
                Self::Contacts => Feature::Contacts,
            }
        }
    }

    fn at(seconds: u64) -> Instant {
        Instant::from_micros(seconds * 1_000_000)
    }

    #[test]
    fn ids_are_per_device_wrap_and_skip_waiting_ones() {
        let mut pending = PendingRequests::default();
        assert_eq!(pending.allocate(PHONE), RequestId(0));
        assert_eq!(pending.allocate(PHONE), RequestId(1));
        assert_eq!(pending.allocate(TABLET), RequestId(0));
        pending.next_ids.insert(PHONE, u32::MAX);
        pending.insert(PHONE, RequestId(0), Asked::Calls, at(1));
        assert_eq!(pending.allocate(PHONE), RequestId(u32::MAX));
        assert_eq!(pending.allocate(PHONE), RequestId(1));
    }

    #[test]
    fn an_answer_resolves_only_the_request_it_fits() {
        let mut pending = PendingRequests::default();
        pending.insert(PHONE, RequestId(4), Asked::Thread(9), at(20));
        let is_thread = |thread: u64| move |asked: &Asked| *asked == Asked::Thread(thread);
        assert_eq!(pending.take_if(TABLET, RequestId(4), is_thread(9)), None);
        assert_eq!(pending.take_if(PHONE, RequestId(4), is_thread(8)), None);
        assert_eq!(
            pending.take_if(PHONE, RequestId(4), is_thread(9)),
            Some(Asked::Thread(9))
        );
        assert_eq!(pending.take_if(PHONE, RequestId(4), is_thread(9)), None);
    }

    #[test]
    fn requests_expire_at_their_deadline() {
        let mut pending = PendingRequests::default();
        pending.insert(PHONE, RequestId(1), Asked::Calls, at(30));
        pending.insert(PHONE, RequestId(2), Asked::Contacts, at(20));
        assert_eq!(pending.next_deadline(), Some(at(20)));
        assert!(pending.expire(at(19)).is_empty());
        let expired: Vec<_> = pending
            .expire(at(20))
            .into_iter()
            .map(|gone| (gone.request, gone.waiting))
            .collect();
        assert_eq!(expired, [(RequestId(2), Asked::Contacts)]);
        assert_eq!(pending.next_deadline(), Some(at(30)));
    }

    #[test]
    fn requests_of_features_no_longer_shared_are_removed() {
        let mut pending = PendingRequests::default();
        pending.insert(PHONE, RequestId(1), Asked::Calls, at(20));
        pending.insert(PHONE, RequestId(2), Asked::Conversations, at(20));
        pending.insert(TABLET, RequestId(1), Asked::Conversations, at(20));
        let shared = |peer: DeviceId| {
            if peer == PHONE {
                FeatureSet::EMPTY.with(Feature::Messages)
            } else {
                FeatureSet::EMPTY
            }
        };
        let removed: Vec<_> = pending
            .unshared(shared)
            .into_iter()
            .map(|gone| (gone.peer, gone.request, gone.waiting))
            .collect();
        assert_eq!(
            removed,
            [
                (PHONE, RequestId(1), Asked::Calls),
                (TABLET, RequestId(1), Asked::Conversations)
            ]
        );
        assert_eq!(pending.next_deadline(), Some(at(20)));
        assert_eq!(pending.pending.len(), 1);
    }
}
