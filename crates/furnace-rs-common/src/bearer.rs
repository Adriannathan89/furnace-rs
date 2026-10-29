//! Shared strict Bearer field-value syntax for transport adapters.
use http::header::AUTHORIZATION;

/// Parses one strict Bearer authorization field without exposing credential errors.
#[doc(hidden)]
pub fn parse(headers: &http::HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next()?;

    if values.next().is_some() {
        return None;
    }

    let value = value.to_str().ok()?;
    // HTTP field-value OWS may surround the credentials. Inside them,
    // RFC 6750 section 2.1 requires 1*SP, not arbitrary whitespace.
    let value = value.trim_matches([' ', '\t']);
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim_start_matches(' ');
    let unpadded = token.trim_end_matches('=');

    if !scheme.eq_ignore_ascii_case("Bearer")
        || unpadded.is_empty()
        || !unpadded.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/')
        })
    {
        return None;
    }

    Some(token)
}
