use llts_signaling::device::{Feature, FeatureSet};
use llts_signaling::message as wire;

use crate::ipc::proto::{CallCounters, Collections, ContactCounters, MessageCounters};

pub(crate) const fn ipc_message_counters(counters: wire::MessageCounters) -> MessageCounters {
    MessageCounters {
        revision: counters.revision,
        unread: counters.unread,
        capabilities: counters.capabilities.0,
        mms_max_bytes: counters.mms_max_bytes,
    }
}

pub(crate) const fn ipc_call_counters(counters: wire::CallCounters) -> CallCounters {
    CallCounters {
        call_log_revision: counters.call_log_revision,
        voicemail_revision: counters.voicemail_revision,
        missed: counters.missed,
        unheard_voicemails: counters.unheard_voicemails,
        capabilities: counters.capabilities.0,
    }
}

pub(crate) const fn ipc_contact_counters(counters: wire::ContactCounters) -> ContactCounters {
    ContactCounters {
        revision: counters.revision,
        count: counters.count,
        capabilities: counters.capabilities.0,
    }
}

/// `collections` without the counters of features `shared` lacks.
pub(crate) const fn shared_collections(
    collections: Collections,
    shared: FeatureSet,
) -> Collections {
    Collections {
        messages: if shared.contains(Feature::Messages) {
            collections.messages
        } else {
            None
        },
        calls: if shared.contains(Feature::CallHistory) {
            collections.calls
        } else {
            None
        },
        contacts: if shared.contains(Feature::Contacts) {
            collections.contacts
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Collections {
        Collections {
            messages: Some(ipc_message_counters(wire::MessageCounters {
                revision: 1,
                unread: 2,
                capabilities: wire::MessageCapabilities(0b1000_0001),
                mms_max_bytes: 300_000,
            })),
            calls: Some(ipc_call_counters(wire::CallCounters {
                call_log_revision: 3,
                voicemail_revision: 4,
                missed: 5,
                unheard_voicemails: 6,
                capabilities: wire::CallCapabilities::VOICEMAIL_AUDIO,
            })),
            contacts: Some(ipc_contact_counters(wire::ContactCounters {
                revision: 7,
                count: 8,
                capabilities: wire::ContactCapabilities::PHOTOS,
            })),
        }
    }

    #[test]
    fn counters_keep_every_field_and_unknown_capability_bits() {
        assert_eq!(
            full(),
            Collections {
                messages: Some(MessageCounters {
                    revision: 1,
                    unread: 2,
                    capabilities: 0b1000_0001,
                    mms_max_bytes: 300_000,
                }),
                calls: Some(CallCounters {
                    call_log_revision: 3,
                    voicemail_revision: 4,
                    missed: 5,
                    unheard_voicemails: 6,
                    capabilities: CallCounters::VOICEMAIL_AUDIO,
                }),
                contacts: Some(ContactCounters {
                    revision: 7,
                    count: 8,
                    capabilities: ContactCounters::PHOTOS,
                }),
            }
        );
    }

    #[test]
    fn counters_of_unshared_features_are_cleared() {
        let shared = FeatureSet::EMPTY.with(Feature::CallHistory);
        assert_eq!(
            shared_collections(full(), shared),
            Collections {
                calls: full().calls,
                ..Collections::default()
            }
        );
        assert_eq!(
            shared_collections(full(), FeatureSet::EMPTY),
            Collections::default()
        );
    }
}
