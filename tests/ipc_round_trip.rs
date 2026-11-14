#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that fails a precondition should fail loudly"
)]

use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crownlink_linux::ipc::proto::{
    self, AudioOptions, AudioRoutes, Battery, BlobFile, CallLogPage, CameraOptions, CaptureRequest,
    CaptureStatus, CastOptions, Collections, ContactPage, Conversation, ConversationPage,
    DeviceClass, DeviceId, DeviceInfo, DeviceLocation, Edge, Feature, FeatureSet, FeatureState,
    FocusInfo, FocusMode, HeadsetOwner, LinkKind, LiveActivity, LocalIdentity, MessageDraft,
    MessagePage, MirrorOptions, MonitorMode, MonitorOptions, PageRequest, PairingOffer,
    Participant, Topic, Transfer, VoicemailPage,
};
use crownos_ipc::{blocking, Error, MethodMsg, RemoteError, ServiceBuilder};

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(5));
const PHONE: DeviceId = DeviceId([7; 32]);
const STRANGER: DeviceId = DeviceId([9; 32]);

#[derive(Debug, Default)]
struct FakeHub {
    features: FeatureSet,
    last_seek: Option<u8>,
    picked_up: Vec<String>,
    focus: Option<FocusInfo>,
    drafts: Vec<MessageDraft>,
}

fn phone() -> DeviceInfo {
    DeviceInfo {
        id: PHONE,
        name: "Pixel".into(),
        class: DeviceClass::Phone,
        trusted: true,
        connected: true,
        link: LinkKind::Lan,
        battery: Some(Battery {
            percent: 80,
            charging: false,
        }),
        features: FeatureSet::EMPTY.with(Feature::Clipboard),
        active: FeatureSet::EMPTY,
    }
}

fn known(id: DeviceId) -> Result<(), RemoteError> {
    if id == PHONE {
        Ok(())
    } else {
        Err(RemoteError::handler(format!("unknown device {id}")))
    }
}

impl proto::Handler for FakeHub {
    fn devices(&mut self) -> Result<Vec<DeviceInfo>, RemoteError> {
        Ok(vec![phone()])
    }

    fn pairing_begin(&mut self) -> Result<PairingOffer, RemoteError> {
        Ok(PairingOffer {
            qr: "crownlink:offer".into(),
            expires_unix_ms: 1,
        })
    }

    fn pairing_confirm(&mut self, id: DeviceId, _accept: bool) -> Result<(), RemoteError> {
        known(id)
    }

    fn pairing_cancel(&mut self) {}

    fn forget(&mut self, id: DeviceId) -> Result<(), RemoteError> {
        known(id)
    }

    fn trust_computer(
        &mut self,
        key: [u8; 32],
        _name: String,
        _address: String,
    ) -> Result<DeviceId, RemoteError> {
        Ok(DeviceId(key))
    }

    fn local_identity(&mut self) -> Result<LocalIdentity, RemoteError> {
        Ok(LocalIdentity {
            id: PHONE,
            llts_port: 0,
        })
    }

    fn set_feature(
        &mut self,
        id: DeviceId,
        feature: Feature,
        enabled: bool,
    ) -> Result<(), RemoteError> {
        known(id)?;
        self.features = if enabled {
            self.features.with(feature)
        } else {
            self.features.without(feature)
        };
        Ok(())
    }

    fn features(&mut self, id: DeviceId) -> Result<FeatureSet, RemoteError> {
        known(id).map(|()| self.features)
    }

    fn send_file(&mut self, id: DeviceId, _path: String) -> Result<(), RemoteError> {
        known(id)
    }

    fn start_mirror(&mut self, id: DeviceId, _options: MirrorOptions) -> Result<(), RemoteError> {
        known(id)
    }

    fn start_camera(&mut self, id: DeviceId, _options: CameraOptions) -> Result<(), RemoteError> {
        known(id)
    }

    fn start_mic(&mut self, id: DeviceId) -> Result<(), RemoteError> {
        known(id)
    }

    fn start_monitor(&mut self, id: DeviceId, _options: MonitorOptions) -> Result<(), RemoteError> {
        known(id)
    }

    fn stop(&mut self, id: DeviceId, _feature: Feature) -> Result<(), RemoteError> {
        known(id)
    }

