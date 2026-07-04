//! Upstream request forwarding.
//!
//! Once the pipeline allows (or redacts) a request, [`forward_request`] sends
//! it to the configured upstream LLM API and streams the response back to the
//! client unmodified (responses are never inspected by the pipeline — only
//! requests are).

use bytes::Bytes;
use guardrail_core::{request::GuardrailRequest, GuardrailError};
use reqwest::{Client, Response};
use std::time::Duration;

/// Forward a (possibly redacted) request to the upstream LLM API.
///
/// # Arguments
///
/// * `client` — a shared [`reqwest::Client`] (connection-pooled).
/// * `upstream_url` — base URL of the upstream, e.g. `https://api.openai.com`.
/// * `path` — the request path, e.g. `/v1/chat/completions`.
/// * `req` — the (possibly redacted) normalized request.
/// * `headers` — headers from the original client request to forward
///   (e.g. `Authorization`), with `Host` and `Content-Length` stripped by
///   the caller.
/// * `timeout` — per-request timeout.
///
/// # Errors
///
/// Returns [`GuardrailError::Upstream`] if the request fails to send or the
/// connection is refused/times out. Returns [`GuardrailError::Serialization`]
/// if `req` cannot be serialized back to JSON.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_proxy::forward::forward_request;
/// use guardrail_core::test_helpers::clean_request;
/// use reqwest::Client;
/// use std::time::Duration;
///
/// # tokio_test::block_on(async {
/// let client = Client::new();
/// let req = clean_request();
/// let resp = forward_request(
///     &client,
///     "https://api.openai.com",
///     "/v1/chat/completions",
///     &req,
///     vec![("authorization".to_string(), "Bearer sk-...".to_string())],
///     Duration::from_secs(60),
/// ).await;
/// # });
/// ```
pub async fn forward_request(
    client: &Client,
    upstream_url: &str,
    path: &str,
    req: &GuardrailRequest,
    headers: Vec<(String, String)>,
    timeout: Duration,
) -> Result<Response, GuardrailError> {
    let body = crate::translate::serialize_request(req)?;
    let url = format!("{}{}", upstream_url.trim_end_matches('/'), path);

    let mut builder = client.post(&url).json(&body).timeout(timeout);

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

/// Headers that must never be forwarded to the upstream as-is.
///
/// `host` is stripped because the upstream has its own host; `content-length`
/// is stripped because the body may have changed size after redaction (and
/// `reqwest` recomputes it); `connection` is a hop-by-hop header per RFC 7230.
/// Headers stripped from the **request** before forwarding upstream.
///
/// `host` is recomputed for the upstream's own hostname; `content-length` is
/// recomputed because the body may change size after redaction;
/// `connection` is hop-by-hop per RFC 7230; `x-guardrail-key` is
/// guardrail-rs's own caller-authentication secret and must never reach the
/// upstream LLM provider.
pub const STRIPPED_REQUEST_HEADERS: &[&str] =
    &["host", "content-length", "connection", "x-guardrail-key"];

/// Headers that must never be forwarded back to the client as-is.
///
/// `transfer-encoding` and `connection` are hop-by-hop; `content-length` is
/// recomputed by the proxy's HTTP server.
pub const STRIPPED_RESPONSE_HEADERS: &[&str] =
    &["transfer-encoding", "connection", "content-length"];

#[cfg(test)]
mod tests {
    use super::*;
    use guardrail_core::test_helpers::clean_request;
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
        let req = clean_request();

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
        let req = clean_request();

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
        assert!(STRIPPED_RESPONSE_HEADERS.contains(&"transfer-encoding"));
    }
}
