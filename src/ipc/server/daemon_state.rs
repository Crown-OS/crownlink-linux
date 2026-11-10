use std::collections::BTreeMap;

use crownos_ipc::RemoteError;
use llts_signaling::message::{Snapshot, Update};
use llts_signaling::state::{
    Applied, AudioRoutes as SignalingRoutes, Battery as SignalingBattery, CallState, Focus,
    Hotspot, LiveActivities, MediaSession, StateCache, Topic as SignalingTopic, TopicSet, Version,
};

use super::convert::{call_info, media_info, signaling_id, signaling_topic};
use super::outgoing::OutgoingEvent;
use crate::ipc::proto::{
    self, AudioRoutes, Battery, CallInfo, CaptureStatus, Collections, DeviceId, DeviceInfo,
    Feature, FeatureSet, FeatureState, FocusInfo, LiveActivity, MessageChange, Topic, Transfer,
};
use crate::peer::convert::{ipc_audio_routes, ipc_focus, ipc_live_activity};

/// One state document as the peer runtime received it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedUpdate {
    pub topic: SignalingTopic,
    pub version: Version,
    pub payload: Vec<u8>,
}

impl OwnedUpdate {
    fn borrowed(&self) -> Update<'_> {
        Update {
            topic: self.topic,
            version: self.version,
            payload: &self.payload,
        }
    }
}

/// What the peer runtime reports about devices, for the IPC server to cache and announce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonEvent {
    /// A device was paired, discovered or changed; replaces what was known about it.
    DeviceUpdated(DeviceInfo),
    DeviceRemoved(DeviceId),
    Connection {
        id: DeviceId,
        connected: bool,
    },
    PairingRequest {
        id: DeviceId,
        name: String,
        code: String,
    },
    PairingResult {
        id: DeviceId,
        accepted: bool,
    },
    FeatureState {
        id: DeviceId,
        feature: Feature,
        state: FeatureState,
    },
    FeatureFailed {
        id: DeviceId,
        feature: Feature,
        reason: String,
    },
    StateUpdate {
        owner: DeviceId,
        update: OwnedUpdate,
    },
    /// A call on a device's phone line changed, as this computer's own hands-free link saw it.
    Call {
        id: DeviceId,
        call: CallInfo,
    },
    /// Everything a device publishes, sent when its session opens.
    StateSnapshot {
        owner: DeviceId,
        topics: TopicSet,
        updates: Vec<OwnedUpdate>,
    },
    MessageChanged {
        id: DeviceId,
        change: MessageChange,
    },
    /// A device's collection counters, already cleared for features it no longer shares.
    Collections {
        id: DeviceId,
        collections: Collections,
    },
    Transfer(Transfer),
    CaptureFinished {
        capture: u32,
        status: CaptureStatus,
    },
    WifiReceived {
        id: DeviceId,
        ssid: String,
    },
    /// Which device rings this computer now, if any.
    LocalRing(Option<DeviceId>),
}

/// The IPC server's view of every device: the registry and the state each one published.
#[derive(Debug, Default)]
pub(crate) struct DaemonState {
    devices: BTreeMap<DeviceId, DeviceInfo>,
    cache: StateCache,
    collections: BTreeMap<DeviceId, Collections>,
    local_ring: Option<DeviceId>,
}

fn unknown(id: DeviceId) -> RemoteError {
    RemoteError::handler(format!("unknown device {id}"))
}

impl DaemonState {
    pub(crate) fn devices(&self) -> Vec<DeviceInfo> {
        self.devices.values().cloned().collect()
    }

    pub(crate) fn features(&self, id: DeviceId) -> Result<FeatureSet, RemoteError> {
        self.devices
            .get(&id)
            .map(|device| device.features)
            .ok_or_else(|| unknown(id))
    }

    pub(crate) fn set_feature(
        &mut self,
        id: DeviceId,
        feature: Feature,
        enabled: bool,
    ) -> Result<(FeatureSet, OutgoingEvent), RemoteError> {
        let device = self.devices.get_mut(&id).ok_or_else(|| unknown(id))?;
        if enabled {
            device.features = device.features.with(feature);
        } else {
            device.features = device.features.without(feature);
            device.active = device.active.without(feature);
        }
        let state = if !enabled {
            FeatureState::Disabled
        } else if device.active.contains(feature) {
            FeatureState::Active
        } else {
            FeatureState::Enabled
        };
        Ok((
            device.features,
            OutgoingEvent::FeatureChanged(proto::FeatureChanged { id, feature, state }),
        ))
    }

