use llts_signaling::message::{Direction as SignalingDirection, StartAudio, StartCast};
use llts_signaling::state::{AudioRoutes as SignalingRoutes, Headset as SignalingHeadset};

use super::{signaling_direction, signaling_limits};
use crate::ipc::proto::{AudioOptions, AudioRoutes, CastOptions, Headset};

/// The device casts one of its apps to this computer, so the stream flows from the peer.
pub(crate) const fn start_cast(options: CastOptions) -> StartCast {
    StartCast {
        direction: SignalingDirection::FromPeer,
        limits: signaling_limits(options.limits),
    }
}

pub(crate) const fn start_audio(options: AudioOptions) -> StartAudio {
    StartAudio {
        direction: signaling_direction(options.direction),
    }
}

pub(crate) fn ipc_audio_routes(routes: &SignalingRoutes<'_>) -> AudioRoutes {
    AudioRoutes {
        headsets: routes
            .headsets
            .iter()
            .map(|headset| Headset {
                address: headset.address.to_owned(),
                name: headset.name.to_owned(),
                connected: headset.connected,
            })
            .collect(),
    }
}

pub(crate) fn signaling_audio_routes(routes: &AudioRoutes) -> SignalingRoutes<'_> {
    SignalingRoutes {
        headsets: routes
            .headsets
            .iter()
            .map(|headset| SignalingHeadset {
                address: &headset.address,
                name: &headset.name,
                connected: headset.connected,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::proto::{Direction, VideoLimits};

    #[test]
    fn audio_routes_cross_both_ways() {
        let routes = AudioRoutes {
            headsets: vec![Headset {
                address: "AA:BB:CC:DD:EE:FF".into(),
                name: "Buds".into(),
                connected: true,
            }],
        };
        assert_eq!(ipc_audio_routes(&signaling_audio_routes(&routes)), routes);
    }

    #[test]
    fn a_cast_always_flows_from_the_device() {
        let limits = VideoLimits {
            max_width: 1920,
            max_height: 1080,
            max_fps: 60,
        };
        let cast = start_cast(CastOptions { limits });
        assert_eq!(cast.direction, SignalingDirection::FromPeer);
        assert_eq!(cast.limits.max_fps, 60);
        let audio = start_audio(AudioOptions {
            direction: Direction::ToPeer,
        });
        assert_eq!(audio.direction, SignalingDirection::ToPeer);
    }
}
