//! Two daemons on loopback asking each other for phone data and opening links: every request
//! crosses the node's lanes and feature gates and comes back as an answer, a refusal or a
//! prompt, never as a silent wait.

#![cfg(feature = "daemon")]
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that fails a precondition should fail loudly"
)]

mod support;

use crownlink_linux::ipc::proto::{DeviceId, Feature, MessageDraft, PageRequest};
use crownos_ipc::Error;
use support::{pair_by_qr, received, wait_until, Daemon, Home, REPLY};

const REFUSED_BY_A_COMPUTER: &str =
    "the device cannot do that: a computer keeps no messages, call history or contacts";

fn daemon(home: &Home) -> Daemon {
    let (media, _requests) = std::sync::mpsc::channel();
    Daemon::start(home, Box::new(media))
}

fn remote_message<T: std::fmt::Debug>(outcome: Result<T, Error>) -> String {
    match outcome {
        Err(Error::Remote(remote)) => remote.message,
        other => panic!("expected a remote error, got {other:?}"),
    }
}

fn set_feature(daemon: &mut Daemon, peer: DeviceId, feature: Feature, enabled: bool) {
    let pending = daemon
        .client
        .set_feature(peer, feature, enabled)
        .expect("set_feature");
    daemon
        .client
        .wait(pending, REPLY)
        .expect("set_feature reply");
}

const fn first_page() -> PageRequest {
    PageRequest {
        cursor: None,
        limit: 50,
    }
}

fn conversations_error(daemon: &mut Daemon, peer: DeviceId) -> String {
    let pending = daemon
        .client
        .conversations(peer, first_page())
        .expect("conversations");
    remote_message(daemon.client.wait(pending, REPLY))
}

#[test]
fn phone_data_requests_and_links_cross_between_daemons() {
    let (alpha_home, beta_home) = Home::pair("alpha", "beta");
    let mut alpha = daemon(&alpha_home);
    let mut beta = daemon(&beta_home);
    let (beta_id, alpha_id) = pair_by_qr(&mut alpha, &mut beta);

    assert_eq!(
        conversations_error(&mut alpha, beta_id),
        "Messages is not shared with the device"
    );
    for feature in [
        Feature::Messages,
        Feature::CallHistory,
        Feature::Contacts,
        Feature::Links,
    ] {
        set_feature(&mut alpha, beta_id, feature, true);
        set_feature(&mut beta, alpha_id, feature, true);
    }
    wait_until("Messages to be shared", || {
        (conversations_error(&mut alpha, beta_id) == REFUSED_BY_A_COMPUTER).then_some(())
    });

    let pending = alpha
        .client
        .messages(beta_id, 4, first_page())
        .expect("messages");
    assert_eq!(
        remote_message(alpha.client.wait(pending, REPLY)),
        REFUSED_BY_A_COMPUTER
    );
    let pending = alpha.client.mark_read(beta_id, 4).expect("mark_read");
    assert_eq!(
        remote_message(alpha.client.wait(pending, REPLY)),
        REFUSED_BY_A_COMPUTER
    );
    let draft = MessageDraft {
        recipients: vec!["+15550100".to_owned()],
        text: "On my way".to_owned(),
        ..MessageDraft::default()
    };
    let pending = alpha
        .client
        .send_message(beta_id, draft.clone())
        .expect("send_message");
    assert_eq!(
        remote_message(alpha.client.wait(pending, REPLY)),
        REFUSED_BY_A_COMPUTER
    );
    let pending = alpha
        .client
        .send_message(beta_id, MessageDraft::default())
        .expect("send_message");
    assert_eq!(
        remote_message(alpha.client.wait(pending, REPLY)),
        "a message needs a conversation or at least one recipient"
    );
    let with_photo = MessageDraft {
        attachments: vec!["/tmp/photo.jpg".to_owned()],
        ..draft
    };
    let pending = alpha
        .client
        .send_message(beta_id, with_photo)
        .expect("send_message");
    assert!(remote_message(alpha.client.wait(pending, REPLY)).contains("blob lane"));

    wait_until("call history and contacts to be shared", || {
        let pending = alpha
            .client
            .call_log(beta_id, first_page())
            .expect("call_log");
        let call_log = remote_message(alpha.client.wait(pending, REPLY));
        let pending = alpha
            .client
            .contacts(beta_id, first_page())
            .expect("contacts");
        let contacts = remote_message(alpha.client.wait(pending, REPLY));
        let pending = alpha
            .client
            .voicemails(beta_id, first_page())
            .expect("voicemails");
        let voicemails = remote_message(alpha.client.wait(pending, REPLY));
        [call_log, contacts, voicemails]
            .iter()
            .all(|message| message == REFUSED_BY_A_COMPUTER)
            .then_some(())
    });

    let pending = alpha
        .client
        .open_on_device(beta_id, "file:///etc/passwd".into())
        .expect("open_on_device");
    assert_eq!(
        remote_message(alpha.client.wait(pending, REPLY)),
        "only http, https, mailto, geo, tel, sms and smsto links can be opened"
    );
    let pending = alpha
        .client
        .open_on_device(beta_id, "mailto:ada@example.org?attach=/etc/shadow".into())
        .expect("open_on_device");
    alpha
        .client
        .wait(pending, REPLY)
        .expect("the link was sent");
    let link = received("the link on beta", &mut beta.links);
    assert_eq!(link.uri(), "mailto:ada@example.org");
    assert_eq!(link.device, "alpha");

    set_feature(&mut beta, alpha_id, Feature::Messages, false);
    wait_until("Messages to stop being shared", || {
        (conversations_error(&mut alpha, beta_id) == "Messages is not shared with the device")
            .then_some(())
    });
}
