use llts_signaling::message as wire;

use crate::ipc::proto::PageRequest;

pub(crate) fn signaling_page(page: &PageRequest) -> wire::PageRequest<'_> {
    wire::PageRequest {
        cursor: page.cursor.as_deref(),
        limit: page.limit,
    }
}

const fn failure_words(error: wire::RequestError) -> &'static str {
    match error.known() {
        wire::RequestError::UNSUPPORTED => "the device cannot do that",
        wire::RequestError::PERMISSION_DENIED => "the device does not allow it",
        wire::RequestError::NOT_FOUND => "the device has no such item",
        wire::RequestError::BUSY => "the device is busy, try again",
        wire::RequestError::CANCELLED => "it was cancelled on the device",
        wire::RequestError::TOO_LARGE => "it is too large for the device to send",
        wire::RequestError::TIMEOUT => "the device timed out",
        _ => "the device could not do it",
    }
}

/// Why the device refused a request, in words for the user; the device's `detail` never
/// carries user data, so it is kept for context.
pub(crate) fn request_failure_reason(failed: &wire::RequestFailed<'_>) -> String {
    let words = failure_words(failed.error);
    if failed.detail.is_empty() {
        words.to_owned()
    } else {
        format!("{words}: {}", failed.detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_request_borrows_its_cursor() {
        let page = PageRequest {
            cursor: Some("1700000000000:12".into()),
            limit: 50,
        };
        assert_eq!(
            signaling_page(&page),
            wire::PageRequest {
                cursor: Some("1700000000000:12"),
                limit: 50,
            }
        );
    }

    #[test]
    fn failures_read_as_words_and_unknown_codes_as_a_plain_failure() {
        let failed = |error, detail| wire::RequestFailed {
            request: wire::RequestId(1),
            error,
            detail,
        };
        assert_eq!(
            request_failure_reason(&failed(
                wire::RequestError::UNSUPPORTED,
                "Mark as read needs the default SMS app"
            )),
            "the device cannot do that: Mark as read needs the default SMS app"
        );
        assert_eq!(
            request_failure_reason(&failed(wire::RequestError(200), "")),
            "the device could not do it"
        );
    }
}
