#![cfg(feature = "daemon")]
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that fails a precondition should fail loudly"
)]

use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crownlink_linux::ipc::proto::{
    self, AudioOptions, Battery, CaptureDestination, CaptureKind, CaptureRequest, CastOptions,
    Collections, ContactCounters, DeviceClass, DeviceId, DeviceInfo, Direction, Feature,
    FeatureSet, FocusInfo, FocusMode, HeadsetOwner, LinkKind, MessageDraft, MirrorOptions,
    PageRequest, PairingOffer, ScanFormat, Topic, VideoLimits,
};
use crownlink_linux::ipc::server::{DaemonEvent, IpcServer, OwnedUpdate};
use crownlink_linux::peer::{PeerCommand, PeerNotice, PeerRequest};
use crownos_ipc::{Error, RemoteError};
use llts_signaling::message::Media;
use llts_signaling::state::{Battery as BatteryState, Incarnation, TopicPayload, Version};
use tokio::sync::mpsc;

const TIMEOUT: Option<Duration> = Some(Duration::from_secs(5));
const PHONE: DeviceId = DeviceId([7; 32]);
const STRANGER: DeviceId = DeviceId([9; 32]);
const NO_SESSION: &str = "no session with that device";

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

fn socket_directory(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("crownlink-server-{}-{name}", std::process::id()))
}

/// Answers pairing, refuses streaming, and reports every notice it receives.
async fn fake_runtime(
    mut commands: mpsc::Receiver<PeerCommand>,
    notices: std_mpsc::Sender<PeerNotice>,
) {
    while let Some(command) = commands.recv().await {
        match command {
            PeerCommand::Request(PeerRequest::PairingBegin(reply)) => reply.reply(PairingOffer {
                qr: "crownlink:test".into(),
                expires_unix_ms: 5,
            }),
            PeerCommand::Request(request) => request.reject(NO_SESSION),
            PeerCommand::Notice(notice) => {
                let _ = notices.send(notice);
            }
            _ => {}
        }
    }
}

enum Runtime {
    Fake(std_mpsc::Sender<PeerNotice>),
    Stopped,
}

struct Daemon {
    events: mpsc::Sender<DaemonEvent>,
    thread: JoinHandle<()>,
}