    fn set_unicursor_edge(&mut self, id: DeviceId, _edge: Edge) -> Result<(), RemoteError> {
        known(id)
    }

    fn set_monitor_mode(&mut self, id: DeviceId, _mode: MonitorMode) -> Result<(), RemoteError> {
        known(id)
    }

    fn set_viewer_title(&mut self, id: DeviceId, _title: String) -> Result<(), RemoteError> {
        known(id)
    }

    fn pickup_call(&mut self, call_id: String) -> Result<(), RemoteError> {
        self.picked_up.push(call_id);
        Ok(())
    }

    fn decline_call(&mut self, _call_id: String) -> Result<(), RemoteError> {
        Ok(())
    }

    fn hangup_call(&mut self, _call_id: String) -> Result<(), RemoteError> {
        Ok(())
    }

    fn dial(&mut self, id: DeviceId, _number: String) -> Result<(), RemoteError> {
        known(id)
    }

    fn media_play(&mut self) {}

    fn media_pause(&mut self) {}

    fn media_next(&mut self) {}

    fn media_previous(&mut self) {}

    fn media_seek(&mut self, percent: u8) {
        self.last_seek = Some(percent);
    }

    fn send_reply(&mut self, _conversation_id: String, _text: String) {}

    fn state(&mut self, id: DeviceId, topic: Topic) -> Result<Option<Vec<u8>>, RemoteError> {
        known(id).map(|()| (topic == Topic::Battery).then(|| vec![80, 0]))
    }

    fn set_hotspot(&mut self, id: DeviceId, _enabled: bool) -> Result<(), RemoteError> {
        known(id)
    }

    fn pair_with_qr(&mut self, qr: String) -> Result<(), RemoteError> {
        if qr.is_empty() {
            Err(RemoteError::handler("empty invitation"))
        } else {
            Ok(())
        }
    }

    fn conversations(
        &mut self,
        id: DeviceId,
        page: PageRequest,
    ) -> Result<ConversationPage, RemoteError> {
        known(id)?;
        Ok(ConversationPage {
            items: vec![Conversation {
                thread_id: 4,
                participants: vec![Participant {
                    address: "+15550100".into(),
                    name: None,
                    contact_key: None,
                }],
                participant_count: 1,
                snippet: "see you".into(),
                last_unix_ms: 9,
                unread: u32::from(page.limit),
                has_attachments: false,
            }],
            next: page.cursor.map(|cursor| format!("{cursor}+")),
        })
    }

    fn messages(
        &mut self,
        id: DeviceId,
        thread_id: u64,
        _page: PageRequest,
    ) -> Result<MessagePage, RemoteError> {
        known(id).map(|()| MessagePage {
            thread_id,
            items: Vec::new(),
            next: None,
        })
    }

    fn send_message(&mut self, id: DeviceId, draft: MessageDraft) -> Result<String, RemoteError> {
        known(id)?;
        self.drafts.push(draft);
        Ok(format!("sms/{}", self.drafts.len()))
    }

    fn mark_read(&mut self, id: DeviceId, _thread_id: u64) -> Result<(), RemoteError> {
        known(id)
    }

    fn fetch_blob(
        &mut self,
        id: DeviceId,
        _feature: Feature,
        _key: String,
    ) -> Result<BlobFile, RemoteError> {
        known(id)?;
        Err(RemoteError::handler("no such blob"))
    }

    fn call_log(&mut self, id: DeviceId, _page: PageRequest) -> Result<CallLogPage, RemoteError> {
        known(id).map(|()| CallLogPage::default())
    }

    fn voicemails(
        &mut self,
        id: DeviceId,
        _page: PageRequest,
    ) -> Result<VoicemailPage, RemoteError> {
        known(id).map(|()| VoicemailPage::default())
    }

    fn contacts(&mut self, id: DeviceId, _page: PageRequest) -> Result<ContactPage, RemoteError> {
        known(id).map(|()| ContactPage::default())
    }

    fn collections(&mut self, id: DeviceId) -> Result<Collections, RemoteError> {
        known(id).map(|()| Collections::default())
    }

    fn capture(&mut self, id: DeviceId, _request: CaptureRequest) -> Result<u32, RemoteError> {
        known(id).map(|()| 1)
    }

