//! The wire contract of the `crownlink` service.
//!
//! Lives in the library target, outside the `daemon` feature, so every client
//! — crownotify, crownsettings, any other CrownOS app that talks to a paired
//! device — is generated from the same declaration as the daemon and cannot
//! drift from it.
//!
//! Frozen: a change to any signature changes its selector and breaks every
//! client built against the old one. Add new methods instead.

pub use super::types::*;

crownos_ipc::protocol! {
    pub mod crownlink: "crownlink" {
        /// Every device this computer has paired with or can currently see.
        fn devices() -> Vec<DeviceInfo>;
        /// Open a pairing window and return the QR invitation for it. Devices
        /// pairing nearby instead arrive as [`PairingRequest`].
        fn pairing_begin() -> PairingOffer;
        /// Accept or reject the device a [`PairingRequest`] asked about.
        fn pairing_confirm(id: DeviceId, accept: bool);
        /// Close the pairing window and invalidate its QR invitation.
        notify pairing_cancel();
        /// Unpair a device and delete its keys.
        fn forget(id: DeviceId);
        /// Trust a computer another authenticated channel vouched for, by its
        /// llts key, and dial it at `address` (`ip:port`). Returns its id.
        fn trust_computer(key: [u8; 32], name: String, address: String) -> DeviceId;
        /// This computer's own id and the UDP port llts listens on, for
        /// introducing it to a peer.
        fn local_identity() -> LocalIdentity;
        /// Allow or forbid one feature for one device.
        fn set_feature(id: DeviceId, feature: Feature, enabled: bool);
        /// The features the user allows for a device.
        fn features(id: DeviceId) -> FeatureSet;
        /// Offer the file at `path` to a device.
        fn send_file(id: DeviceId, path: String);

        /// Mirror a screen between this computer and a device.
        fn start_mirror(id: DeviceId, options: MirrorOptions);
        /// Use a device's camera as a camera on this computer.
        fn start_camera(id: DeviceId, options: CameraOptions);
        /// Use a device's microphone as a microphone on this computer.
        fn start_mic(id: DeviceId);
        /// Use a device as an extra monitor for this computer.
        fn start_monitor(id: DeviceId, options: MonitorOptions);
        /// Stop whatever `feature` is streaming with a device.
        fn stop(id: DeviceId, feature: Feature);
        /// Hand the pointer and keyboard to a device when the cursor crosses
        /// `edge` of this computer's desktop.
        fn set_unicursor_edge(id: DeviceId, edge: Edge);
        /// Give the virtual output a device shows as a monitor this mode, now if it is
        /// streaming and from its start otherwise.
        fn set_monitor_mode(id: DeviceId, mode: MonitorMode);
        /// Retitle the window this computer shows a device's screen or monitor in.
        fn set_viewer_title(id: DeviceId, title: String);

        /// Answer an incoming call.
        fn pickup_call(call_id: String);
        /// Decline an incoming call.
        fn decline_call(call_id: String);
        /// End a call in progress.
        fn hangup_call(call_id: String);
        /// Place a call from a device's phone line.
        fn dial(id: DeviceId, number: String);
        /// Resume playback on the device.
        notify media_play();
        /// Pause playback on the device.
        notify media_pause();
        /// Skip to the next track on the device.
        notify media_next();
        /// Go back to the previous track on the device.
        notify media_previous();
        /// Seek to a position in the current track, as a percentage.
        notify media_seek(percent: u8);
        /// Send an inline reply back to the conversation it came from.
        notify send_reply(conversation_id: String, text: String);

        /// The last snapshot of `topic` a device published, postcard-encoded
        /// as the llts-signaling state message for that topic. `None` until
        /// the device has published one.
        fn state(id: DeviceId, topic: Topic) -> Option<Vec<u8>>;
        /// Turn a device's mobile hotspot on or off.
        fn set_hotspot(id: DeviceId, enabled: bool);
        /// Pair with the device showing this QR invitation, as another computer's
        /// [`pairing_begin`] produced it. Phones scan the code instead.
        fn pair_with_qr(qr: String);

        /// One page of a device's conversations, newest first.
        fn conversations(id: DeviceId, page: PageRequest) -> ConversationPage;
        /// One page of one conversation's messages, newest first.
        fn messages(id: DeviceId, thread_id: u64, page: PageRequest) -> MessagePage;
        /// Send an SMS/MMS from a device; returns the phone's message id once it is queued.
        /// `draft.attachments` are local file paths, streamed to the phone after the request;
        /// a text over 32 KiB or more than 32 recipients or 16 attachments fails at once.
        fn send_message(id: DeviceId, draft: MessageDraft) -> String;
        /// Mark a conversation read on the device.
        fn mark_read(id: DeviceId, thread_id: u64);
        /// A blob a page referenced (attachment, thumbnail, contact photo, voicemail audio),
        /// downloaded into crownlink's cache; returns where it is. Fails unless `feature` is
        /// enabled for the device and was granted by it, even when the blob is cached.
        fn fetch_blob(id: DeviceId, feature: Feature, key: String) -> BlobFile;

        /// One page of a device's call log, newest first.
        fn call_log(id: DeviceId, page: PageRequest) -> CallLogPage;
        /// One page of a device's voicemails, newest first.
        fn voicemails(id: DeviceId, page: PageRequest) -> VoicemailPage;
        /// One page of a device's contacts, by display name.
        fn contacts(id: DeviceId, page: PageRequest) -> ContactPage;
        /// The device's counters and capabilities, one field per collection; a field is `None`
        /// until the device published it and whenever its feature is not shared.
        fn collections(id: DeviceId) -> Collections;

        /// Ask the device to capture; returns a capture id at once. The outcome arrives as
        /// `CaptureFinished` and stays readable through `capture_status`. For `Paste`, the
        /// window focused when this is called is the paste target; callers close their own
        /// popups first.
        fn capture(id: DeviceId, request: CaptureRequest) -> u32;
        /// Give up on a capture; the device closes its capture screen.
        notify cancel_capture(capture: u32);
        /// Where a capture this daemon started stands.
        fn capture_status(capture: u32) -> CaptureStatus;

        /// The files and blobs moving between this computer and its devices, newest last.
        fn transfers() -> Vec<Transfer>;

        /// Show one app of the device on this computer (the device picks the app).
        fn start_cast(id: DeviceId, options: CastOptions);
        /// Play this computer's audio on the device (`ToPeer`) or the device's here (`FromPeer`).
        fn start_audio(id: DeviceId, options: AudioOptions);
        /// Move a headset both devices are bonded with to `to`.
        fn move_headset(id: DeviceId, address: String, to: HeadsetOwner);
        /// The headsets a device could hand over; `None` until it published them.
        fn audio_routes(id: DeviceId) -> Option<AudioRoutes>;
        /// The headsets this computer could hand over.
        fn local_audio_routes() -> AudioRoutes;

        /// This computer's focus changed (called by crownotify); published to devices.
        fn set_focus(focus: FocusInfo);
        /// A device's focus mode; `None` until it published one.
        fn focus(id: DeviceId) -> Option<FocusInfo>;

        /// A device's ongoing activities, most important first.
        fn live_activities(id: DeviceId) -> Vec<LiveActivity>;
        /// Press a button of one of a device's live activities.
        fn live_activity_action(id: DeviceId, activity_id: String, action_id: String);

        /// Send this computer's saved credentials for `ssid` to the device, after the user
        /// confirms a crownlink notification and NetworkManager (polkit) releases the secret.
        fn share_wifi(id: DeviceId, ssid: String);
        /// Ask the device for `ssid`'s credentials; answered once the device's user approved
        /// and the network is saved (or its secret updated) here.
        fn request_wifi(id: DeviceId, ssid: String);

        /// Ring a device at full volume, even when it is silenced, until stopped.
        fn ring_device(id: DeviceId);
        /// Stop a device ringing.
        fn stop_ringing(id: DeviceId);
        /// Where a device is now, as its next position fix says.
        fn locate(id: DeviceId) -> DeviceLocation;
        /// Which device is ringing this computer, if any.
        fn local_ring() -> Option<DeviceId>;
        /// Stop this computer ringing.
        notify stop_local_ring();

        /// Open `uri` on the device (tel:, sms:, smsto:, http(s):, mailto:, geo:; ≤ 2048 bytes);
        /// the device asks its user before opening it.
        fn open_on_device(id: DeviceId, uri: String);

        /// A device connected or disconnected.
        event DeviceConnected { id: DeviceId, name: String, connected: bool };
        /// The device list changed; carries the whole new list.
        event DevicesChanged { devices: Vec<DeviceInfo> };
        /// A nearby device wants to pair. Show `code` and answer with
        /// `pairing_confirm` once the user has compared it with the device.
        event PairingRequest { id: DeviceId, name: String, code: String };
        /// A pairing finished, either way.
        event PairingResult { id: DeviceId, accepted: bool };
        /// A call on a device's phone line changed.
        event CallStateChanged { id: DeviceId, call: CallInfo };
        /// What a device is playing changed; `None` when it stopped playing.
        event MediaChanged { id: DeviceId, media: Option<MediaInfo> };
        /// A device's battery changed.
        event BatteryChanged { id: DeviceId, percent: u8, charging: bool };
        /// A device's hotspot was turned on or off.
        event HotspotChanged { id: DeviceId, enabled: bool };
        /// One feature of one device changed state.
        event FeatureChanged { id: DeviceId, feature: Feature, state: FeatureState };
        /// A feature could not start or its pipeline stopped on its own; `reason` says why in
        /// words for the user. A `FeatureChanged` with the resulting state follows.
        event FeatureFailed { id: DeviceId, feature: Feature, reason: String };
        /// A message arrived, changed or went away on a device; `messages` has the rest.
        event MessageChanged { id: DeviceId, change: MessageChange };
        /// A device's counters changed, or a feature stopped being shared and cleared them.
        event CollectionsChanged { id: DeviceId, collections: Collections };
        /// A capture ended, either way; `capture_status` keeps answering with `status`.
        event CaptureFinished { capture: u32, status: CaptureStatus };
        /// A transfer started, progressed or ended.
        event TransferChanged { transfer: Transfer };
        /// The headsets a device could hand over changed.
        event AudioRoutesChanged { id: DeviceId, routes: AudioRoutes };
        /// A device's focus mode changed.
        event FocusChanged { id: DeviceId, focus: FocusInfo };
        /// A device's ongoing activities changed; carries all of them.
        event LiveActivitiesChanged { id: DeviceId, activities: Vec<LiveActivity> };
        /// This computer's user chose to join a network a device shared.
        event WifiReceived { id: DeviceId, ssid: String };
        /// A device started or stopped ringing this computer.
        event LocalRingChanged { by: Option<DeviceId> };
        /// The window showing a device's screen or monitor was resized to this logical size.
        event ViewerResized { id: DeviceId, width: u16, height: u16 };
        /// The window on the monitor a device shows changed its title.
        event MonitorTitleChanged { id: DeviceId, title: String };
    }
}

pub use crownlink::*;