fn spawn_daemon(directory: PathBuf, runtime: Runtime) -> Daemon {
    let (ready, started) = std_mpsc::channel();
    let thread = std::thread::spawn(move || {
        let tokio = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        tokio.block_on(async move {
            let (peer, commands) = mpsc::channel(8);
            let (events, event_receiver) = mpsc::channel(8);
            let server = IpcServer::bind(Some(&directory), peer, event_receiver).expect("bind");
            match runtime {
                Runtime::Fake(notices) => {
                    tokio::spawn(fake_runtime(commands, notices));
                }
                Runtime::Stopped => drop(commands),
            }
            ready.send(events).expect("ready");
            server.run().await.expect("serve");
        });
    });
    let events = started.recv().expect("daemon started");
    Daemon { events, thread }
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

fn battery_update(percent: u8) -> OwnedUpdate {
    let battery = BatteryState {
        percent,
        charging: false,
        time_to_empty_min: Some(90),
    };
    OwnedUpdate {
        topic: BatteryState::TOPIC,
        version: Version {
            incarnation: Incarnation::from_raw(3),
            seq: 1,
        },
        payload: postcard::to_stdvec(&battery).expect("encode"),
    }
}

fn handler_error<T: std::fmt::Debug>(outcome: Result<T, Error>) -> RemoteError {
    match outcome {
        Err(Error::Remote(remote)) => remote,
        other => panic!("expected a remote error, got {other:?}"),
    }
}

const MIRROR: MirrorOptions = MirrorOptions {
    direction: Direction::FromPeer,
    limits: VideoLimits {
        max_width: 1920,
        max_height: 1080,
        max_fps: 60,
    },
    remote_input: true,
};

#[test]
fn the_server_caches_runtime_events_and_routes_requests_to_the_runtime() {
    let directory = socket_directory("fake");
    let (notices, notice_receiver) = std_mpsc::channel();
    let daemon = spawn_daemon(directory.clone(), Runtime::Fake(notices));
    let mut client = proto::Client::connect_in(&directory).expect("connect");
    client
        .subscribe_blocking::<proto::DevicesChanged>(TIMEOUT)
        .expect("subscribe devices");
    client
        .subscribe_blocking::<proto::BatteryChanged>(TIMEOUT)
        .expect("subscribe battery");
    client
        .subscribe_blocking::<proto::FeatureChanged>(TIMEOUT)
        .expect("subscribe features");

    daemon
        .events
        .blocking_send(DaemonEvent::DeviceUpdated(phone()))
        .expect("device");
    match next_event(&mut client) {
        proto::Event::DevicesChanged(changed) => assert_eq!(changed.devices, [phone()]),
        other => panic!("unexpected event {other:?}"),
    }

    let update = battery_update(55);
    daemon
        .events
        .blocking_send(DaemonEvent::StateUpdate {
            owner: PHONE,
            update: update.clone(),
        })
        .expect("battery");
    match next_event(&mut client) {
        proto::Event::BatteryChanged(changed) => {
            assert_eq!(
                (changed.id, changed.percent, changed.charging),
                (PHONE, 55, false)
            );
        }
        other => panic!("unexpected event {other:?}"),
    }
    let pending = client.state(PHONE, Topic::Battery).expect("state");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("state reply"),
        Some(update.payload)
    );
    let pending = client.devices().expect("devices");
    let devices = client.wait(pending, TIMEOUT).expect("devices reply");
    assert_eq!(
        devices.first().and_then(|device| device.battery),
        Some(Battery {
            percent: 55,
            charging: false
        })
    );

    let pending = client
        .set_feature(PHONE, Feature::Camera, true)
        .expect("set_feature");
    client.wait(pending, TIMEOUT).expect("set_feature reply");
    assert!(matches!(
        next_event(&mut client),
        proto::Event::FeatureChanged(_)
    ));
    let pending = client.features(PHONE).expect("features");
    assert!(client
        .wait(pending, TIMEOUT)
        .expect("features reply")
        .contains(Feature::Camera));
    let pending = client.features(STRANGER).expect("features");
    assert_eq!(
        handler_error(client.wait(pending, TIMEOUT)).code,
        RemoteError::HANDLER
    );

    let pending = client.pairing_begin().expect("pairing_begin");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("offer").qr,
        "crownlink:test"
    );
    let pending = client.start_mirror(PHONE, MIRROR).expect("start_mirror");
    assert_eq!(
        handler_error(client.wait(pending, TIMEOUT)).message,
        NO_SESSION
    );

    client.media_seek(42).expect("seek");
    client.pairing_cancel().expect("cancel");
    let received: Vec<PeerNotice> = (0..3)
        .map(|_| {
            notice_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("notice")
        })
        .collect();
    assert!(received.contains(&PeerNotice::Media(Media::Seek { percent: 42 })));
    assert!(received.contains(&PeerNotice::PairingCancel));
    assert!(received.iter().any(|notice| matches!(
        notice,
        PeerNotice::FeaturesChanged { id, features } if *id == PHONE && features.contains(Feature::Camera)
    )));

    drop(daemon.events);
    daemon.thread.join().expect("daemon thread");
}

#[test]
fn without_a_peer_runtime_peer_requests_fail_with_a_clear_error() {
    let directory = socket_directory("stopped");
    let daemon = spawn_daemon(directory.clone(), Runtime::Stopped);
    let mut client = proto::Client::connect_in(&directory).expect("connect");

    let pending = client.pairing_begin().expect("pairing_begin");
    let error = handler_error(client.wait(pending, TIMEOUT));
    assert_eq!(error.code, RemoteError::HANDLER);
    assert!(error.message.starts_with("peer runtime unavailable"));

    let pending = client.devices().expect("devices");
    assert!(client
        .wait(pending, TIMEOUT)
        .expect("devices reply")
        .is_empty());

    drop(daemon.events);
    daemon.thread.join().expect("daemon thread");
}

fn assert_routed_to_runtime<T: std::fmt::Debug>(outcome: Result<T, Error>) {
    assert_eq!(handler_error(outcome).message, NO_SESSION);
}