    fn cancel_capture(&mut self, _capture: u32) {}

    fn capture_status(&mut self, _capture: u32) -> Result<CaptureStatus, RemoteError> {
        Ok(CaptureStatus::Pending)
    }

    fn transfers(&mut self) -> Result<Vec<Transfer>, RemoteError> {
        Ok(Vec::new())
    }

    fn start_cast(&mut self, id: DeviceId, _options: CastOptions) -> Result<(), RemoteError> {
        known(id)
    }

    fn start_audio(&mut self, id: DeviceId, _options: AudioOptions) -> Result<(), RemoteError> {
        known(id)
    }

    fn move_headset(
        &mut self,
        id: DeviceId,
        _address: String,
        _to: HeadsetOwner,
    ) -> Result<(), RemoteError> {
        known(id)
    }

    fn audio_routes(&mut self, id: DeviceId) -> Result<Option<AudioRoutes>, RemoteError> {
        known(id).map(|()| None)
    }

    fn local_audio_routes(&mut self) -> Result<AudioRoutes, RemoteError> {
        Ok(AudioRoutes::default())
    }

    fn set_focus(&mut self, focus: FocusInfo) -> Result<(), RemoteError> {
        self.focus = Some(focus);
        Ok(())
    }

    fn focus(&mut self, id: DeviceId) -> Result<Option<FocusInfo>, RemoteError> {
        known(id).map(|()| self.focus.clone())
    }

    fn live_activities(&mut self, id: DeviceId) -> Result<Vec<LiveActivity>, RemoteError> {
        known(id).map(|()| Vec::new())
    }

    fn live_activity_action(
        &mut self,
        id: DeviceId,
        _activity_id: String,
        _action_id: String,
    ) -> Result<(), RemoteError> {
        known(id)
    }

    fn share_wifi(&mut self, id: DeviceId, _ssid: String) -> Result<(), RemoteError> {
        known(id)
    }

    fn request_wifi(&mut self, id: DeviceId, _ssid: String) -> Result<(), RemoteError> {
        known(id)
    }

    fn ring_device(&mut self, id: DeviceId) -> Result<(), RemoteError> {
        known(id)
    }

    fn stop_ringing(&mut self, id: DeviceId) -> Result<(), RemoteError> {
        known(id)
    }

    fn locate(&mut self, id: DeviceId) -> Result<DeviceLocation, RemoteError> {
        known(id).map(|()| DeviceLocation {
            latitude: 51.5,
            longitude: -0.12,
            accuracy_m: 12,
            altitude_m: None,
            fix_unix_ms: 3,
        })
    }

    fn local_ring(&mut self) -> Result<Option<DeviceId>, RemoteError> {
        Ok(None)
    }

    fn stop_local_ring(&mut self) {}

    fn open_on_device(&mut self, id: DeviceId, _uri: String) -> Result<(), RemoteError> {
        known(id)
    }
}

fn socket_directory() -> PathBuf {
    std::env::temp_dir().join(format!("crownlink-ipc-{}", std::process::id()))
}

fn spawn_hub(directory: PathBuf) -> JoinHandle<FakeHub> {
    let ready = Arc::new(Barrier::new(2));
    let server_ready = Arc::clone(&ready);
    let handle = std::thread::spawn(move || {
        let mut server = ServiceBuilder::new(proto::SERVICE)
            .directory(&directory)
            .build()
            .expect("bind");
        server_ready.wait();
        let mut hub = FakeHub::default();
        blocking::serve(&mut server, |server, peer, message| {
            let selector = message.selector;
            proto::dispatch(&mut hub, server, peer, message)?;
            if selector == <proto::set_feature as MethodMsg>::SELECTOR {
                server.emit(&proto::FeatureChanged {
                    id: PHONE,
                    feature: Feature::Camera,
                    state: FeatureState::Enabled,
                })?;
            }
            Ok(selector != <proto::pairing_cancel as MethodMsg>::SELECTOR)
        })
        .expect("serve");
        hub
    });
    ready.wait();
    handle
}

