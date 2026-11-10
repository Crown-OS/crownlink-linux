use crownos_ipc::Server;

use crate::ipc::proto;

/// An IPC event waiting to be broadcast to its subscribers.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OutgoingEvent {
    DeviceConnected(proto::DeviceConnected),
    DevicesChanged(proto::DevicesChanged),
    PairingRequest(proto::PairingRequest),
    PairingResult(proto::PairingResult),
    CallStateChanged(proto::CallStateChanged),
    MediaChanged(proto::MediaChanged),
    BatteryChanged(proto::BatteryChanged),
    HotspotChanged(proto::HotspotChanged),
    FeatureChanged(proto::FeatureChanged),
    FeatureFailed(proto::FeatureFailed),
    MessageChanged(proto::MessageChanged),
    CollectionsChanged(proto::CollectionsChanged),
    CaptureFinished(proto::CaptureFinished),
    TransferChanged(proto::TransferChanged),
    AudioRoutesChanged(proto::AudioRoutesChanged),
    FocusChanged(proto::FocusChanged),
    LiveActivitiesChanged(proto::LiveActivitiesChanged),
    WifiReceived(proto::WifiReceived),
    LocalRingChanged(proto::LocalRingChanged),
}

impl OutgoingEvent {
    pub(crate) fn emit(&self, server: &mut Server) -> Result<(), crownos_ipc::Error> {
        match self {
            Self::DeviceConnected(event) => server.emit(event),
            Self::DevicesChanged(event) => server.emit(event),
            Self::PairingRequest(event) => server.emit(event),
            Self::PairingResult(event) => server.emit(event),
            Self::CallStateChanged(event) => server.emit(event),
            Self::MediaChanged(event) => server.emit(event),
            Self::BatteryChanged(event) => server.emit(event),
            Self::HotspotChanged(event) => server.emit(event),
            Self::FeatureChanged(event) => server.emit(event),
            Self::FeatureFailed(event) => server.emit(event),
            Self::MessageChanged(event) => server.emit(event),
            Self::CollectionsChanged(event) => server.emit(event),
            Self::CaptureFinished(event) => server.emit(event),
            Self::TransferChanged(event) => server.emit(event),
            Self::AudioRoutesChanged(event) => server.emit(event),
            Self::FocusChanged(event) => server.emit(event),
            Self::LiveActivitiesChanged(event) => server.emit(event),
            Self::WifiReceived(event) => server.emit(event),
            Self::LocalRingChanged(event) => server.emit(event),
        }
    }
}
