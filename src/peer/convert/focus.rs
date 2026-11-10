use llts_signaling::state::{Focus, FocusMode as SignalingFocusMode};

use crate::ipc::proto::{FocusInfo, FocusMode};

pub(crate) fn ipc_focus(focus: &Focus<'_>) -> FocusInfo {
    FocusInfo {
        mode: match focus.mode {
            SignalingFocusMode::Off => FocusMode::Off,
            SignalingFocusMode::DoNotDisturb => FocusMode::DoNotDisturb,
            SignalingFocusMode::Named(name) => FocusMode::Named(name.to_owned()),
        },
        until_unix_ms: focus.until_unix_ms,
    }
}

pub(crate) fn signaling_focus_mode(mode: &FocusMode) -> SignalingFocusMode<'_> {
    match mode {
        FocusMode::Off => SignalingFocusMode::Off,
        FocusMode::DoNotDisturb => SignalingFocusMode::DoNotDisturb,
        FocusMode::Named(name) => SignalingFocusMode::Named(name),
    }
}

#[cfg(test)]
mod tests {
    use llts_signaling::state::HybridTimestamp;

    use super::*;

    #[test]
    fn every_focus_mode_crosses_both_ways() {
        let modes = [
            FocusMode::Off,
            FocusMode::DoNotDisturb,
            FocusMode::Named("Work".into()),
        ];
        for mode in modes {
            let focus = Focus {
                mode: signaling_focus_mode(&mode),
                until_unix_ms: Some(42),
                stamp: HybridTimestamp::default(),
            };
            assert_eq!(
                ipc_focus(&focus),
                FocusInfo {
                    mode: mode.clone(),
                    until_unix_ms: Some(42)
                }
            );
        }
    }
}