fn next_event(client: &mut proto::Client) -> proto::Event {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(event) = client.next_event() {
            return event;
        }
        assert!(Instant::now() < deadline, "event never arrived");
        let _ = client.handle_readable();
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_client_drives_the_hub_through_the_frozen_schema() {
    let directory = socket_directory();
    let hub = spawn_hub(directory.clone());
    let mut client = proto::Client::connect_in(&directory).expect("connect");

    let pending = client.devices().expect("devices");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("devices reply"),
        [phone()]
    );

    client
        .subscribe_blocking::<proto::FeatureChanged>(TIMEOUT)
        .expect("subscribe");
    let pending = client
        .set_feature(PHONE, Feature::Camera, true)
        .expect("set_feature");
    client.wait(pending, TIMEOUT).expect("set_feature reply");
    match next_event(&mut client) {
        proto::Event::FeatureChanged(changed) => {
            assert_eq!(changed.feature, Feature::Camera);
            assert_eq!(changed.state, FeatureState::Enabled);
        }
        other => panic!("unexpected event {other:?}"),
    }

    let pending = client.features(PHONE).expect("features");
    let features = client.wait(pending, TIMEOUT).expect("features reply");
    assert!(features.contains(Feature::Camera));

    let pending = client.state(PHONE, Topic::Battery).expect("state");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("state reply"),
        Some(vec![80, 0])
    );

    let pending = client.forget(STRANGER).expect("forget");
    match client.wait(pending, TIMEOUT) {
        Err(Error::Remote(remote)) => assert_eq!(remote.code, RemoteError::HANDLER),
        other => panic!("expected a handler error, got {other:?}"),
    }

    let pending = client
        .conversations(
            PHONE,
            PageRequest {
                cursor: Some("c1".into()),
                limit: 25,
            },
        )
        .expect("conversations");
    let page = client.wait(pending, TIMEOUT).expect("conversations reply");
    assert_eq!(page.next.as_deref(), Some("c1+"));
    assert_eq!(page.items.first().map(|item| item.unread), Some(25));

    let draft = MessageDraft {
        recipients: vec!["+15550100".into()],
        text: "hello".into(),
        ..MessageDraft::default()
    };
    let pending = client.send_message(PHONE, draft.clone()).expect("send");
    assert_eq!(client.wait(pending, TIMEOUT).expect("send reply"), "sms/1");

    let focus = FocusInfo {
        mode: FocusMode::Named("Work".into()),
        until_unix_ms: Some(7),
    };
    let pending = client.set_focus(focus.clone()).expect("set_focus");
    client.wait(pending, TIMEOUT).expect("set_focus reply");
    let pending = client.focus(PHONE).expect("focus");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("focus reply"),
        Some(focus)
    );

    let pending = client.locate(PHONE).expect("locate");
    let location = client.wait(pending, TIMEOUT).expect("locate reply");
    assert_eq!(location.accuracy_m, 12);

    let pending = client.pickup_call("call-1".into()).expect("pickup");
    client.wait(pending, TIMEOUT).expect("pickup reply");
    client.media_seek(42).expect("seek");
    client.pairing_cancel().expect("stop");

    let hub = hub.join().expect("hub thread");
    assert_eq!(hub.last_seek, Some(42));
    assert_eq!(hub.picked_up, ["call-1"]);
    assert_eq!(hub.drafts, [draft]);
}

