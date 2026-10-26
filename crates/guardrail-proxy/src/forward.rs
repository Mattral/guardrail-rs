//! Upstream request forwarding.
//!
//! Once the pipeline allows (or redacts) a request, [`forward_request`] sends
//! the body upstream. Responses are streamed back by the handler.

use bytes::Bytes;
use guardrail_core::GuardrailError;
use reqwest::{Client, Response};
use std::time::Duration;

/// Forward a (possibly redacted) request body to the upstream LLM API.
///
/// # Arguments
///
/// * `client` — a shared [`reqwest::Client`] (connection-pooled).
/// * `upstream_url` — base URL of the upstream, e.g. `https://api.openai.com`.
/// * `path` — the request path, e.g. `/v1/chat/completions`.
/// * `body` — the JSON body to send (see
///   [`crate::translate::ScannedBody::apply`]).
/// * `headers` — headers from the original client request to forward
///   (see [`forwardable_headers`]).
/// * `timeout` — per-request timeout.
///
/// # Errors
///
/// Returns [`GuardrailError::Upstream`] if the request fails to send or the
/// connection is refused/times out.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_proxy::forward::forward_request;
/// use reqwest::Client;
/// use std::time::Duration;
///
/// # tokio_test::block_on(async {
/// let client = Client::new();
/// let body = serde_json::json!({"model": "gpt-4o", "messages": []});
/// let resp = forward_request(
///     &client,
///     "https://api.openai.com",
///     "/v1/chat/completions",
///     &body,
///     vec![("authorization".to_string(), "Bearer sk-...".to_string())],
///     Duration::from_secs(60),
/// ).await;
/// # });
/// ```
pub async fn forward_request(
    client: &Client,
    upstream_url: &str,
    path: &str,
    body: &serde_json::Value,
    headers: Vec<(String, String)>,
    timeout: Duration,
) -> Result<Response, GuardrailError> {
    let url = format!("{}{}", upstream_url.trim_end_matches('/'), path);

    let mut builder = client.post(&url).json(body).timeout(timeout);

    for (key, value) in headers {
        builder = builder.header(key, value);
    }

    builder.send().await.map_err(GuardrailError::from)
}

/// Read the full response body as bytes.
///
/// Used for non-streaming responses where the entire body is buffered before
/// returning to the client.
///
/// # Errors
///
/// Returns [`GuardrailError::Upstream`] if reading the body fails.
pub async fn read_body(response: Response) -> Result<Bytes, GuardrailError> {
    response.bytes().await.map_err(GuardrailError::from)
}

/// Hop-by-hop headers (RFC 9110, section 7.6.1). They describe one connection,
/// not the message, so a proxy must not pass them on in either direction.
/// Any header named in a `Connection` header is hop-by-hop too; see
/// [`forwardable_headers`].
pub const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "proxy-connection",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// Headers stripped from the **request** before forwarding upstream, in
/// addition to [`HOP_BY_HOP_HEADERS`].
///
/// - `host` is recomputed for the upstream's own hostname.
/// - `content-length` is recomputed because redaction can change the body.
/// - `accept-encoding` is left to the proxy's HTTP client, which only asks
///   for encodings it can decode. Passing the client's value on would let
///   the upstream compress the body with an encoding the proxy can't read,
///   which would hide its text from response redaction.
/// - `x-guardrail-key` is guardrail-rs's own caller secret and must never
///   reach the upstream provider.
pub const STRIPPED_REQUEST_HEADERS: &[&str] = &[
    "host",
    "content-length",
    "accept-encoding",
    "x-guardrail-key",
];

/// Headers stripped from the upstream **response** before it goes back to
/// the client, in addition to [`HOP_BY_HOP_HEADERS`]. `content-length` is
/// recomputed by the proxy's HTTP server, since redaction can change the
/// body.
pub const STRIPPED_RESPONSE_HEADERS: &[&str] = &["content-length"];

