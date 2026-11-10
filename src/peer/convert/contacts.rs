use llts_signaling::message as wire;

use crate::ipc::proto::{Contact, ContactPage, ContactPhone, PhoneKind};

const fn ipc_phone_kind(kind: wire::PhoneKind) -> PhoneKind {
    match kind {
        wire::PhoneKind::HOME => PhoneKind::Home,
        wire::PhoneKind::MOBILE => PhoneKind::Mobile,
        wire::PhoneKind::WORK => PhoneKind::Work,
        wire::PhoneKind::MAIN => PhoneKind::Main,
        wire::PhoneKind(other) => PhoneKind::Other(other),
    }
}

fn ipc_contact_phone(phone: &wire::ContactPhone<'_>) -> ContactPhone {
    ContactPhone {
        number: phone.number.to_owned(),
        kind: ipc_phone_kind(phone.kind),
        label: phone.label.map(str::to_owned),
    }
}

fn ipc_contact(contact: &wire::Contact<'_>) -> Contact {
    Contact {
        contact_key: contact.contact_key.to_owned(),
        name: contact.name.to_owned(),
        starred: contact.starred,
        phones: contact.phones.iter().map(ipc_contact_phone).collect(),
        emails: contact
            .emails
            .iter()
            .map(|email| (*email).to_owned())
            .collect(),
        organization: contact.organization.map(str::to_owned),
        photo_key: contact.photo_key.map(str::to_owned),
    }
}

pub(crate) fn ipc_contact_page(page: &wire::ContactPage<'_>) -> ContactPage {
    ContactPage {
        items: page.items.iter().map(ipc_contact).collect(),
        next: page.next.map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_kinds_map_and_unknown_ones_survive() {
        assert_eq!(ipc_phone_kind(wire::PhoneKind::HOME), PhoneKind::Home);
        assert_eq!(ipc_phone_kind(wire::PhoneKind::MOBILE), PhoneKind::Mobile);
        assert_eq!(ipc_phone_kind(wire::PhoneKind::WORK), PhoneKind::Work);
        assert_eq!(ipc_phone_kind(wire::PhoneKind::MAIN), PhoneKind::Main);
        assert_eq!(ipc_phone_kind(wire::PhoneKind::OTHER), PhoneKind::Other(7));
        assert_eq!(ipc_phone_kind(wire::PhoneKind(19)), PhoneKind::Other(19));
    }

    #[test]
    fn a_contact_page_keeps_every_field() {
        let page = wire::ContactPage {
            request: wire::RequestId(5),
            items: vec![wire::Contact {
                contact_key: "0r1-ABC",
                name: "Ada Lovelace",
                starred: true,
                phones: vec![wire::ContactPhone {
                    number: "+15550100",
                    kind: wire::PhoneKind(0),
                    label: Some("Lab"),
                }],
                emails: vec!["ada@example.org"],
                organization: Some("Analytical Engines"),
                photo_key: Some("contact-photo/1/2"),
            }],
            next: Some("lovelace:1"),
        };
        assert_eq!(
            ipc_contact_page(&page),
            ContactPage {
                items: vec![Contact {
                    contact_key: "0r1-ABC".into(),
                    name: "Ada Lovelace".into(),
                    starred: true,
                    phones: vec![ContactPhone {
                        number: "+15550100".into(),
                        kind: PhoneKind::Other(0),
                        label: Some("Lab".into()),
                    }],
                    emails: vec!["ada@example.org".into()],
                    organization: Some("Analytical Engines".into()),
                    photo_key: Some("contact-photo/1/2".into()),
                }],
                next: Some("lovelace:1".into()),
            }
        );
    }
}