    pub(crate) fn state(&self, id: DeviceId, topic: Topic) -> Result<Option<Vec<u8>>, RemoteError> {
        self.known(id)?;
        Ok(self
            .cache
            .update(&signaling_id(id), signaling_topic(topic))
            .map(|update| update.payload.to_vec()))
    }

    pub(crate) fn focus(&self, id: DeviceId) -> Result<Option<FocusInfo>, RemoteError> {
        self.known(id)?;
        Ok(self.focus_of(id))
    }

    pub(crate) fn live_activities(&self, id: DeviceId) -> Result<Vec<LiveActivity>, RemoteError> {
        self.known(id)?;
        Ok(self.live_activities_of(id))
    }

    pub(crate) fn audio_routes(&self, id: DeviceId) -> Result<Option<AudioRoutes>, RemoteError> {
        self.known(id)?;
        Ok(self.audio_routes_of(id))
    }

    pub(crate) fn collections(&self, id: DeviceId) -> Result<Collections, RemoteError> {
        self.known(id)?;
        Ok(self.collections.get(&id).copied().unwrap_or_default())
    }

    pub(crate) const fn local_ring(&self) -> Option<DeviceId> {
        self.local_ring
    }

    fn known(&self, id: DeviceId) -> Result<(), RemoteError> {
        if self.devices.contains_key(&id) {
            Ok(())
        } else {
            Err(unknown(id))
        }
    }

    fn focus_of(&self, id: DeviceId) -> Option<FocusInfo> {
        self.cache
            .get::<Focus<'_>>(&signaling_id(id))
            .ok()
            .flatten()
            .as_ref()
            .map(ipc_focus)
    }

    fn live_activities_of(&self, id: DeviceId) -> Vec<LiveActivity> {
        self.cache
            .get::<LiveActivities<'_>>(&signaling_id(id))
            .ok()
            .flatten()
            .map(|live| live.activities.iter().map(ipc_live_activity).collect())
            .unwrap_or_default()
    }

    fn audio_routes_of(&self, id: DeviceId) -> Option<AudioRoutes> {
        self.cache
            .get::<SignalingRoutes<'_>>(&signaling_id(id))
            .ok()
            .flatten()
            .as_ref()
            .map(ipc_audio_routes)
    }

    pub(crate) fn apply(&mut self, event: DaemonEvent) -> Vec<OutgoingEvent> {
        match event {
            DaemonEvent::DeviceUpdated(device) => {
                let connection = self
                    .devices
                    .get(&device.id)
                    .is_none_or(|known| known.connected != device.connected)
                    .then(|| connected_event(&device));
                self.devices.insert(device.id, device);
                connection
                    .into_iter()
                    .chain([self.devices_changed()])
                    .collect()
            }
            DaemonEvent::DeviceRemoved(id) => {
                self.cache.forget(&signaling_id(id));
                self.collections.remove(&id);
                if self.devices.remove(&id).is_some() {
                    vec![self.devices_changed()]
                } else {
                    Vec::new()
                }
            }
            DaemonEvent::Connection { id, connected } => {
                let Some(device) = self.devices.get_mut(&id) else {
                    return Vec::new();
                };
                device.connected = connected;
                vec![connected_event(device), self.devices_changed()]
            }
            DaemonEvent::PairingRequest { id, name, code } => {
                vec![OutgoingEvent::PairingRequest(proto::PairingRequest {
                    id,
                    name,
                    code,
                })]
            }
            DaemonEvent::PairingResult { id, accepted } => {
                vec![OutgoingEvent::PairingResult(proto::PairingResult {
                    id,
                    accepted,
                })]
            }
            DaemonEvent::FeatureState { id, feature, state } => {
                if let Some(device) = self.devices.get_mut(&id) {
                    device.active = if state == FeatureState::Active {
                        device.active.with(feature)
                    } else {
                        device.active.without(feature)
                    };
                }
                vec![OutgoingEvent::FeatureChanged(proto::FeatureChanged {
                    id,
                    feature,
                    state,
                })]
            }
            DaemonEvent::FeatureFailed {
                id,
                feature,
                reason,
            } => vec![OutgoingEvent::FeatureFailed(proto::FeatureFailed {
                id,
                feature,
                reason,
            })],
            DaemonEvent::Call { id, call } => {
                vec![OutgoingEvent::CallStateChanged(proto::CallStateChanged {
                    id,
                    call,
                })]
            }
            DaemonEvent::StateUpdate { owner, update } => {
                match self
                    .cache
                    .apply_update(signaling_id(owner), &update.borrowed())
                {
                    Applied::Accepted => self.topic_events(owner, update.topic),
                    Applied::Stale => Vec::new(),
                }
            }
            DaemonEvent::StateSnapshot {
                owner,
                topics,
                updates,
            } => {
                let snapshot = Snapshot {
                    topics,
                    updates: updates.iter().map(OwnedUpdate::borrowed).collect(),
                };
                self.cache.apply_snapshot(signaling_id(owner), &snapshot);
                topics
                    .iter()
                    .flat_map(|topic| self.topic_events(owner, topic))
                    .collect()
            }
            DaemonEvent::MessageChanged { id, change } => {
                vec![OutgoingEvent::MessageChanged(proto::MessageChanged {
                    id,
                    change,
                })]
            }
            DaemonEvent::Collections { id, collections } => {
                self.collections.insert(id, collections);
                vec![OutgoingEvent::CollectionsChanged(
                    proto::CollectionsChanged { id, collections },
                )]
            }
            DaemonEvent::Transfer(transfer) => {
                vec![OutgoingEvent::TransferChanged(proto::TransferChanged {
                    transfer,
                })]
            }
            DaemonEvent::CaptureFinished { capture, status } => {
                vec![OutgoingEvent::CaptureFinished(proto::CaptureFinished {
                    capture,
                    status,
                })]
            }
            DaemonEvent::WifiReceived { id, ssid } => {
                vec![OutgoingEvent::WifiReceived(proto::WifiReceived {
                    id,
                    ssid,
                })]
            }
            DaemonEvent::LocalRing(by) => {
                self.local_ring = by;
                vec![OutgoingEvent::LocalRingChanged(proto::LocalRingChanged {
                    by,
                })]
            }
        }
    }

