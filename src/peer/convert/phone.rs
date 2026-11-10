use llts_signaling::message as wire;

use crate::ipc::proto::{CallKind, CallLogEntry, CallLogPage, Voicemail, VoicemailPage};

const fn ipc_call_kind(kind: wire::CallKind) -> CallKind {
    match kind {
        wire::CallKind::INCOMING => CallKind::Incoming,
        wire::CallKind::OUTGOING => CallKind::Outgoing,
        wire::CallKind::MISSED => CallKind::Missed,
        wire::CallKind::VOICEMAIL => CallKind::Voicemail,
        wire::CallKind::REJECTED => CallKind::Rejected,
        wire::CallKind::BLOCKED => CallKind::Blocked,
        wire::CallKind::ANSWERED_EXTERNALLY => CallKind::AnsweredElsewhere,
        wire::CallKind(other) => CallKind::Other(other),
    }
}

fn ipc_call_log_entry(entry: &wire::CallLogEntry<'_>) -> CallLogEntry {
    CallLogEntry {
        entry_id: entry.entry_id.to_owned(),
        number: entry.number.to_owned(),
        name: entry.name.map(str::to_owned),
        contact_key: entry.contact_key.map(str::to_owned),
        kind: ipc_call_kind(entry.kind),
        start_unix_ms: entry.start_unix_ms,
        duration_s: entry.duration_s,
        is_new: entry.is_new,
        sim_slot: entry.sim_slot,
    }
}

pub(crate) fn ipc_call_log_page(page: &wire::CallLogPage<'_>) -> CallLogPage {
    CallLogPage {
        items: page.items.iter().map(ipc_call_log_entry).collect(),
        next: page.next.map(str::to_owned),
    }
}

fn ipc_voicemail(voicemail: &wire::Voicemail<'_>) -> Voicemail {
    Voicemail {
        voicemail_id: voicemail.voicemail_id.to_owned(),
        number: voicemail.number.to_owned(),
        name: voicemail.name.map(str::to_owned),
        received_unix_ms: voicemail.received_unix_ms,
        duration_s: voicemail.duration_s,
        heard: voicemail.heard,
        transcript: voicemail.transcript.map(str::to_owned),
        transcript_truncated: voicemail.transcript_truncated,
        audio_key: voicemail.audio_key.map(str::to_owned),
    }
}

pub(crate) fn ipc_voicemail_page(page: &wire::VoicemailPage<'_>) -> VoicemailPage {
    VoicemailPage {
        items: page.items.iter().map(ipc_voicemail).collect(),
        next: page.next.map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_call_kind_maps_and_unknown_ones_survive() {
        let kinds = [
            CallKind::Incoming,
            CallKind::Outgoing,
            CallKind::Missed,
            CallKind::Voicemail,
            CallKind::Rejected,
            CallKind::Blocked,
            CallKind::AnsweredElsewhere,
        ];
        for (raw, kind) in (1..).zip(kinds) {
            assert_eq!(ipc_call_kind(wire::CallKind(raw)), kind);
        }
        assert_eq!(ipc_call_kind(wire::CallKind(0)), CallKind::Other(0));
        assert_eq!(ipc_call_kind(wire::CallKind(99)), CallKind::Other(99));
    }

    #[test]
    fn a_call_log_page_keeps_every_field() {
        let page = wire::CallLogPage {
            request: wire::RequestId(3),
            items: vec![wire::CallLogEntry {
                entry_id: "17",
                number: "+15550100",
                name: Some("Ada"),
                contact_key: Some("0r1-ABC"),
                kind: wire::CallKind::MISSED,
                start_unix_ms: 1_700_000_000_000,
                duration_s: 0,
                is_new: true,
                sim_slot: Some(0),
            }],
            next: Some("1700000000000:17"),
        };
        assert_eq!(
            ipc_call_log_page(&page),
            CallLogPage {
                items: vec![CallLogEntry {
                    entry_id: "17".into(),
                    number: "+15550100".into(),
                    name: Some("Ada".into()),
                    contact_key: Some("0r1-ABC".into()),
                    kind: CallKind::Missed,
                    start_unix_ms: 1_700_000_000_000,
                    duration_s: 0,
                    is_new: true,
                    sim_slot: Some(0),
                }],
                next: Some("1700000000000:17".into()),
            }
        );
    }

    #[test]
    fn a_voicemail_page_keeps_every_field() {
        let page = wire::VoicemailPage {
            request: wire::RequestId(4),
            items: vec![wire::Voicemail {
                voicemail_id: "voicemail/5/1700000000000",
                number: "+15550100",
                name: None,
                received_unix_ms: 1_700_000_000_000,
                duration_s: 31,
                heard: false,
                transcript: Some("Call me back"),
                transcript_truncated: true,
                audio_key: None,
            }],
            next: None,
        };
        assert_eq!(
            ipc_voicemail_page(&page),
            VoicemailPage {
                items: vec![Voicemail {
                    voicemail_id: "voicemail/5/1700000000000".into(),
                    number: "+15550100".into(),
                    name: None,
                    received_unix_ms: 1_700_000_000_000,
                    duration_s: 31,
                    heard: false,
                    transcript: Some("Call me back".into()),
                    transcript_truncated: true,
                    audio_key: None,
                }],
                next: None,
            }
        );
    }
}