#[test]
fn every_continuity_method_reaches_the_runtime_or_the_cache() {
    let directory = socket_directory("continuity");
    let (notices, notice_receiver) = std_mpsc::channel();
    let daemon = spawn_daemon(directory.clone(), Runtime::Fake(notices));
    let mut client = proto::Client::connect_in(&directory).expect("connect");
    client
        .subscribe_blocking::<proto::CollectionsChanged>(TIMEOUT)
        .expect("subscribe collections");
    daemon
        .events
        .blocking_send(DaemonEvent::DeviceUpdated(phone()))
        .expect("device");

    let page = PageRequest {
        cursor: None,
        limit: 50,
    };
    let pending = client
        .conversations(PHONE, page.clone())
        .expect("conversations");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.messages(PHONE, 3, page.clone()).expect("messages");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .send_message(PHONE, MessageDraft::default())
        .expect("send_message");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.mark_read(PHONE, 3).expect("mark_read");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .fetch_blob(PHONE, Feature::Messages, "mms-part/1/2".into())
        .expect("fetch_blob");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.call_log(PHONE, page.clone()).expect("call_log");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.voicemails(PHONE, page.clone()).expect("voicemails");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.contacts(PHONE, page).expect("contacts");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let capture = CaptureRequest {
        kind: CaptureKind::Scan(ScanFormat::Pdf),
        destination: CaptureDestination::Paste,
    };
    let pending = client.capture(PHONE, capture).expect("capture");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .start_cast(
            PHONE,
            CastOptions {
                limits: MIRROR.limits,
            },
        )
        .expect("start_cast");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .start_audio(
            PHONE,
            AudioOptions {
                direction: Direction::FromPeer,
            },
        )
        .expect("start_audio");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .move_headset(PHONE, "AA:BB:CC:DD:EE:FF".into(), HeadsetOwner::Computer)
        .expect("move_headset");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let focus = FocusInfo {
        mode: FocusMode::DoNotDisturb,
        until_unix_ms: None,
    };
    let pending = client.set_focus(focus).expect("set_focus");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .live_activity_action(PHONE, "timer".into(), "stop".into())
        .expect("live_activity_action");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.share_wifi(PHONE, "home".into()).expect("share_wifi");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .request_wifi(PHONE, "home".into())
        .expect("request_wifi");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.ring_device(PHONE).expect("ring_device");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.stop_ringing(PHONE).expect("stop_ringing");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client.locate(PHONE).expect("locate");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));
    let pending = client
        .open_on_device(PHONE, "tel:+15550100".into())
        .expect("open_on_device");
    assert_routed_to_runtime(client.wait(pending, TIMEOUT));

    let pending = client.focus(PHONE).expect("focus");
    assert_eq!(client.wait(pending, TIMEOUT).expect("focus reply"), None);
    let pending = client.live_activities(PHONE).expect("live_activities");
    assert!(client
        .wait(pending, TIMEOUT)
        .expect("live_activities reply")
        .is_empty());
    let pending = client.audio_routes(PHONE).expect("audio_routes");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("audio_routes reply"),
        None
    );
    let pending = client.local_ring().expect("local_ring");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("local_ring reply"),
        None
    );
    let pending = client.focus(STRANGER).expect("focus");
    assert_eq!(
        handler_error(client.wait(pending, TIMEOUT)).code,
        RemoteError::HANDLER
    );
    for outcome in [
        client
            .transfers()
            .and_then(|pending| client.wait(pending, TIMEOUT).map(drop)),
        client
            .capture_status(1)
            .and_then(|pending| client.wait(pending, TIMEOUT).map(drop)),
        client
            .local_audio_routes()
            .and_then(|pending| client.wait(pending, TIMEOUT).map(drop)),
    ] {
        assert!(handler_error(outcome)
            .message
            .ends_with("is not available in this build of crownlink"));
    }

    let pending = client.collections(PHONE).expect("collections");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("collections reply"),
        Collections::default()
    );
    let collections = Collections {
        contacts: Some(ContactCounters {
            revision: 1,
            count: 240,
            capabilities: ContactCounters::PHOTOS,
        }),
        ..Collections::default()
    };
    daemon
        .events
        .blocking_send(DaemonEvent::Collections {
            id: PHONE,
            collections,
        })
        .expect("collections");
    match next_event(&mut client) {
        proto::Event::CollectionsChanged(changed) => {
            assert_eq!((changed.id, changed.collections), (PHONE, collections));
        }
        other => panic!("unexpected event {other:?}"),
    }
    let pending = client.collections(PHONE).expect("collections");
    assert_eq!(
        client.wait(pending, TIMEOUT).expect("collections reply"),
        collections
    );

    client.cancel_capture(9).expect("cancel_capture");
    client.stop_local_ring().expect("stop_local_ring");
    let received: Vec<PeerNotice> = (0..2)
        .map(|_| {
            notice_receiver
                .recv_timeout(Duration::from_secs(5))
                .expect("notice")
        })
        .collect();
    assert!(received.contains(&PeerNotice::CancelCapture(9)));
    assert!(received.contains(&PeerNotice::StopLocalRing));

    drop(daemon.events);
    daemon.thread.join().expect("daemon thread");
}