#[test]
fn every_selector_in_the_schema_is_distinct() {
    use crownos_ipc::EventMsg;
    let mut selectors = vec![
        <proto::devices as MethodMsg>::SELECTOR,
        <proto::pairing_begin as MethodMsg>::SELECTOR,
        <proto::pairing_confirm as MethodMsg>::SELECTOR,
        <proto::pairing_cancel as MethodMsg>::SELECTOR,
        <proto::forget as MethodMsg>::SELECTOR,
        <proto::set_feature as MethodMsg>::SELECTOR,
        <proto::features as MethodMsg>::SELECTOR,
        <proto::send_file as MethodMsg>::SELECTOR,
        <proto::start_mirror as MethodMsg>::SELECTOR,
        <proto::start_camera as MethodMsg>::SELECTOR,
        <proto::start_mic as MethodMsg>::SELECTOR,
        <proto::start_monitor as MethodMsg>::SELECTOR,
        <proto::stop as MethodMsg>::SELECTOR,
        <proto::set_unicursor_edge as MethodMsg>::SELECTOR,
        <proto::pickup_call as MethodMsg>::SELECTOR,
        <proto::decline_call as MethodMsg>::SELECTOR,
        <proto::hangup_call as MethodMsg>::SELECTOR,
        <proto::dial as MethodMsg>::SELECTOR,
        <proto::media_play as MethodMsg>::SELECTOR,
        <proto::media_pause as MethodMsg>::SELECTOR,
        <proto::media_next as MethodMsg>::SELECTOR,
        <proto::media_previous as MethodMsg>::SELECTOR,
        <proto::media_seek as MethodMsg>::SELECTOR,
        <proto::send_reply as MethodMsg>::SELECTOR,
        <proto::state as MethodMsg>::SELECTOR,
        <proto::set_hotspot as MethodMsg>::SELECTOR,
        <proto::pair_with_qr as MethodMsg>::SELECTOR,
        <proto::conversations as MethodMsg>::SELECTOR,
        <proto::messages as MethodMsg>::SELECTOR,
        <proto::send_message as MethodMsg>::SELECTOR,
        <proto::mark_read as MethodMsg>::SELECTOR,
        <proto::fetch_blob as MethodMsg>::SELECTOR,
        <proto::call_log as MethodMsg>::SELECTOR,
        <proto::voicemails as MethodMsg>::SELECTOR,
        <proto::contacts as MethodMsg>::SELECTOR,
        <proto::collections as MethodMsg>::SELECTOR,
        <proto::capture as MethodMsg>::SELECTOR,
        <proto::cancel_capture as MethodMsg>::SELECTOR,
        <proto::capture_status as MethodMsg>::SELECTOR,
        <proto::transfers as MethodMsg>::SELECTOR,
        <proto::start_cast as MethodMsg>::SELECTOR,
        <proto::start_audio as MethodMsg>::SELECTOR,
        <proto::move_headset as MethodMsg>::SELECTOR,
        <proto::audio_routes as MethodMsg>::SELECTOR,
        <proto::local_audio_routes as MethodMsg>::SELECTOR,
        <proto::set_focus as MethodMsg>::SELECTOR,
        <proto::focus as MethodMsg>::SELECTOR,
        <proto::live_activities as MethodMsg>::SELECTOR,
        <proto::live_activity_action as MethodMsg>::SELECTOR,
        <proto::share_wifi as MethodMsg>::SELECTOR,
        <proto::request_wifi as MethodMsg>::SELECTOR,
        <proto::ring_device as MethodMsg>::SELECTOR,
        <proto::stop_ringing as MethodMsg>::SELECTOR,
        <proto::locate as MethodMsg>::SELECTOR,
        <proto::local_ring as MethodMsg>::SELECTOR,
        <proto::stop_local_ring as MethodMsg>::SELECTOR,
        <proto::open_on_device as MethodMsg>::SELECTOR,
        <proto::DeviceConnected as EventMsg>::SELECTOR,
        <proto::DevicesChanged as EventMsg>::SELECTOR,
        <proto::PairingRequest as EventMsg>::SELECTOR,
        <proto::PairingResult as EventMsg>::SELECTOR,
        <proto::CallStateChanged as EventMsg>::SELECTOR,
        <proto::MediaChanged as EventMsg>::SELECTOR,
        <proto::BatteryChanged as EventMsg>::SELECTOR,
        <proto::HotspotChanged as EventMsg>::SELECTOR,
        <proto::FeatureChanged as EventMsg>::SELECTOR,
        <proto::FeatureFailed as EventMsg>::SELECTOR,
        <proto::MessageChanged as EventMsg>::SELECTOR,
        <proto::CollectionsChanged as EventMsg>::SELECTOR,
        <proto::CaptureFinished as EventMsg>::SELECTOR,
        <proto::TransferChanged as EventMsg>::SELECTOR,
        <proto::AudioRoutesChanged as EventMsg>::SELECTOR,
        <proto::FocusChanged as EventMsg>::SELECTOR,
        <proto::LiveActivitiesChanged as EventMsg>::SELECTOR,
        <proto::WifiReceived as EventMsg>::SELECTOR,
        <proto::LocalRingChanged as EventMsg>::SELECTOR,
    ];
    let declared = selectors.len();
    selectors.sort_unstable();
    selectors.dedup();
    assert_eq!(selectors.len(), declared);
}
