//! HTTP response and error-construction helpers shared across the request
//! handler.
//!
//! Kept separate from `crate::handler` so error-shape concerns (what does
//! a 400 vs 403 vs 502 body look like) are reviewable independently of
//! routing/business logic, and so the body-reading size-limit logic — which
//! has a non-obvious "did the connection error or did the body exceed the
//! limit" distinction — has room to be documented and tested on its own.

use bytes::Bytes;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Response, StatusCode};

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
#[derive(Debug)]
pub enum BodyReadError {
    /// The body exceeded `server.max_body_size_bytes`.
    TooLarge,
    /// A transport-level error occurred while reading.
    Io(hyper::Error),
}

/// Read an incoming request body, enforcing `max_size` as a byte ceiling.
///
/// The limit is enforced while reading: frames are accumulated only until
/// the total would exceed `max_size`, then reading stops and the rest of the
/// body is never pulled into memory. A body whose declared length
/// (`Content-Length`) is already over the limit is rejected before reading
/// anything. Counting actual bytes also covers clients that omit or
/// understate `Content-Length` and stream with chunked encoding.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_proxy::error::read_limited_body;
/// # async fn example(body: hyper::body::Incoming) {
/// match read_limited_body(body, 1024).await {
///     Ok(bytes) => { /* use bytes */ }
///     Err(err) => { let _ = err; /* too large or I/O error */ }
/// }
/// # }
/// ```
pub async fn read_limited_body(body: Incoming, max_size: usize) -> Result<Bytes, BodyReadError> {
    match collect_limited(body, max_size).await {
        Ok(bytes) => Ok(bytes),
        Err(None) => Err(BodyReadError::TooLarge),
        Err(Some(e)) => Err(BodyReadError::Io(e)),
    }
}

/// Collect `body` into memory, stopping as soon as it's known to be longer
/// than `max_size`. `Err(None)` means too large; `Err(Some(e))` is a
/// transport error from the body.
async fn collect_limited<B>(body: B, max_size: usize) -> Result<Bytes, Option<B::Error>>
where
    B: hyper::body::Body<Data = Bytes> + Unpin,
{
    let declared = body.size_hint().lower();
    if declared > max_size as u64 {
        return Err(None);
    }

    let mut body = body;
    let mut buf = bytes::BytesMut::with_capacity(declared as usize);
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(Some)?;
        if let Ok(data) = frame.into_data() {
            if buf.len() + data.len() > max_size {
                return Err(None);
            }
            buf.extend_from_slice(&data);
        }
    }
    Ok(buf.freeze())
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

    /// A body that never ends. The old implementation collected the whole
    /// body before checking its size, so it never returned (and a real
    /// client could make the proxy buffer as much as it liked).
    fn endless_body() -> http_body_util::StreamBody<
        impl futures::Stream<Item = Result<hyper::body::Frame<Bytes>, std::convert::Infallible>>,
    > {
        let chunk = Bytes::from(vec![b'a'; 64 * 1024]);
        http_body_util::StreamBody::new(futures::stream::repeat_with(move || {
            Ok(hyper::body::Frame::data(chunk.clone()))
        }))
    }

    #[tokio::test]
    async fn test_body_limit_stops_reading_an_endless_body() {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            collect_limited(endless_body(), 1024 * 1024),
        )
        .await
        .expect("must stop at the limit instead of reading forever");
        assert!(matches!(result, Err(None)));
    }

    #[tokio::test]
    async fn test_body_limit_rejects_declared_length_without_reading() {
        // Full<Bytes> reports its exact length as the size hint, like a
        // request with Content-Length.
        let body = Full::new(Bytes::from(vec![b'a'; 2048]));
        assert!(matches!(collect_limited(body, 1024).await, Err(None)));
    }

    #[tokio::test]
    async fn test_body_limit_allows_body_at_the_limit() {
        let data = Bytes::from(vec![b'a'; 1024]);
        let body = Full::new(data.clone());
        assert_eq!(collect_limited(body, 1024).await.ok(), Some(data));

        // A streamed body (no declared length) under the limit, in pieces.
        let pieces = futures::stream::iter((0..4).map(|_| {
            Ok::<_, std::convert::Infallible>(hyper::body::Frame::data(Bytes::from_static(b"abcd")))
        }));
        let body = http_body_util::StreamBody::new(pieces);
        assert_eq!(
            collect_limited(body, 16).await.ok(),
            Some(Bytes::from_static(b"abcdabcdabcdabcd"))
        );
    }

    #[tokio::test]
    async fn test_classify_upstream_error_timeout() {
        use guardrail_core::GuardrailError;
        use std::time::Duration;

        // A local server that accepts the connection but never answers, so
        // the only way the request can end is the client's timeout. No real
        // network is involved (this previously targeted 10.255.255.1 and
        // depended on nothing answering there).
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_socket, _) = listener.accept().await.unwrap();
            // Hold the connection open without reading or writing.
            std::future::pending::<()>().await;
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let result = client.get(format!("http://{addr}/")).send().await;
        server.abort();
        let err = result.expect_err("a server that never responds must time out");
        let guardrail_err = GuardrailError::from(err);

        assert_eq!(classify_upstream_error(&guardrail_err), "timeout");
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
