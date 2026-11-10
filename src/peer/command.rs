use llts_signaling::message::Media;

use crate::bluetooth::telephony::PhoneCalls;
use crate::discovery::PresenceEvent;
use crate::ipc::proto::{
    AudioOptions, BlobFile, CallLogPage, CameraOptions, CaptureRequest, CastOptions, ContactPage,
    ConversationPage, DeviceId, DeviceLocation, Edge, Feature, FeatureSet, FocusInfo, HeadsetOwner,
    MessageDraft, MessagePage, MirrorOptions, MonitorOptions, PageRequest, PairingOffer,
    VoicemailPage,
};
use crate::ipc::server::Responder;
use crate::state::LocalState;

/// Why this build refuses a request for `feature`, in words for the user.
pub fn unavailable_reason(feature: Feature) -> String {
    format!("{feature:?} is not available in this build of crownlink")
}

/// An IPC request that needs a paired device, with the responder that answers it.
#[derive(Debug)]
pub enum PeerRequest {
    PairingBegin(Responder<PairingOffer>),
    /// Answered once the pairing finished either way.
    PairWithQr {
        qr: String,
        reply: Responder<()>,
    },
    PairingConfirm {
        id: DeviceId,
        accept: bool,
        reply: Responder<()>,
    },
    Forget {
        id: DeviceId,
        reply: Responder<()>,
    },
    SendFile {
        id: DeviceId,
        path: String,
        reply: Responder<()>,
    },
    StartMirror {
        id: DeviceId,
        options: MirrorOptions,
        reply: Responder<()>,
    },
    StartCamera {
        id: DeviceId,
        options: CameraOptions,
        reply: Responder<()>,
    },
    StartMic {
        id: DeviceId,
        reply: Responder<()>,
    },
    StartMonitor {
        id: DeviceId,
        options: MonitorOptions,
        reply: Responder<()>,
    },
    Stop {
        id: DeviceId,
        feature: Feature,
        reply: Responder<()>,
    },
    SetUnicursorEdge {
        id: DeviceId,
        edge: Edge,
        reply: Responder<()>,
    },
    PickupCall {
        call_id: String,
        reply: Responder<()>,
    },
    DeclineCall {
        call_id: String,
        reply: Responder<()>,
    },
    HangupCall {
        call_id: String,
        reply: Responder<()>,
    },
    Dial {
        id: DeviceId,
        number: String,
        reply: Responder<()>,
    },
    SetHotspot {
        id: DeviceId,
        enabled: bool,
        reply: Responder<()>,
    },
    Conversations {
        id: DeviceId,
        page: PageRequest,
        reply: Responder<ConversationPage>,
    },
    Messages {
        id: DeviceId,
        thread_id: u64,
        page: PageRequest,
        reply: Responder<MessagePage>,
    },
    /// Answered with the device's id for the message once it is queued there.
    SendMessage {
        id: DeviceId,
        draft: MessageDraft,
        reply: Responder<String>,
    },
    MarkRead {
        id: DeviceId,
        thread_id: u64,
        reply: Responder<()>,
    },
    FetchBlob {
        id: DeviceId,
        feature: Feature,
        key: String,
        reply: Responder<BlobFile>,
    },
    CallLog {
        id: DeviceId,
        page: PageRequest,
        reply: Responder<CallLogPage>,
    },
    Voicemails {
        id: DeviceId,
        page: PageRequest,
        reply: Responder<VoicemailPage>,
    },
    Contacts {
        id: DeviceId,
        page: PageRequest,
        reply: Responder<ContactPage>,
    },
    /// Answered at once with the capture's id; the outcome follows as an event.
    Capture {
        id: DeviceId,
        request: CaptureRequest,
        reply: Responder<u32>,
    },
    StartCast {
        id: DeviceId,
        options: CastOptions,
        reply: Responder<()>,
    },
    StartAudio {
        id: DeviceId,
        options: AudioOptions,
        reply: Responder<()>,
    },
    MoveHeadset {
        id: DeviceId,
        address: String,
        to: HeadsetOwner,
        reply: Responder<()>,
    },
    /// This computer's own focus changed.
    SetFocus {
        focus: FocusInfo,
        reply: Responder<()>,
    },
    LiveActivityAction {
        id: DeviceId,
        activity_id: String,
        action_id: String,
        reply: Responder<()>,
    },
    ShareWifi {
        id: DeviceId,
        ssid: String,
        reply: Responder<()>,
    },
    RequestWifi {
        id: DeviceId,
        ssid: String,
        reply: Responder<()>,
    },
    RingDevice {
        id: DeviceId,
        reply: Responder<()>,
    },
    StopRinging {
        id: DeviceId,
        reply: Responder<()>,
    },
    Locate {
        id: DeviceId,
        reply: Responder<DeviceLocation>,
    },
    OpenOnDevice {
        id: DeviceId,
        uri: String,
        reply: Responder<()>,
    },
}