/// The headers of `headers` that may be forwarded, as lowercase name/value
/// pairs: everything except [`HOP_BY_HOP_HEADERS`], headers named in a
/// `Connection` header, `extra_stripped`, and values that aren't visible
/// ASCII.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::forward::{forwardable_headers, STRIPPED_REQUEST_HEADERS};
/// use reqwest::header::{HeaderMap, HeaderValue};
///
/// let mut headers = HeaderMap::new();
/// headers.insert("authorization", HeaderValue::from_static("Bearer sk-1"));
/// headers.insert("connection", HeaderValue::from_static("x-trace"));
/// headers.insert("x-trace", HeaderValue::from_static("1"));
/// headers.insert("accept-encoding", HeaderValue::from_static("gzip"));
///
/// let out = forwardable_headers(&headers, STRIPPED_REQUEST_HEADERS);
/// assert_eq!(out, vec![("authorization".to_string(), "Bearer sk-1".to_string())]);
/// ```
pub fn forwardable_headers(
    headers: &reqwest::header::HeaderMap,
    extra_stripped: &[&str],
) -> Vec<(String, String)> {
    let connection_listed: Vec<String> = headers
        .get_all("connection")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(|name| name.trim().to_ascii_lowercase())
        .filter(|name| !name.is_empty())
        .collect();

    headers
        .iter()
        .filter_map(|(name, value)| {
            // `HeaderName`s are always lowercase.
            let name = name.as_str();
            let stripped = HOP_BY_HOP_HEADERS.contains(&name)
                || extra_stripped.contains(&name)
                || connection_listed.iter().any(|listed| listed == name);
            if stripped {
                return None;
            }
            Some((name.to_string(), value.to_str().ok()?.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_forward_request_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-123",
                "choices": []
            })))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let req = serde_json::json!({"model": "gpt-4o", "messages": []});

        let resp = forward_request(
            &client,
            &mock_server.uri(),
            "/v1/chat/completions",
            &req,
            vec![("authorization".to_string(), "Bearer test-key".to_string())],
            Duration::from_secs(5),
        )
        .await
        .unwrap();

        assert_eq!(resp.status(), 200);
        let body = read_body(resp).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], "chatcmpl-123");
    }

    #[tokio::test]
    async fn test_forward_request_connection_refused() {
        let client = Client::new();
        let req = serde_json::json!({"model": "gpt-4o", "messages": []});

        // Port 1 is reserved and should refuse connections immediately.
        let result = forward_request(
            &client,
            "http://127.0.0.1:1",
            "/v1/chat/completions",
            &req,
            vec![],
            Duration::from_secs(1),
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(GuardrailError::Upstream(_))));
    }

    #[test]
    fn test_stripped_headers_lists() {
        assert!(STRIPPED_REQUEST_HEADERS.contains(&"host"));
        assert!(STRIPPED_REQUEST_HEADERS.contains(&"content-length"));
        assert!(STRIPPED_REQUEST_HEADERS.contains(&"accept-encoding"));
        assert!(STRIPPED_RESPONSE_HEADERS.contains(&"content-length"));
        assert!(HOP_BY_HOP_HEADERS.contains(&"transfer-encoding"));
    }

    #[test]
    fn test_forwardable_headers_strips_hop_by_hop_and_connection_listed() {
        use reqwest::header::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();
        for (name, value) in [
            ("authorization", "Bearer sk-1"),
            ("content-type", "application/json"),
            ("connection", "keep-alive, X-Hop"),
            ("keep-alive", "timeout=5"),
            ("te", "trailers"),
            ("trailer", "x-checksum"),
            ("upgrade", "h2c"),
            ("proxy-authorization", "Basic abc"),
            ("proxy-connection", "keep-alive"),
            ("transfer-encoding", "chunked"),
            ("x-hop", "1"),
            ("accept-encoding", "gzip, br"),
            ("host", "proxy.local"),
            ("x-guardrail-key", "secret"),
        ] {
            headers.insert(name, HeaderValue::from_static(value));
        }

        let mut out = forwardable_headers(&headers, STRIPPED_REQUEST_HEADERS);
        out.sort();
        assert_eq!(
            out,
            vec![
                ("authorization".to_string(), "Bearer sk-1".to_string()),
                ("content-type".to_string(), "application/json".to_string()),
            ]
        );
    }

    #[test]
    fn test_forwardable_headers_response_side_keeps_content_encoding() {
        use reqwest::header::{HeaderMap, HeaderValue};

        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", HeaderValue::from_static("gzip"));
        headers.insert("content-length", HeaderValue::from_static("42"));
        headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));

        let out = forwardable_headers(&headers, STRIPPED_RESPONSE_HEADERS);
        assert_eq!(
            out,
            vec![("content-encoding".to_string(), "gzip".to_string())]
        );
    }
}
