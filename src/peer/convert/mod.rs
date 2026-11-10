//! Crossings between the IPC schema and llts types, which describe the same things separately
//! so neither crate depends on the other.

mod cast;
mod contacts;
mod counters;
mod focus;
mod live;
mod messages;
mod phone;
mod request;

pub(crate) use cast::{ipc_audio_routes, signaling_audio_routes, start_audio, start_cast};
pub(crate) use contacts::ipc_contact_page;
pub(crate) use counters::{
    ipc_call_counters, ipc_contact_counters, ipc_message_counters, shared_collections,
};
pub(crate) use focus::{ipc_focus, signaling_focus_mode};
pub(crate) use live::ipc_live_activity;
pub(crate) use messages::{ipc_conversation_page, ipc_message_change, ipc_message_page};
pub(crate) use phone::{ipc_call_log_page, ipc_voicemail_page};
pub(crate) use request::{request_failure_reason, signaling_page};

use llts_node::FeatureState as NodeFeatureState;
use llts_signaling::device::{
    DeviceClass as SignalingClass, DeviceId as SignalingId, Feature as SignalingFeature,
    FeatureSet as SignalingFeatureSet,
};
use llts_signaling::message::{
    CameraLens as SignalingLens, Direction as SignalingDirection, Edge as SignalingEdge,
    StartCamera, StartMirror, StartMonitor, VideoLimits as SignalingLimits,
};

use crate::ipc::proto::{
    CameraLens, CameraOptions, DeviceClass, DeviceId, Direction, Edge, Feature, FeatureSet,
    FeatureState, MirrorOptions, MonitorOptions, VideoLimits,
};

/// A monitor is drawn at 1:1; scale is in 1/120 steps.
const UNSCALED: u16 = 120;
const MILLIHERTZ_PER_HERTZ: u32 = 1_000;

pub(crate) const fn signaling_id(id: DeviceId) -> SignalingId {
    SignalingId(id.0)
}

pub(crate) const fn ipc_id(id: SignalingId) -> DeviceId {
    DeviceId(id.0)
}

pub(crate) const fn signaling_feature(feature: Feature) -> SignalingFeature {
    match feature {
        Feature::Mirror => SignalingFeature::Mirror,
        Feature::Camera => SignalingFeature::Camera,
        Feature::Mic => SignalingFeature::Mic,
        Feature::Monitor => SignalingFeature::Monitor,
        Feature::Unicursor => SignalingFeature::Unicursor,
        Feature::Calls => SignalingFeature::Calls,
        Feature::Clipboard => SignalingFeature::Clipboard,
        Feature::Notifications => SignalingFeature::Notifications,
        Feature::Battery => SignalingFeature::Battery,
        Feature::Hotspot => SignalingFeature::Hotspot,
        Feature::Files => SignalingFeature::Files,
        Feature::Messages => SignalingFeature::Messages,
        Feature::CallHistory => SignalingFeature::CallHistory,
        Feature::Contacts => SignalingFeature::Contacts,
        Feature::Capture => SignalingFeature::Capture,
        Feature::DragDrop => SignalingFeature::DragDrop,
        Feature::Cast => SignalingFeature::Cast,
        Feature::AudioHandoff => SignalingFeature::AudioHandoff,
        Feature::Focus => SignalingFeature::Focus,
        Feature::LiveActivities => SignalingFeature::LiveActivities,
        Feature::WifiSharing => SignalingFeature::WifiSharing,
        Feature::FindMy => SignalingFeature::FindMy,
        Feature::Links => SignalingFeature::Links,
    }
}

pub(crate) const fn ipc_feature(feature: SignalingFeature) -> Feature {
    match feature {
        SignalingFeature::Mirror => Feature::Mirror,
        SignalingFeature::Camera => Feature::Camera,
        SignalingFeature::Mic => Feature::Mic,
        SignalingFeature::Monitor => Feature::Monitor,
        SignalingFeature::Unicursor => Feature::Unicursor,
        SignalingFeature::Calls => Feature::Calls,
        SignalingFeature::Clipboard => Feature::Clipboard,
        SignalingFeature::Notifications => Feature::Notifications,
        SignalingFeature::Battery => Feature::Battery,
        SignalingFeature::Hotspot => Feature::Hotspot,
        SignalingFeature::Files => Feature::Files,
        SignalingFeature::Messages => Feature::Messages,
        SignalingFeature::CallHistory => Feature::CallHistory,
        SignalingFeature::Contacts => Feature::Contacts,
        SignalingFeature::Capture => Feature::Capture,
        SignalingFeature::DragDrop => Feature::DragDrop,
        SignalingFeature::Cast => Feature::Cast,
        SignalingFeature::AudioHandoff => Feature::AudioHandoff,
        SignalingFeature::Focus => Feature::Focus,
        SignalingFeature::LiveActivities => Feature::LiveActivities,
        SignalingFeature::WifiSharing => Feature::WifiSharing,
        SignalingFeature::FindMy => Feature::FindMy,
        SignalingFeature::Links => Feature::Links,
    }
}

