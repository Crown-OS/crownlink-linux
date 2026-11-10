use llts_signaling::state::{
    Chronometer as SignalingChronometer, LiveActivity as SignalingActivity,
    LiveProgress as SignalingProgress, NotificationAction,
};

use crate::ipc::proto::{Chronometer, LiveAction, LiveActivity, LiveProgress};

const fn ipc_progress(progress: SignalingProgress) -> LiveProgress {
    LiveProgress {
        value: progress.value,
        max: progress.max,
        indeterminate: progress.indeterminate,
    }
}

const fn ipc_chronometer(chronometer: SignalingChronometer) -> Chronometer {
    Chronometer {
        base_unix_ms: chronometer.base_unix_ms,
        counts_down: chronometer.counts_down,
    }
}

fn ipc_action(action: &NotificationAction<'_>) -> LiveAction {
    LiveAction {
        id: action.id.to_owned(),
        label: action.label.to_owned(),
        accepts_text: action.accepts_text,
    }
}

pub(crate) fn ipc_live_activity(activity: &SignalingActivity<'_>) -> LiveActivity {
    LiveActivity {
        id: activity.id.to_owned(),
        package: activity.package.to_owned(),
        app: activity.app.to_owned(),
        title: activity.title.to_owned(),
        text: activity.text.to_owned(),
        short_text: activity.short_text.map(str::to_owned),
        category: activity.category.map(str::to_owned),
        progress: activity.progress.map(ipc_progress),
        chronometer: activity.chronometer.map(ipc_chronometer),
        promoted: activity.promoted,
        color_argb: activity.color_argb,
        actions: activity.actions.iter().map(ipc_action).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_activity_keeps_every_field() {
        let activity = SignalingActivity {
            id: "0|timer|1",
            package: "com.clock",
            app: "Clock",
            title: "Timer",
            text: "Pasta",
            short_text: Some("4:59"),
            category: Some("stopwatch"),
            progress: Some(SignalingProgress {
                value: 1,
                max: 300,
                indeterminate: false,
            }),
            chronometer: Some(SignalingChronometer {
                base_unix_ms: 9,
                counts_down: true,
            }),
            promoted: true,
            color_argb: Some(0xFF00_FF00),
            actions: vec![NotificationAction {
                id: "stop",
                label: "Stop",
                accepts_text: false,
            }],
        };
        let mapped = ipc_live_activity(&activity);
        assert_eq!(mapped.short_text.as_deref(), Some("4:59"));
        assert_eq!(mapped.progress.map(|progress| progress.max), Some(300));
        assert_eq!(
            mapped.chronometer,
            Some(Chronometer {
                base_unix_ms: 9,
                counts_down: true
            })
        );
        assert_eq!(
            mapped.actions,
            [LiveAction {
                id: "stop".into(),
                label: "Stop".into(),
                accepts_text: false
            }]
        );
        assert!(mapped.promoted);
    }
}
