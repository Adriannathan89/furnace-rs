//! Request identity helpers shared by the cache and rate-limit layers.

use furnace_rs_common::axum::http::{HeaderMap, header};
use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Sorts query pairs so equivalent queries share one key.
pub(crate) fn canonical_query(query: Option<&str>) -> String {
    let mut pairs: Vec<&str> = query
        .unwrap_or("")
        .split('&')
        .filter(|p| !p.is_empty())
        .collect();
    pairs.sort_unstable();
    pairs.join("&")
}

/// Lowercase hex SHA-256 over `parts`, which never leaves a raw credential recoverable.
pub(crate) fn hex_sha256(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Hash of the `Authorization` and `Cookie` values, or `None` when both are absent.
pub(crate) fn fingerprint(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get(header::AUTHORIZATION);
    let cookie = headers.get(header::COOKIE);
    if auth.is_none() && cookie.is_none() {
        return None;
    }
    Some(hex_sha256(&[
        auth.map_or(&[][..], |v| v.as_bytes()),
        &[0],
        cookie.map_or(&[][..], |v| v.as_bytes()),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use furnace_rs_common::axum::http::{HeaderMap, HeaderValue, header};

    fn headers(auth: Option<&str>, cookie: Option<&str>) -> HeaderMap {
        let mut map = HeaderMap::new();
        if let Some(value) = auth {
            map.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        }
        if let Some(value) = cookie {
            map.insert(header::COOKIE, HeaderValue::from_str(value).unwrap());
        }
        map
    }

    #[test]
    fn canonical_query_sorts_pairs() {
        assert_eq!(canonical_query(Some("b=2&a=1")), "a=1&b=2");
        assert_eq!(canonical_query(None), "");
    }

    #[test]
    fn fingerprint_is_none_without_credentials() {
        assert_eq!(fingerprint(&headers(None, None)), None);
    }

    #[test]
    fn fingerprint_is_stable_hex_and_credential_specific() {
        let a = fingerprint(&headers(Some("Bearer A"), None)).unwrap();
        assert_eq!(a, fingerprint(&headers(Some("Bearer A"), None)).unwrap());
        assert_ne!(a, fingerprint(&headers(Some("Bearer B"), None)).unwrap());
        assert_ne!(a, fingerprint(&headers(None, Some("Bearer A"))).unwrap());
        assert_eq!(a.len(), 64);
        assert!(
            a.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
    }
}