pub(crate) fn ipc_features(features: SignalingFeatureSet) -> FeatureSet {
    features.iter().map(ipc_feature).collect()
}

pub(crate) fn signaling_features(features: FeatureSet) -> SignalingFeatureSet {
    features.iter().map(signaling_feature).collect()
}

pub(crate) const fn ipc_class(class: SignalingClass) -> DeviceClass {
    match class {
        SignalingClass::Phone => DeviceClass::Phone,
        SignalingClass::Tablet => DeviceClass::Tablet,
        SignalingClass::Watch => DeviceClass::Watch,
        SignalingClass::Computer => DeviceClass::Computer,
    }
}

pub(crate) const fn ipc_feature_state(state: NodeFeatureState) -> FeatureState {
    match state {
        NodeFeatureState::Disabled => FeatureState::Disabled,
        NodeFeatureState::Enabled => FeatureState::Enabled,
        NodeFeatureState::Active => FeatureState::Active,
    }
}

pub(crate) const fn signaling_limits(limits: VideoLimits) -> SignalingLimits {
    SignalingLimits {
        max_width: limits.max_width,
        max_height: limits.max_height,
        max_fps: limits.max_fps,
    }
}

pub(crate) const fn signaling_edge(edge: Edge) -> SignalingEdge {
    match edge {
        Edge::Left => SignalingEdge::Left,
        Edge::Right => SignalingEdge::Right,
        Edge::Top => SignalingEdge::Top,
        Edge::Bottom => SignalingEdge::Bottom,
    }
}

pub(crate) const fn signaling_direction(direction: Direction) -> SignalingDirection {
    match direction {
        Direction::ToPeer => SignalingDirection::ToPeer,
        Direction::FromPeer => SignalingDirection::FromPeer,
    }
}

pub(crate) const fn start_mirror(options: MirrorOptions) -> StartMirror {
    StartMirror {
        direction: signaling_direction(options.direction),
        limits: signaling_limits(options.limits),
        remote_input: options.remote_input,
    }
}

pub(crate) const fn start_camera(options: CameraOptions) -> StartCamera {
    StartCamera {
        lens: match options.lens {
            CameraLens::Back => SignalingLens::Back,
            CameraLens::Front => SignalingLens::Front,
        },
        limits: signaling_limits(options.limits),
    }
}

pub(crate) fn start_monitor(options: MonitorOptions) -> StartMonitor {
    StartMonitor {
        width: options.limits.max_width,
        height: options.limits.max_height,
        refresh_mhz: u32::from(options.limits.max_fps) * MILLIHERTZ_PER_HERTZ,
        scale_120: UNSCALED,
        placement: signaling_edge(options.placement),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_feature_lists_name_the_same_features_in_the_same_order() {
        assert_eq!(Feature::ALL.len(), SignalingFeature::ALL.len());
        for (ipc, signaling) in Feature::ALL.into_iter().zip(SignalingFeature::ALL) {
            assert_eq!(signaling_feature(ipc), signaling);
            assert_eq!(ipc as u8, signaling.bit_index());
        }
    }

    #[test]
    fn mirrored_capability_bits_match_the_wire() {
        use llts_signaling::message::{CallCapabilities, ContactCapabilities, MessageCapabilities};

        use crate::ipc::proto::{CallCounters, ContactCounters, MessageCounters};

        assert_eq!(MessageCounters::SEND_SMS, MessageCapabilities::SEND_SMS.0);
        assert_eq!(MessageCounters::SEND_MMS, MessageCapabilities::SEND_MMS.0);
        assert_eq!(MessageCounters::MARK_READ, MessageCapabilities::MARK_READ.0);
        assert_eq!(
            CallCounters::VOICEMAIL_AUDIO,
            CallCapabilities::VOICEMAIL_AUDIO.0
        );
        assert_eq!(
            CallCounters::VOICEMAIL_TRANSCRIPTS,
            CallCapabilities::VOICEMAIL_TRANSCRIPTS.0
        );
        assert_eq!(ContactCounters::PHOTOS, ContactCapabilities::PHOTOS.0);
    }

    #[test]
    fn every_feature_crosses_both_ways() {
        for feature in Feature::ALL {
            assert_eq!(ipc_feature(signaling_feature(feature)), feature);
        }
        let all: SignalingFeatureSet = SignalingFeature::ALL.into_iter().collect();
        assert_eq!(ipc_features(all).iter().count(), Feature::ALL.len());
    }

    #[test]
    fn a_monitor_asks_for_its_limits_as_a_mode() {
        let monitor = start_monitor(MonitorOptions {
            placement: Edge::Left,
            limits: VideoLimits {
                max_width: 2560,
                max_height: 1600,
                max_fps: 60,
            },
        });
        assert_eq!(
            (monitor.width, monitor.height, monitor.refresh_mhz),
            (2560, 1600, 60_000)
        );
        assert_eq!(monitor.placement, SignalingEdge::Left);
    }
}