impl PeerRequest {
    /// Answers the request with a handler error.
    pub fn reject(self, reason: &str) {
        match self {
            Self::PairingBegin(reply) => reply.fail(reason),
            Self::PairWithQr { reply, .. }
            | Self::PairingConfirm { reply, .. }
            | Self::Forget { reply, .. }
            | Self::SendFile { reply, .. }
            | Self::StartMirror { reply, .. }
            | Self::StartCamera { reply, .. }
            | Self::StartMic { reply, .. }
            | Self::StartMonitor { reply, .. }
            | Self::Stop { reply, .. }
            | Self::SetUnicursorEdge { reply, .. }
            | Self::PickupCall { reply, .. }
            | Self::DeclineCall { reply, .. }
            | Self::HangupCall { reply, .. }
            | Self::Dial { reply, .. }
            | Self::SetHotspot { reply, .. }
            | Self::MarkRead { reply, .. }
            | Self::StartCast { reply, .. }
            | Self::StartAudio { reply, .. }
            | Self::MoveHeadset { reply, .. }
            | Self::SetFocus { reply, .. }
            | Self::LiveActivityAction { reply, .. }
            | Self::ShareWifi { reply, .. }
            | Self::RequestWifi { reply, .. }
            | Self::RingDevice { reply, .. }
            | Self::StopRinging { reply, .. }
            | Self::OpenOnDevice { reply, .. } => reply.fail(reason),
            Self::Conversations { reply, .. } => reply.fail(reason),
            Self::Messages { reply, .. } => reply.fail(reason),
            Self::SendMessage { reply, .. } => reply.fail(reason),
            Self::FetchBlob { reply, .. } => reply.fail(reason),
            Self::CallLog { reply, .. } => reply.fail(reason),
            Self::Voicemails { reply, .. } => reply.fail(reason),
            Self::Contacts { reply, .. } => reply.fail(reason),
            Self::Capture { reply, .. } => reply.fail(reason),
            Self::Locate { reply, .. } => reply.fail(reason),
        }
    }

    /// The feature a continuity request belongs to; `None` for the ones that predate them.
    pub const fn continuity_feature(&self) -> Option<Feature> {
        Some(match self {
            Self::Conversations { .. }
            | Self::Messages { .. }
            | Self::SendMessage { .. }
            | Self::MarkRead { .. } => Feature::Messages,
            Self::FetchBlob { feature, .. } => *feature,
            Self::CallLog { .. } | Self::Voicemails { .. } => Feature::CallHistory,
            Self::Contacts { .. } => Feature::Contacts,
            Self::Capture { .. } => Feature::Capture,
            Self::StartCast { .. } => Feature::Cast,
            Self::StartAudio { .. } | Self::MoveHeadset { .. } => Feature::AudioHandoff,
            Self::SetFocus { .. } => Feature::Focus,
            Self::LiveActivityAction { .. } => Feature::LiveActivities,
            Self::ShareWifi { .. } | Self::RequestWifi { .. } => Feature::WifiSharing,
            Self::RingDevice { .. } | Self::StopRinging { .. } | Self::Locate { .. } => {
                Feature::FindMy
            }
            Self::OpenOnDevice { .. } => Feature::Links,
            Self::PairingBegin(_)
            | Self::PairWithQr { .. }
            | Self::PairingConfirm { .. }
            | Self::Forget { .. }
            | Self::SendFile { .. }
            | Self::StartMirror { .. }
            | Self::StartCamera { .. }
            | Self::StartMic { .. }
            | Self::StartMonitor { .. }
            | Self::Stop { .. }
            | Self::SetUnicursorEdge { .. }
            | Self::PickupCall { .. }
            | Self::DeclineCall { .. }
            | Self::HangupCall { .. }
            | Self::Dial { .. }
            | Self::SetHotspot { .. } => return None,
        })
    }
}

/// A fire-and-forget IPC notification for the peer runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerNotice {
    PairingCancel,
    /// Drives the media session of whichever device is playing.
    Media(Media),
    SendReply {
        conversation_id: String,
        text: String,
    },
    /// The user changed what a device may use; the runtime persists and enforces it.
    FeaturesChanged {
        id: DeviceId,
        features: FeatureSet,
    },
    CancelCapture(u32),
    StopLocalRing,
}

/// Everything the peer runtime is told.
#[derive(Debug)]
pub enum PeerCommand {
    Request(PeerRequest),
    Notice(PeerNotice),
    /// This computer's own state changed and should reach subscribed peers.
    LocalState(LocalState),
    Presence(PresenceEvent),
    Calls(PhoneCalls),
}