    fn devices_changed(&self) -> OutgoingEvent {
        OutgoingEvent::DevicesChanged(proto::DevicesChanged {
            devices: self.devices(),
        })
    }

    fn topic_events(&mut self, owner: DeviceId, topic: SignalingTopic) -> Vec<OutgoingEvent> {
        let id = signaling_id(owner);
        match topic {
            SignalingTopic::Battery => {
                let Ok(Some(battery)) = self.cache.get::<SignalingBattery>(&id) else {
                    return Vec::new();
                };
                if let Some(device) = self.devices.get_mut(&owner) {
                    device.battery = Some(Battery {
                        percent: battery.percent,
                        charging: battery.charging,
                    });
                }
                vec![OutgoingEvent::BatteryChanged(proto::BatteryChanged {
                    id: owner,
                    percent: battery.percent,
                    charging: battery.charging,
                })]
            }
            SignalingTopic::MediaSession => {
                let media = self
                    .cache
                    .get::<Option<MediaSession<'_>>>(&id)
                    .ok()
                    .flatten()
                    .flatten()
                    .as_ref()
                    .map(media_info);
                vec![OutgoingEvent::MediaChanged(proto::MediaChanged {
                    id: owner,
                    media,
                })]
            }
            SignalingTopic::Hotspot => match self.cache.get::<Hotspot<'_>>(&id) {
                Ok(Some(hotspot)) => vec![OutgoingEvent::HotspotChanged(proto::HotspotChanged {
                    id: owner,
                    enabled: hotspot.enabled,
                })],
                _ => Vec::new(),
            },
            SignalingTopic::CallState => match self.cache.get::<CallState<'_>>(&id) {
                Ok(Some(calls)) => calls
                    .calls
                    .iter()
                    .map(|call| {
                        OutgoingEvent::CallStateChanged(proto::CallStateChanged {
                            id: owner,
                            call: call_info(call),
                        })
                    })
                    .collect(),
                _ => Vec::new(),
            },
            SignalingTopic::Focus => self
                .focus_of(owner)
                .map(|focus| OutgoingEvent::FocusChanged(proto::FocusChanged { id: owner, focus }))
                .into_iter()
                .collect(),
            SignalingTopic::LiveActivities => {
                vec![OutgoingEvent::LiveActivitiesChanged(
                    proto::LiveActivitiesChanged {
                        id: owner,
                        activities: self.live_activities_of(owner),
                    },
                )]
            }
            SignalingTopic::AudioRoutes => self
                .audio_routes_of(owner)
                .map(|routes| {
                    OutgoingEvent::AudioRoutesChanged(proto::AudioRoutesChanged {
                        id: owner,
                        routes,
                    })
                })
                .into_iter()
                .collect(),
            SignalingTopic::DeviceInfo
            | SignalingTopic::Notifications
            | SignalingTopic::Clipboard
            | SignalingTopic::Volume => Vec::new(),
        }
    }
}

