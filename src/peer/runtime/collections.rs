//! The counters each device last pushed for its messages, calls and contacts.

use std::collections::BTreeMap;

use llts_signaling::device::{DeviceId, FeatureSet};

use crate::ipc::proto::Collections;
use crate::peer::convert::shared_collections;

#[derive(Debug, Default)]
pub(super) struct DeviceCollections {
    devices: BTreeMap<DeviceId, Collections>,
}

impl DeviceCollections {
    pub(super) fn get(&self, peer: &DeviceId) -> Collections {
        self.devices.get(peer).copied().unwrap_or_default()
    }

    /// Applies `change` to `peer`'s counters; the result when it changed anything.
    pub(super) fn update(
        &mut self,
        peer: DeviceId,
        change: impl FnOnce(&mut Collections),
    ) -> Option<Collections> {
        let collections = self.devices.entry(peer).or_default();
        let before = *collections;
        change(collections);
        (*collections != before).then_some(*collections)
    }

    /// Clears every counter whose feature `shared` no longer allows (`None`: the device is
    /// gone); the devices whose counters changed, with what is left of them.
    pub(super) fn retain_shared(
        &mut self,
        shared: impl Fn(&DeviceId) -> Option<FeatureSet>,
    ) -> Vec<(DeviceId, Collections)> {
        let mut changed = Vec::new();
        self.devices.retain(|peer, collections| {
            let Some(features) = shared(peer) else {
                return false;
            };
            let kept = shared_collections(*collections, features);
            if kept != *collections {
                *collections = kept;
                changed.push((*peer, kept));
            }
            true
        });
        changed
    }
}

#[cfg(test)]
mod tests {
    use llts_signaling::device::Feature;

    use super::*;
    use crate::ipc::proto::{ContactCounters, MessageCounters};

    const PHONE: DeviceId = DeviceId([1; 32]);
    const GONE: DeviceId = DeviceId([2; 32]);
    const MESSAGES: MessageCounters = MessageCounters {
        revision: 1,
        unread: 3,
        capabilities: MessageCounters::SEND_SMS,
        mms_max_bytes: 0,
    };
    const CONTACTS: ContactCounters = ContactCounters {
        revision: 2,
        count: 40,
        capabilities: 0,
    };

    #[test]
    fn an_update_reports_only_real_changes() {
        let mut collections = DeviceCollections::default();
        let set_messages = |collections: &mut Collections| collections.messages = Some(MESSAGES);
        assert_eq!(
            collections.update(PHONE, set_messages),
            Some(Collections {
                messages: Some(MESSAGES),
                ..Collections::default()
            })
        );
        assert_eq!(collections.update(PHONE, set_messages), None);
        assert_eq!(collections.get(&PHONE).messages, Some(MESSAGES));
        assert_eq!(collections.get(&GONE), Collections::default());
    }

    #[test]
    fn unshared_counters_are_cleared_and_gone_devices_forgotten() {
        let mut collections = DeviceCollections::default();
        let both = |collections: &mut Collections| {
            collections.messages = Some(MESSAGES);
            collections.contacts = Some(CONTACTS);
        };
        let _ = collections.update(PHONE, both);
        let _ = collections.update(GONE, both);
        let changed = collections.retain_shared(|peer| {
            (*peer == PHONE).then_some(FeatureSet::EMPTY.with(Feature::Contacts))
        });
        let left = Collections {
            contacts: Some(CONTACTS),
            ..Collections::default()
        };
        assert_eq!(changed, [(PHONE, left)]);
        assert_eq!(collections.get(&GONE), Collections::default());
        assert!(collections
            .retain_shared(|_| Some(FeatureSet::EMPTY.with(Feature::Contacts)))
            .is_empty());
    }
}
