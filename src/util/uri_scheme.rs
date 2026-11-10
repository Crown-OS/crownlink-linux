//! The links a device may open on this computer, and this computer on a device: a short
//! allowlist of schemes, a length cap, and `mailto:` cut down to its address and text fields.

use std::borrow::Cow;

use llts_signaling::message::limits::{ALLOWED_URI_SCHEMES, MAX_URI_BYTES};

/// The `mailto:` fields that only address or prefill a message; `attach=` and every other
/// header are dropped.
const MAILTO_FIELDS: [&str; 5] = ["to", "cc", "bcc", "subject", "body"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UriError {
    #[error("a {len}-byte link exceeds the {MAX_URI_BYTES}-byte limit")]
    TooLong { len: usize },
    #[error("only http, https, mailto, geo, tel, sms and smsto links can be opened")]
    SchemeRefused,
    #[error("the link contains whitespace or control characters")]
    Unprintable,
}

/// `uri` as it may be opened: within the length cap, of an allowed scheme, printable, and for
/// `mailto:` without attachments or headers beyond the address and text fields.
///
/// # Errors
///
/// Fails when `uri` breaks any of those rules.
pub fn check(uri: &str) -> Result<Cow<'_, str>, UriError> {
    if uri.len() > MAX_URI_BYTES {
        return Err(UriError::TooLong { len: uri.len() });
    }
    if uri
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(UriError::Unprintable);
    }
    let (scheme, rest) = uri.split_once(':').ok_or(UriError::SchemeRefused)?;
    if !ALLOWED_URI_SCHEMES
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(scheme))
    {
        return Err(UriError::SchemeRefused);
    }
    if scheme.eq_ignore_ascii_case("mailto") {
        Ok(mailto(uri, scheme, rest))
    } else {
        Ok(Cow::Borrowed(uri))
    }
}

fn mailto<'a>(uri: &'a str, scheme: &str, rest: &str) -> Cow<'a, str> {
    let Some((address, query)) = rest.split_once('?') else {
        return Cow::Borrowed(uri);
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|field| {
            let name = field.split_once('=').map_or(*field, |(name, _)| name);
            MAILTO_FIELDS
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(name))
        })
        .collect();
    if kept.len() == query.split('&').count() {
        return Cow::Borrowed(uri);
    }
    let mut rebuilt = format!("{scheme}:{address}");
    if !kept.is_empty() {
        rebuilt.push('?');
        rebuilt.push_str(&kept.join("&"));
    }
    Cow::Owned(rebuilt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_schemes_pass_unchanged() {
        for uri in [
            "tel:+1-555-0100;ext=12",
            "SMS:+1555,+1666?body=hi%20there",
            "smsto:+1555",
            "https://crownos.org/a?b=c",
            "geo:52.5,13.4",
        ] {
            assert_eq!(check(uri), Ok(Cow::Borrowed(uri)));
        }
    }

    #[test]
    fn other_schemes_are_refused() {
        for uri in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "intent://x",
            "no scheme",
        ] {
            assert!(check(uri).is_err(), "{uri}");
        }
    }

    #[test]
    fn whitespace_and_control_characters_are_refused() {
        assert_eq!(check("tel:+1 555"), Err(UriError::Unprintable));
        assert_eq!(check("tel:+1555\n"), Err(UriError::Unprintable));
    }

    #[test]
    fn links_over_the_cap_are_refused() {
        let uri = format!("https://a/{}", "x".repeat(MAX_URI_BYTES));
        assert_eq!(check(&uri), Err(UriError::TooLong { len: uri.len() }));
        let fits = format!("tel:{}", "1".repeat(MAX_URI_BYTES - 4));
        assert!(check(&fits).is_ok());
    }

    #[test]
    fn mailto_keeps_only_address_and_text_fields() {
        assert_eq!(
            check("mailto:a@b.c?subject=Hi&attach=/etc/shadow&Body=x&X-Header=y"),
            Ok(Cow::Owned("mailto:a@b.c?subject=Hi&Body=x".to_owned()))
        );
        assert_eq!(
            check("mailto:a@b.c?attachment=/home/u/.ssh/id_ed25519"),
            Ok(Cow::Owned("mailto:a@b.c".to_owned()))
        );
        assert_eq!(
            check("mailto:a@b.c?cc=d@e.f"),
            Ok(Cow::Borrowed("mailto:a@b.c?cc=d@e.f"))
        );
        assert_eq!(check("mailto:a@b.c"), Ok(Cow::Borrowed("mailto:a@b.c")));
    }
}
