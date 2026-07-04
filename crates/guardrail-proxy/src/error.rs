//! HTTP response and error-construction helpers shared across the request
//! handler.
//!
//! Kept separate from [`crate::handler`] so error-shape concerns (what does
//! a 400 vs 403 vs 502 body look like) are reviewable independently of
//! routing/business logic, and so the body-reading size-limit logic — which
//! has a non-obvious "did the connection error or did the body exceed the
//! limit" distinction — has room to be documented and tested on its own.

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Response, StatusCode};
use http_body_util::Full;

/// Build a standard error response body and wrap it as a full HTTP response.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::error::error_response;
/// use hyper::StatusCode;
///
/// let resp = error_response(StatusCode::BAD_REQUEST, "invalid JSON", "bad_request");
/// assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
/// assert_eq!(
///     resp.headers().get("content-type").unwrap(),
///     "application/json"
/// );
/// ```
pub fn error_response(status: StatusCode, message: &str, code: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": {
            "message": message,
            "type": "guardrail_error",
            "code": code,
        }
    });
    error_body_response(status, &body)
}

/// Wrap an arbitrary JSON value as a full HTTP response with the given status.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::error::error_body_response;
/// use hyper::StatusCode;
///
/// let body = serde_json::json!({"error": {"code": "prompt_injection"}});
/// let resp = error_body_response(StatusCode::FORBIDDEN, &body);
/// assert_eq!(resp.status(), StatusCode::FORBIDDEN);
/// ```
pub fn error_body_response(status: StatusCode, body: &serde_json::Value) -> Response<Full<Bytes>> {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(bytes)))
        .expect("response build never fails for valid status/headers")
}

/// A generic `500` response for failures that have no more specific status.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::error::internal_error_response;
/// use hyper::StatusCode;
///
/// let resp = internal_error_response();
/// assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
/// ```
pub fn internal_error_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(
            r#"{"error":{"message":"internal server error","type":"guardrail_internal_error"}}"#,
        )))
        .expect("static response is valid")
}

/// Why reading the request body failed.
pub(crate) enum BodyReadError {
    /// The body exceeded `server.max_body_size_bytes`.
    TooLarge,
    /// A transport-level error occurred while reading.
    Io(hyper::Error),
}

/// Read an incoming request body, enforcing `max_size` as a byte ceiling.
///
/// Unlike checking `Content-Length` alone (which a client can omit, lie
/// about, or stream past via chunked transfer-encoding), this collects the
/// actual bytes and checks the real total — closing the gap a
/// length-header-only check would leave open.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_proxy::error::read_limited_body;
/// # async fn example(body: hyper::body::Incoming) {
/// match read_limited_body(body, 1024).await {
///     Ok(bytes) => { /* use bytes */ }
///     Err(_) => { /* too large or I/O error */ }
/// }
/// # }
/// ```
pub async fn read_limited_body(
    body: Incoming,
    max_size: usize,
) -> Result<Bytes, BodyReadError> {
    let collected = body.collect().await.map_err(BodyReadError::Io)?;
    let bytes = collected.to_bytes();
    if bytes.len() > max_size {
        Err(BodyReadError::TooLarge)
    } else {
        Ok(bytes)
    }
}

/// Classify a [`guardrail_core::GuardrailError::Upstream`] into a coarse
/// error class for the `guardrail_upstream_errors_total` metric label.
///
/// With the `reqwest-errors` feature enabled (always on for
/// `guardrail-proxy`), this inspects the structured `reqwest::Error` via
/// `.is_timeout()` / `.is_connect()` rather than string-matching the
/// `Display` output, which is more reliable across `reqwest`/`hyper`
/// versions and locales.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::error::classify_upstream_error;
/// use guardrail_core::GuardrailError;
///
/// let err = GuardrailError::Internal("unrelated".into());
/// assert_eq!(classify_upstream_error(&err), "other");
/// ```
pub fn classify_upstream_error(err: &guardrail_core::GuardrailError) -> &'static str {
    if let guardrail_core::GuardrailError::Upstream(reqwest_err) = err {
        if reqwest_err.is_timeout() {
            return "timeout";
        }
        if reqwest_err.is_connect() {
            return "connect";
        }
        return "other";
    }
    "other"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response_shape() {
        let resp = error_response(StatusCode::BAD_REQUEST, "bad input", "bad_request");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_internal_error_response_is_500() {
        let resp = internal_error_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_body_response_preserves_arbitrary_json() {
        let body = serde_json::json!({"custom": "shape", "n": 42});
        let resp = error_body_response(StatusCode::FORBIDDEN, &body);
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_classify_upstream_error_timeout() {
        use guardrail_core::GuardrailError;
        use std::time::Duration;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(50))
            .build()
            .unwrap();

        let result = client.get("http://10.255.255.1/").send().await;
        let err = result.expect_err("request to non-routable address must fail");
        let guardrail_err = GuardrailError::from(err);

        let class = classify_upstream_error(&guardrail_err);
        assert!(
            class == "timeout" || class == "connect",
            "unexpected classification: {class}"
        );
    }

    #[tokio::test]
    async fn test_classify_upstream_error_connect_refused() {
        use guardrail_core::GuardrailError;

        let client = reqwest::Client::new();
        let result = client.get("http://127.0.0.1:1/").send().await;
        let err = result.expect_err("connection to port 1 must fail");
        let guardrail_err = GuardrailError::from(err);

        assert_eq!(classify_upstream_error(&guardrail_err), "connect");
    }

    #[test]
    fn test_classify_upstream_error_non_upstream_variant_is_other() {
        use guardrail_core::GuardrailError;
        let err = GuardrailError::Internal("something else".into());
        assert_eq!(classify_upstream_error(&err), "other");
    }
}