fn connected_event(device: &DeviceInfo) -> OutgoingEvent {
    OutgoingEvent::DeviceConnected(proto::DeviceConnected {
        id: device.id,
        name: device.name.clone(),
        connected: device.connected,
    })
}

#[cfg(test)]
mod tests {
    use llts_signaling::state::Incarnation;

    use super::*;
    use crate::ipc::proto::{DeviceClass, LinkKind};

    const PHONE: DeviceId = DeviceId([7; 32]);

    fn phone() -> DeviceInfo {
        DeviceInfo {
            id: PHONE,
            name: "Pixel".into(),
            class: DeviceClass::Phone,
            trusted: true,
            connected: true,
            link: LinkKind::Lan,
            battery: None,
            features: FeatureSet::EMPTY.with(Feature::Battery),
            active: FeatureSet::EMPTY,
        }
    }

    fn update<'a, P: llts_signaling::state::TopicPayload<'a>>(
        value: &P,
        seq: u64,
    ) -> Result<OwnedUpdate, postcard::Error> {
        Ok(OwnedUpdate {
            topic: P::TOPIC,
            version: Version {
                incarnation: Incarnation::from_raw(1),
                seq,
            },
            payload: postcard::to_stdvec(value)?,
        })
    }

    #[test]
    fn a_new_device_is_announced_with_the_list() {
        let mut state = DaemonState::default();
        let events = state.apply(DaemonEvent::DeviceUpdated(phone()));
        assert!(matches!(
            events.as_slice(),
            [
                OutgoingEvent::DeviceConnected(_),
                OutgoingEvent::DevicesChanged(_)
            ]
        ));
        let events = state.apply(DaemonEvent::DeviceUpdated(phone()));
        assert!(matches!(
            events.as_slice(),
            [OutgoingEvent::DevicesChanged(_)]
        ));
    }

    #[test]
    fn a_battery_update_is_cached_announced_and_served() -> Result<(), postcard::Error> {
        let mut state = DaemonState::default();
        state.apply(DaemonEvent::DeviceUpdated(phone()));
        let battery = SignalingBattery {
            percent: 64,
            charging: true,
            time_to_empty_min: None,
        };
        let owned = update(&battery, 1)?;
        let events = state.apply(DaemonEvent::StateUpdate {
            owner: PHONE,
            update: owned.clone(),
        });
        assert_eq!(
            events,
            [OutgoingEvent::BatteryChanged(proto::BatteryChanged {
                id: PHONE,
                percent: 64,
                charging: true
            })]
        );
        assert_eq!(
            state.state(PHONE, Topic::Battery).ok().flatten(),
            Some(owned.payload.clone())
        );
        assert_eq!(
            state.devices().first().and_then(|device| device.battery),
            Some(Battery {
                percent: 64,
                charging: true
            })
        );
        let stale = state.apply(DaemonEvent::StateUpdate {
            owner: PHONE,
            update: owned,
        });
        assert!(stale.is_empty());
        Ok(())
    }

    #[test]
    fn a_snapshot_without_media_announces_that_nothing_plays() {
        let mut state = DaemonState::default();
        state.apply(DaemonEvent::DeviceUpdated(phone()));
        let events = state.apply(DaemonEvent::StateSnapshot {
            owner: PHONE,
            topics: TopicSet::from(SignalingTopic::MediaSession),
            updates: Vec::new(),
        });
        assert_eq!(
            events,
            [OutgoingEvent::MediaChanged(proto::MediaChanged {
                id: PHONE,
                media: None
            })]
        );
    }

