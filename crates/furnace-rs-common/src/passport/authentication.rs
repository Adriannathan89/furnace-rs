//! Strict Bearer extraction and sanitized, asynchronous authentication.
use super::{
    ErasedAuthentication, PassportContext, PassportError, PassportResult, PassportStrategyAdapter,
};
use furnace_rs_core::ApplicationContext;
use http::{HeaderMap, Method, Uri, header::AUTHORIZATION};

pub(crate) async fn authenticate_bearer(
    application: &ApplicationContext,
    adapter: PassportStrategyAdapter,
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    remote_addr: Option<std::net::SocketAddr>,
) -> PassportResult<ErasedAuthentication> {
    let token = bearer_token(headers)?;
    let context = PassportContext::new(headers, method, uri, remote_addr);
    adapter(application, &context, token).await
}

pub(crate) fn bearer_token(headers: &http::HeaderMap) -> PassportResult<&str> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let Some(value) = values.next() else {
        return Err(PassportError::reject());
    };
    if values.next().is_some() {
        return Err(PassportError::reject());
    }
    let value = value.to_str().map_err(|_| PassportError::reject())?;
    // HTTP field-value OWS may surround the credentials. Inside them,
    // RFC 6750 section 2.1 requires 1*SP, not arbitrary whitespace.
    let value = value.trim_matches([' ', '\t']);
    let Some((scheme, token)) = value.split_once(' ') else {
        return Err(PassportError::reject());
    };
    let token = token.trim_start_matches(' ');
    let unpadded = token.trim_end_matches('=');
    if !scheme.eq_ignore_ascii_case("Bearer")
        || unpadded.is_empty()
        || !unpadded.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/')
        })
    {
        return Err(PassportError::reject());
    }
    Ok(token)
}

#[cfg(test)]
mod bearer_syntax_tests {
    use super::bearer_token;
    use http::{HeaderMap, HeaderValue, header::AUTHORIZATION};

    #[test]
    fn rejects_non_bearer_token_characters_and_misplaced_padding() {
        for credential in [
            "Bearer opaque:token",
            "Bearer opaque,token",
            "Bearer =",
            "Bearer a=b",
            "Bearer a==b",
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_str(credential).unwrap());
            assert!(
                bearer_token(&headers).is_err(),
                "accepted malformed credentials"
            );
        }
    }

    #[test]
    fn accepts_bearer_alphabet_padding_and_outer_http_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("\t bEaReR   AZaz09-._~+/== \t"),
        );
        assert_eq!(bearer_token(&headers).unwrap(), "AZaz09-._~+/==");
    }

    #[test]
    fn rejects_duplicate_authorization_fields_in_either_order() {
        for values in [
            ["Bearer valid", "Bearer invalid"],
            ["Bearer invalid", "Bearer valid"],
        ] {
            let mut headers = HeaderMap::new();
            for value in values {
                headers.append(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
            }
            assert!(bearer_token(&headers).is_err());
        }
    }
}
