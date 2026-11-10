use llts_signaling::state::{
    AudioRoutes as RoutesPayload, Battery, Clipboard, ClipboardContent, Focus, Hotspot,
    HybridTimestamp, MediaSession, Published, StatePublisher, Topic, Volume,
};
use llts_signaling::SignalingError;

use crate::ipc::proto::{AudioRoutes, FocusInfo};
use crate::peer::convert::{signaling_audio_routes, signaling_focus_mode};

/// What a publisher owns and sends; the llts payloads borrow, so each has a `payload` view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalState {
    Battery(Battery),
    Volume(Volume),
    Media(Option<MediaSnapshot>),
    Clipboard(ClipboardSnapshot),
    Hotspot(HotspotSnapshot),
    Focus(FocusSnapshot),
    AudioRoutes(AudioRoutes),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSnapshot {
    pub app: String,
    pub title: String,
    pub artist: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardSnapshot {
    pub mime: String,
    pub bytes: Vec<u8>,
    pub stamp: HybridTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotspotSnapshot {
    pub enabled: bool,
    pub ssid: Option<String>,
}

/// This computer's focus, stamped by the runtime when it is published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusSnapshot {
    pub focus: FocusInfo,
    pub stamp: HybridTimestamp,
}

impl FocusSnapshot {
    pub fn payload(&self) -> Focus<'_> {
        Focus {
            mode: signaling_focus_mode(&self.focus.mode),
            until_unix_ms: self.focus.until_unix_ms,
            stamp: self.stamp,
        }
    }
}

impl MediaSnapshot {
    pub fn payload(&self) -> MediaSession<'_> {
        MediaSession {
            app: &self.app,
            title: &self.title,
            artist: &self.artist,
            position_ms: self.position_ms,
            duration_ms: self.duration_ms,
            playing: self.playing,
        }
    }
}

impl ClipboardSnapshot {
    /// # Errors
    ///
    /// Fails for content above the inline limit, which has to travel as an ffsp offer.
    pub fn payload(&self) -> Result<Clipboard<'_>, SignalingError> {
        Ok(Clipboard {
            mime: &self.mime,
            content: ClipboardContent::inline(&self.bytes)?,
            stamp: self.stamp,
        })
    }
}

impl HotspotSnapshot {
    pub fn payload(&self) -> Hotspot<'_> {
        Hotspot {
            enabled: self.enabled,
            ssid: self.ssid.as_deref(),
        }
    }
}

impl LocalState {
    pub const fn topic(&self) -> Topic {
        match self {
            Self::Battery(_) => Topic::Battery,
            Self::Volume(_) => Topic::Volume,
            Self::Media(_) => Topic::MediaSession,
            Self::Clipboard(_) => Topic::Clipboard,
            Self::Hotspot(_) => Topic::Hotspot,
            Self::Focus(_) => Topic::Focus,
            Self::AudioRoutes(_) => Topic::AudioRoutes,
        }
    }

    /// # Errors
    ///
    /// Fails when the value cannot be encoded, such as an oversized clipboard.
    pub fn publish_into(
        &self,
        publisher: &mut StatePublisher,
    ) -> Result<Published, SignalingError> {
        match self {
            Self::Battery(battery) => publisher.publish(battery),
            Self::Volume(volume) => publisher.publish(volume),
            Self::Media(media) => publisher.publish(&media.as_ref().map(MediaSnapshot::payload)),
            Self::Clipboard(clipboard) => publisher.publish(&clipboard.payload()?),
            Self::Hotspot(hotspot) => publisher.publish(&hotspot.payload()),
            Self::Focus(focus) => publisher.publish(&focus.payload()),
            Self::AudioRoutes(routes) => {
                publisher.publish::<RoutesPayload<'_>>(&signaling_audio_routes(routes))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use llts_signaling::state::Incarnation;

    use super::*;

    #[test]
    fn republishing_an_identical_value_is_a_no_op() -> Result<(), SignalingError> {
        let mut publisher = StatePublisher::new(Incarnation::from_raw(7));
        let hotspot = LocalState::Hotspot(HotspotSnapshot {
            enabled: true,
            ssid: Some("crown".into()),
        });
        assert!(matches!(
            hotspot.publish_into(&mut publisher)?,
            Published::Changed(_)
        ));
        assert_eq!(hotspot.publish_into(&mut publisher)?, Published::Unchanged);
        let published: Option<Hotspot<'_>> = publisher.get()?;
        assert_eq!(published.and_then(|hotspot| hotspot.ssid), Some("crown"));
        Ok(())
    }

    #[test]
    fn media_stopping_publishes_an_empty_session() -> Result<(), SignalingError> {
        let mut publisher = StatePublisher::new(Incarnation::from_raw(1));
        let playing = LocalState::Media(Some(MediaSnapshot {
            app: "Player".into(),
            title: "Song".into(),
            artist: "Band".into(),
            position_ms: 1,
            duration_ms: 2,
            playing: true,
        }));
        assert!(matches!(
            playing.publish_into(&mut publisher)?,
            Published::Changed(_)
        ));
        assert!(matches!(
            LocalState::Media(None).publish_into(&mut publisher)?,
            Published::Changed(_)
        ));
        assert_eq!(publisher.get::<Option<MediaSession<'_>>>()?, Some(None));
        Ok(())
    }

    #[test]
    fn focus_and_audio_routes_publish_under_their_topics() -> Result<(), SignalingError> {
        let mut publisher = StatePublisher::new(Incarnation::from_raw(3));
        let focus = LocalState::Focus(FocusSnapshot {
            focus: FocusInfo {
                mode: crate::ipc::proto::FocusMode::Named("Work".into()),
                until_unix_ms: None,
            },
            stamp: HybridTimestamp {
                wall_ms: 5,
                counter: 0,
            },
        });
        assert_eq!(focus.topic(), Topic::Focus);
        assert!(matches!(
            focus.publish_into(&mut publisher)?,
            Published::Changed(_)
        ));
        let published: Option<Focus<'_>> = publisher.get()?;
        assert_eq!(
            published.map(|focus| focus.mode),
            Some(llts_signaling::state::FocusMode::Named("Work"))
        );
        let routes = LocalState::AudioRoutes(AudioRoutes::default());
        assert_eq!(routes.topic(), Topic::AudioRoutes);
        assert!(matches!(
            routes.publish_into(&mut publisher)?,
            Published::Changed(_)
        ));
        Ok(())
    }

    #[test]
    fn oversized_clipboards_are_refused() {
        let mut publisher = StatePublisher::new(Incarnation::from_raw(1));
        let huge = LocalState::Clipboard(ClipboardSnapshot {
            mime: "text/plain".into(),
            bytes: vec![0; llts_signaling::state::MAX_INLINE_CLIPBOARD_BYTES + 1],
            stamp: HybridTimestamp::default(),
        });
        assert!(huge.publish_into(&mut publisher).is_err());
        assert_eq!(huge.topic(), Topic::Clipboard);
    }
}