    #[test]
    fn focus_live_activities_and_audio_routes_are_announced_and_served(
    ) -> Result<(), postcard::Error> {
        use llts_signaling::state::{FocusMode, Headset, HybridTimestamp, LiveActivity as Live};

        let mut state = DaemonState::default();
        state.apply(DaemonEvent::DeviceUpdated(phone()));
        let focus = Focus {
            mode: FocusMode::DoNotDisturb,
            until_unix_ms: Some(10),
            stamp: HybridTimestamp::default(),
        };
        let events = state.apply(DaemonEvent::StateUpdate {
            owner: PHONE,
            update: update(&focus, 1)?,
        });
        let expected = FocusInfo {
            mode: proto::FocusMode::DoNotDisturb,
            until_unix_ms: Some(10),
        };
        assert_eq!(
            events,
            [OutgoingEvent::FocusChanged(proto::FocusChanged {
                id: PHONE,
                focus: expected.clone()
            })]
        );
        assert_eq!(state.focus(PHONE).ok().flatten(), Some(expected));

        let live = LiveActivities {
            activities: vec![Live {
                id: "k",
                package: "p",
                app: "Maps",
                title: "Home",
                text: "5 min",
                short_text: None,
                category: Some("navigation"),
                progress: None,
                chronometer: None,
                promoted: true,
                color_argb: None,
                actions: Vec::new(),
            }],
        };
        let events = state.apply(DaemonEvent::StateUpdate {
            owner: PHONE,
            update: update(&live, 1)?,
        });
        assert!(matches!(
            events.as_slice(),
            [OutgoingEvent::LiveActivitiesChanged(changed)] if changed.activities.len() == 1
        ));
        assert_eq!(
            state
                .live_activities(PHONE)
                .ok()
                .map(|activities| activities.len()),
            Some(1)
        );

        let routes = SignalingRoutes {
            headsets: vec![Headset {
                address: "AA:BB:CC:DD:EE:FF",
                name: "Buds",
                connected: false,
            }],
        };
        let events = state.apply(DaemonEvent::StateUpdate {
            owner: PHONE,
            update: update(&routes, 1)?,
        });
        assert!(matches!(
            events.as_slice(),
            [OutgoingEvent::AudioRoutesChanged(changed)] if changed.routes.headsets.len() == 1
        ));
        assert!(state.audio_routes(DeviceId([1; 32])).is_err());
        Ok(())
    }

    #[test]
    fn collections_are_none_until_published_and_forgotten_with_the_device() {
        let mut state = DaemonState::default();
        state.apply(DaemonEvent::DeviceUpdated(phone()));
        assert_eq!(state.collections(PHONE).ok(), Some(Collections::default()));
        let collections = Collections {
            calls: Some(proto::CallCounters {
                call_log_revision: 1,
                voicemail_revision: 2,
                missed: 3,
                unheard_voicemails: 0,
                capabilities: proto::CallCounters::VOICEMAIL_AUDIO,
            }),
            ..Collections::default()
        };
        let events = state.apply(DaemonEvent::Collections {
            id: PHONE,
            collections,
        });
        assert_eq!(
            events,
            [OutgoingEvent::CollectionsChanged(
                proto::CollectionsChanged {
                    id: PHONE,
                    collections
                }
            )]
        );
        assert_eq!(state.collections(PHONE).ok(), Some(collections));
        state.apply(DaemonEvent::DeviceRemoved(PHONE));
        assert!(state.collections(PHONE).is_err());
    }

    #[test]
    fn the_local_ring_is_remembered_until_it_stops() {
        let mut state = DaemonState::default();
        assert_eq!(state.local_ring(), None);
        state.apply(DaemonEvent::LocalRing(Some(PHONE)));
        assert_eq!(state.local_ring(), Some(PHONE));
        let events = state.apply(DaemonEvent::LocalRing(None));
        assert_eq!(
            events,
            [OutgoingEvent::LocalRingChanged(proto::LocalRingChanged {
                by: None
            })]
        );
    }

    #[test]
    fn disabling_a_feature_also_stops_it() -> Result<(), RemoteError> {
        let mut state = DaemonState::default();
        state.apply(DaemonEvent::DeviceUpdated(phone()));
        state.apply(DaemonEvent::FeatureState {
            id: PHONE,
            feature: Feature::Battery,
            state: FeatureState::Active,
        });
        let (features, event) = state.set_feature(PHONE, Feature::Battery, false)?;
        assert!(!features.contains(Feature::Battery));
        assert_eq!(
            event,
            OutgoingEvent::FeatureChanged(proto::FeatureChanged {
                id: PHONE,
                feature: Feature::Battery,
                state: FeatureState::Disabled
            })
        );
        assert!(state
            .devices()
            .iter()
            .all(|device| device.active.is_empty()));
        assert!(state
            .set_feature(DeviceId([1; 32]), Feature::Mic, true)
            .is_err());
        Ok(())
    }
}
