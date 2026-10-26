//! Per-connection request routing and the core proxy flow.
//!
//! `handle_request` is the top-level entry point hyper calls for every
//! incoming request; it routes to `/healthz`, `/metrics`, or
//! [`proxy_request`], which is where requests are parsed, run through the
//! pipeline, and either blocked or forwarded upstream.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use futures::StreamExt;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Method, Request, Response, StatusCode};

use guardrail_core::{
    decision::Decision,
    pipeline::{StageEvent, StageOutcome},
    request::Provider,
};

use crate::auth::is_authorized;
use crate::error::{
    classify_upstream_error, error_body_response, error_response, internal_error_response,
    read_limited_body, BodyReadError,
};
use crate::metrics::Metrics;
use crate::state::AppState;
use crate::{audit::AuditRecord, forward, translate};

/// Response body type for everything the proxy serves: either a complete
/// buffer or a stream forwarded (and possibly rewritten) from the upstream.
pub(crate) type ProxyBody = UnsyncBoxBody<Bytes, std::io::Error>;

/// Wrap a complete response in [`ProxyBody`].
fn reply(resp: Response<Full<Bytes>>) -> Response<ProxyBody> {
    resp.map(|body| body.map_err(|never| match never {}).boxed_unsync())
}

/// Where a request goes.
#[derive(Debug, Clone, PartialEq)]
enum Route {
    Health,
    Metrics,
    /// Parsed, run through the pipeline, then forwarded.
    Scanned(ScannedRoute),
    /// Listed in `server.passthrough_paths`: forwarded without scanning.
    Passthrough,
    /// A scanned path with a method other than `POST`.
    MethodNotAllowed,
    NotFound,
}

#[derive(Debug, Clone, PartialEq)]
struct ScannedRoute {
    api: translate::Api,
    provider: Provider,
    /// Model name to use when the body has none (the Azure deployment).
    default_model: Option<String>,
}

/// The scanned route for `path`, whatever the method.
fn scanned_route(path: &str) -> Option<ScannedRoute> {
    let (api, provider, default_model) = match path {
        "/v1/chat/completions" => (translate::Api::ChatCompletions, Provider::OpenAI, None),
        "/v1/messages" => (translate::Api::Messages, Provider::Anthropic, None),
        "/v1/responses" => (translate::Api::Responses, Provider::OpenAI, None),
        _ => {
            // Azure OpenAI: /openai/deployments/{deployment}/chat/completions
            let deployment = path
                .strip_prefix("/openai/deployments/")?
                .strip_suffix("/chat/completions")?;
            if deployment.is_empty() || deployment.contains('/') {
                return None;
            }
            (
                translate::Api::ChatCompletions,
                Provider::Azure,
                Some(deployment.to_string()),
            )
        }
    };
    Some(ScannedRoute {
        api,
        provider,
        default_model,
    })
}

/// Route a request. Anything that is neither a scanned route nor listed in
/// `passthrough` is `NotFound` (deny by default).
fn route(method: &Method, path: &str, passthrough: &[String]) -> Route {
    match (method, path) {
        (&Method::GET, "/healthz") => return Route::Health,
        (&Method::GET, "/metrics") => return Route::Metrics,
        _ => {}
    }
    let scanned = scanned_route(path);
    if let (&Method::POST, Some(route)) = (method, &scanned) {
        return Route::Scanned(route.clone());
    }
    let listed = passthrough
        .iter()
        .filter_map(|entry| guardrail_config::schema::PassthroughRoute::parse(entry))
        .any(|entry| entry.matches(method.as_str(), path));
    if listed {
        Route::Passthrough
    } else if scanned.is_some() {
        Route::MethodNotAllowed
    } else {
        Route::NotFound
    }
}

/// Top-level request handler: health and metrics endpoints, then the
/// route table (see [`route`]).
pub(crate) async fn handle_request(
    state: Arc<AppState>,
    req: Request<Incoming>,
) -> Result<Response<ProxyBody>, Infallible> {
    let config = state.config.config();
    let route = route(
        req.method(),
        req.uri().path(),
        &config.server.passthrough_paths,
    );

    let response = match route {
        Route::Health => reply(healthz()),
        Route::Metrics => reply(metrics_endpoint(&state)),
        Route::Scanned(route) => {
            let span = crate::telemetry::incoming_request_span(provider_label(&route.provider));
            let response =
                tracing::Instrument::instrument(proxy_request(state, req, route), span.clone())
                    .await;
            span.record("http.status_code", response.status().as_u16());
            response
        }
        Route::Passthrough => passthrough_request(state, req).await,
        Route::MethodNotAllowed => reply(error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "this route only accepts POST",
            "method_not_allowed",
        )),
        Route::NotFound => reply(error_response(
            StatusCode::NOT_FOUND,
            "no such route on this proxy (see server.passthrough_paths)",
            "not_found",
        )),
    };

    Ok(response)
}

fn healthz() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
        .expect("static response is valid")
}

fn metrics_endpoint(state: &Arc<AppState>) -> Response<Full<Bytes>> {
    match state.metrics.render() {
        Ok(body) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain; version=0.0.4")
            .body(Full::new(Bytes::from(body)))
            .expect("static response is valid"),
        Err(e) => {
            tracing::error!(error = %e, "failed to render metrics");
            internal_error_response()
        }
    }
}

/// What every proxied request (scanned or passthrough) has once it is
/// authorized and its body is read.
struct Admitted {
    /// Path plus query string, forwarded upstream as is.
    target: String,
    method: Method,
    headers: Vec<(String, String)>,
    body: Bytes,
}

/// Authenticate, collect forwardable headers and read the body with the
/// size limit. `Err` is the response to send instead.
async fn admit(
    config: &guardrail_config::Config,
    req: Request<Incoming>,
) -> Result<Admitted, Box<Response<ProxyBody>>> {
    let path = req.uri().path().to_string();
    if !is_authorized(&config.auth, &path, req.headers()) {
        tracing::warn!(path = %path, "rejected request: missing or invalid X-Guardrail-Key");
        return Err(Box::new(reply(error_response(
            StatusCode::UNAUTHORIZED,
            "missing or invalid X-Guardrail-Key header",
            "unauthorized",
        ))));
    }

    let target = req
        .uri()
        .path_and_query()
        .map_or(path, |pq| pq.as_str().to_string());
    let method = req.method().clone();
    let headers = forward::forwardable_headers(req.headers(), forward::STRIPPED_REQUEST_HEADERS);

    let body = match read_limited_body(req.into_body(), config.server.max_body_size_bytes).await {
        Ok(b) => b,
        Err(BodyReadError::TooLarge) => {
            return Err(Box::new(reply(error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "request body exceeds maximum allowed size",
                "payload_too_large",
            ))));
        }
        Err(BodyReadError::Io(e)) => {
            tracing::warn!(error = %e, "failed to read request body");
            return Err(Box::new(reply(error_response(
                StatusCode::BAD_REQUEST,
                "failed to read request body",
                "bad_request",
            ))));
        }
    };
    Ok(Admitted {
        target,
        method,
        headers,
        body,
    })
}

/// A route from `server.passthrough_paths`: forwarded with its method and
/// body as is, without running the pipeline. The response still goes
/// through response redaction.
async fn passthrough_request(state: Arc<AppState>, req: Request<Incoming>) -> Response<ProxyBody> {
    let start = Instant::now();
    let config = state.config.config();
    let incoming = match admit(&config, req).await {
        Ok(incoming) => incoming,
        Err(response) => return *response,
    };
    let request_id = uuid::Uuid::new_v4().to_string();

    let result = forward::forward_passthrough(
        &state.http_client,
        incoming.method,
        &config.upstream.url,
        &incoming.target,
        incoming.body,
        incoming.headers,
        std::time::Duration::from_secs(config.upstream.timeout_secs),
    )
    .await;
    let response = relay_upstream(&state, result, &Provider::OpenAI, &request_id).await;

    state
        .metrics
        .requests_total
        .with_label_values(&["passthrough", "other"])
        .inc();
    state
        .metrics
        .request_duration_seconds
        .with_label_values(&["passthrough"])
        .observe(start.elapsed().as_secs_f64());
    response
}

/// The core proxy flow: authenticate → parse → run pipeline → forward or block.
async fn proxy_request(
    state: Arc<AppState>,
    req: Request<Incoming>,
    route: ScannedRoute,
) -> Response<ProxyBody> {
    let start = Instant::now();
    let config = state.config.config();
    let pipeline = state.config.pipeline();
    let provider = route.provider.clone();

    let Admitted {
        target,
        headers: forward_headers,
        body: body_bytes,
        ..
    } = match admit(&config, req).await {
        Ok(incoming) => incoming,
        Err(response) => return *response,
    };

    // Parse into normalized request.
    let parsed = translate::parse_api_body(
        &body_bytes,
        route.api,
        provider.clone(),
        route.default_model.as_deref(),
    );
    let (guardrail_req, scanned_body) = match parsed {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(error = %e, "failed to parse request body");
            return reply(error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid request body: {e}"),
                "bad_request",
            ));
        }
    };

    let request_id = guardrail_req.id.to_string();
    let request_span = tracing::Span::current();
    request_span.record("request_id", request_id.as_str());
    request_span.record("model", guardrail_req.model.as_str());

    // `streaming_responses = "reject"`: refuse streamed requests outright
    // while response redaction is on, before spending anything on them.
    if guardrail_req.stream
        && state.config.response_redactor().is_some()
        && config.stages.pii_redactor.streaming_responses
            == guardrail_config::schema::StreamingResponses::Reject
    {
        return reply(error_response(
            StatusCode::BAD_REQUEST,
            "streaming responses are disabled while response PII redaction is on \
             (stages.pii_redactor.streaming_responses = \"reject\"); send stream: false",
            "streaming_not_allowed",
        ));
    }

    // Streamed Responses API events aren't redacted yet: refuse rather than
    // stream PII through.
    if guardrail_req.stream
        && route.api == translate::Api::Responses
        && state.config.response_redactor().is_some()
    {
        return reply(error_response(
            StatusCode::BAD_REQUEST,
            "streamed /v1/responses responses can't be redacted yet; \
             send stream: false while response PII redaction is on",
            "streaming_not_allowed",
        ));
    }

    // Run the pipeline, recording per-stage latency and stage errors.
    let pipeline_start = Instant::now();
    let metrics_for_observer = state.metrics.clone();
    let mut failed_open_stages: Vec<String> = Vec::new();
    let (decision, final_req) = match pipeline
        .run_with_events(guardrail_req, |event| {
            observe_stage_event(&metrics_for_observer, event, &mut failed_open_stages);
        })
        .await
    {
        Ok(result) => result,
        Err(e) => {
            // Stage errors never get here: the pipeline resolves them with
            // each stage's fail mode (`on_error`) and returns a decision.
            // `Pipeline::run_with_events` doesn't return `Err` today; if it
            // ever does, the request was consumed and can't be forwarded, so
            // the only safe answer is an error response.
            tracing::error!(error = %e, "pipeline evaluation failed unexpectedly");
            return reply(internal_error_response());
        }
    };
    request_span.record("decision", decision.name());
    if let Decision::Redact { entities, .. } = &decision {
        request_span.record("entities_found", entities.len());
    }
    let pipeline_elapsed_ms = pipeline_start.elapsed().as_secs_f64() * 1000.0;
    state
        .metrics
        .pipeline_duration_seconds
        .observe(pipeline_elapsed_ms / 1000.0);

    let (response, decision_label) = match decision {
        Decision::Block { reason, code } => {
            let body = translate::block_response_body(&reason, &code, &request_id);
            let block_response = error_body_response(StatusCode::FORBIDDEN, &body);

            let total_elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let audit_decision = Decision::Block {
                reason: reason.clone(),
                code: code.clone(),
            };
            AuditRecord::from_decision(
                &final_req,
                &audit_decision,
                pipeline_elapsed_ms,
                total_elapsed_ms,
            )
            .with_failed_open_stages(failed_open_stages)
            .emit();

            state
                .metrics
                .blocked_total
                .with_label_values(&[code.as_str()])
                .inc();
            state
                .metrics
                .requests_total
                .with_label_values(&["block", provider_label(&provider)])
                .inc();

            (reply(block_response), "block")
        }
        Decision::Allow | Decision::Redact { .. } => {
            let is_redact = matches!(&decision, Decision::Redact { .. });

            // Write the (possibly redacted) text back into the client's
            // body; everything else in it is forwarded untouched.
            let upstream_response = match scanned_body.apply(&final_req) {
                Ok(body) => {
                    let span = crate::telemetry::upstream_span(&config.upstream.url);
                    let result = tracing::Instrument::instrument(
                        forward::forward_request(
                            &state.http_client,
                            &config.upstream.url,
                            &target,
                            &body,
                            forward_headers,
                            std::time::Duration::from_secs(config.upstream.timeout_secs),
                        ),
                        span.clone(),
                    )
                    .await;
                    if let Ok(resp) = &result {
                        span.record("http.status_code", resp.status().as_u16());
                    }
                    drop(span);
                    relay_upstream(&state, result, &final_req.provider, &request_id).await
                }
                Err(e) => {
                    tracing::error!(error = %e, "could not write pipeline output back to the request");
                    reply(internal_error_response())
                }
            };

            let total_elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            // `from_decision` reads the PII entity list directly from
            // `decision` (Decision::Redact's own `entities` field) — no
            // separate parameter to keep in sync.
            AuditRecord::from_decision(
                &final_req,
                &decision,
                pipeline_elapsed_ms,
                total_elapsed_ms,
            )
            .with_failed_open_stages(failed_open_stages)
            .emit();

            if is_redact {
                state.metrics.redacted_total.inc();
                state
                    .metrics
                    .requests_total
                    .with_label_values(&["redact", provider_label(&provider)])
                    .inc();
            } else {
                state
                    .metrics
                    .requests_total
                    .with_label_values(&["allow", provider_label(&provider)])
                    .inc();
            }

            let label = if is_redact { "redact" } else { "allow" };
            (upstream_response, label)
        }
    };

    // Record end-to-end request latency (pipeline + upstream forwarding),
    // labeled with the decision as documented on the metric.
    state
        .metrics
        .request_duration_seconds
        .with_label_values(&[decision_label])
        .observe(start.elapsed().as_secs_f64());

    response
}

/// Turn the upstream's answer into the client's response: copy status
/// and headers, and redact the body when response redaction is on.
async fn relay_upstream(
    state: &Arc<AppState>,
    result: Result<reqwest::Response, guardrail_core::GuardrailError>,
    provider: &Provider,
    request_id: &str,
) -> Response<ProxyBody> {
    match result {
        Ok(upstream_resp) => {
            let status = StatusCode::from_u16(upstream_resp.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            let content_type = upstream_resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string);

            let redactor = state.config.response_redactor();

            // The HTTP client decodes every encoding it asks for and drops
            // `content-encoding` when it does. One that's still there is an
            // encoding the proxy can't read, so with redaction on the body
            // can't be checked: refuse it rather than pass PII through.
            let content_encoding = upstream_resp
                .headers()
                .get("content-encoding")
                .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned());
            if redactor.is_some() {
                if let Some(encoding) = content_encoding
                    .as_deref()
                    .filter(|e| !e.trim().eq_ignore_ascii_case("identity"))
                {
                    record_redaction_skipped(state, "unknown_encoding");
                    tracing::warn!(
                        request_id = %request_id,
                        content_encoding = %encoding,
                        "refusing upstream response: its encoding can't be decoded for redaction"
                    );
                    return reply(error_response(
                        StatusCode::BAD_GATEWAY,
                        "upstream response uses a content-encoding the proxy cannot decode, \
                         so it cannot be checked for PII",
                        "response_unredactable",
                    ));
                }
            }

            let mut builder = Response::builder().status(status);

            for (name, value) in forward::forwardable_headers(
                upstream_resp.headers(),
                forward::STRIPPED_RESPONSE_HEADERS,
            ) {
                if let (Ok(header_name), Ok(header_value)) = (
                    HeaderName::from_bytes(name.as_bytes()),
                    HeaderValue::from_str(&value),
                ) {
                    builder = builder.header(header_name, header_value);
                }
            }

            let is_sse = content_type
                .as_deref()
                .is_some_and(|ct| ct.starts_with("text/event-stream"));
            let is_json = content_type
                .as_deref()
                .is_some_and(|ct| ct.starts_with("application/json"));

            if redactor.is_some() && !is_sse && !is_json && status.is_success() {
                record_redaction_skipped(state, "unknown_content_type");
                tracing::warn!(
                    request_id = %request_id,
                    content_type = content_type.as_deref().unwrap_or("<none>"),
                    "forwarding upstream response unredacted: unknown content-type"
                );
            }

            let body = if redactor.is_some() && is_sse {
                // Streamed response with redaction on: rewrite text deltas
                // as they flow.
                redacting_sse_body(
                    state.clone(),
                    redactor,
                    provider.clone(),
                    request_id.to_string(),
                    upstream_resp,
                )
            } else if redactor.is_some() && is_json {
                // Complete JSON document: buffer, redact, send.
                match forward::read_body(upstream_resp).await {
                    Ok(body) => {
                        let body = maybe_redact_response(
                            state,
                            request_id,
                            &body,
                            content_type.as_deref(),
                            false,
                        );
                        Full::new(body)
                            .map_err(|never| match never {})
                            .boxed_unsync()
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to read upstream response body");
                        return reply(internal_error_response());
                    }
                }
            } else {
                // Nothing to rewrite: forward bytes as they arrive, so
                // streamed responses reach the client token by token.
                passthrough_body(upstream_resp)
            };

            builder
                .body(body)
                .unwrap_or_else(|_| reply(internal_error_response()))
        }
        Err(e) => {
            tracing::error!(error = %e, "upstream request failed");
            state
                .metrics
                .upstream_errors_total
                .with_label_values(&[classify_upstream_error(&e)])
                .inc();
            reply(error_response(
                StatusCode::BAD_GATEWAY,
                "upstream request failed",
                "upstream_error",
            ))
        }
    }
}

/// Forward the upstream body chunk by chunk, unmodified.
fn passthrough_body(upstream: reqwest::Response) -> ProxyBody {
    let frames = upstream
        .bytes_stream()
        .map(|chunk| chunk.map(Frame::data).map_err(std::io::Error::other));
    StreamBody::new(frames).boxed_unsync()
}

/// Forward a `text/event-stream` body while redacting PII in its text
/// deltas (see [`crate::sse`]). When the stream ends, any redactions are
/// counted in `guardrail_response_redacted_total` and logged as a
/// `response_redact` audit event, as for non-streamed responses.
fn redacting_sse_body(
    state: Arc<AppState>,
    redactor: Arc<Option<guardrail_classifiers::PiiRedactor>>,
    provider: Provider,
    request_id: String,
    upstream: reqwest::Response,
) -> ProxyBody {
    struct Ctx {
        upstream: futures::stream::BoxStream<'static, reqwest::Result<Bytes>>,
        sse: crate::sse::SseRedactor,
        redactor: Arc<Option<guardrail_classifiers::PiiRedactor>>,
        state: Arc<AppState>,
        request_id: String,
        done: bool,
    }

    let ctx = Ctx {
        upstream: upstream.bytes_stream().boxed(),
        sse: crate::sse::SseRedactor::new(&provider),
        redactor,
        state,
        request_id,
        done: false,
    };

    let frames = futures::stream::unfold(ctx, |mut ctx| async move {
        loop {
            if ctx.done {
                return None;
            }
            let redactor = ctx.redactor.as_ref().as_ref()?;
            match ctx.upstream.next().await {
                Some(Ok(bytes)) => {
                    let out = ctx.sse.push(redactor, &bytes);
                    // Skip empty writes; wait for more upstream bytes.
                    if !out.is_empty() {
                        return Some((Ok(Frame::data(Bytes::from(out))), ctx));
                    }
                }
                Some(Err(e)) => {
                    ctx.done = true;
                    tracing::error!(error = %e, "upstream stream failed");
                    return Some((Err(std::io::Error::other(e)), ctx));
                }
                None => {
                    ctx.done = true;
                    let out = ctx.sse.finish(redactor);
                    if ctx.sse.redactions() > 0 {
                        ctx.state.metrics.response_redacted_total.inc();
                        tracing::info!(
                            target: "guardrail::audit",
                            request_id = %ctx.request_id,
                            decision = "response_redact",
                            total_redactions = ctx.sse.redactions(),
                            entity_types = ?ctx.sse.entity_types(),
                            streamed = true,
                            "response PII redacted"
                        );
                    }
                    if !out.is_empty() {
                        return Some((Ok(Frame::data(Bytes::from(out))), ctx));
                    }
                    return None;
                }
            }
        }
    });

    StreamBody::new(frames).boxed_unsync()
}

/// If response-side PII redaction is enabled and the response is a
/// non-streaming JSON document, scan it for PII and return the redacted
/// bytes. Otherwise return `body` unchanged.
///
/// Any redactions are recorded in [`crate::metrics::Metrics::response_redacted_total`]
/// and emitted as a `guardrail::audit` event.
fn maybe_redact_response(
    state: &Arc<AppState>,
    request_id: &str,
    body: &Bytes,
    content_type: Option<&str>,
    is_streaming: bool,
) -> Bytes {
    let redactor_snapshot = state.config.response_redactor();
    let redactor = match &*redactor_snapshot {
        Some(r) => r,
        None => return body.clone(),
    };

    if !crate::response::is_redactable_response(content_type, is_streaming) {
        return body.clone();
    }

    match crate::response::try_redact_response_body(body, redactor) {
        Ok(Some((new_body, summary))) => {
            state.metrics.response_redacted_total.inc();
            tracing::info!(
                target: "guardrail::audit",
                request_id = %request_id,
                decision = "response_redact",
                total_redactions = summary.total_redactions,
                entity_types = ?summary.entity_types,
                "response PII redacted"
            );
            Bytes::from(new_body)
        }
        Ok(None) => body.clone(),
        Err(e) => {
            record_redaction_skipped(state, "unparseable_body");
            tracing::warn!(
                request_id = %request_id,
                error = %e,
                "forwarding upstream response unredacted: body is not valid JSON"
            );
            body.clone()
        }
    }
}

/// Count a response that response redaction was on for but couldn't
/// inspect. Callers log the details.
fn record_redaction_skipped(state: &AppState, reason: &'static str) {
    state
        .metrics
        .response_redaction_skipped_total
        .with_label_values(&[reason])
        .inc();
}

/// Record one stage evaluation: its latency, and, if it errored, a
/// `guardrail_pipeline_errors_total` increment labeled with how `on_error`
/// resolved it. Stages that failed open are appended to `failed_open` for
/// the request's audit record. Input too long for a stage is counted in
/// `guardrail_stage_oversize_total`.
fn observe_stage_event(metrics: &Metrics, event: &StageEvent, failed_open: &mut Vec<String>) {
    metrics
        .stage_duration_seconds
        .with_label_values(&[event.stage])
        .observe(event.elapsed.as_secs_f64());

    let oversize = match event.outcome {
        StageOutcome::OversizeAllowed => Some("allowed"),
        StageOutcome::OversizeBlocked => Some("blocked"),
        _ => None,
    };
    if let Some(outcome) = oversize {
        metrics
            .stage_oversize_total
            .with_label_values(&[event.stage, outcome])
            .inc();
        return;
    }

    let outcome = match event.outcome {
        StageOutcome::ErrorAllowed => {
            failed_open.push(event.stage.to_string());
            "allowed"
        }
        StageOutcome::ErrorBlocked => "blocked",
        _ => return,
    };
    metrics
        .pipeline_errors_total
        .with_label_values(&[event.stage, outcome])
        .inc();
}

fn provider_label(provider: &Provider) -> &'static str {
    match provider {
        Provider::OpenAI => "openai",
        Provider::Anthropic => "anthropic",
        Provider::Azure => "azure",
        Provider::Other(_) => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::Metrics;
    use guardrail_config::ConfigHandle;
    use std::io::Write;

    fn write_temp_config(contents: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_route_table() {
        use translate::Api;
        let allow = vec!["GET /v1/models".to_string(), "GET /v1/models/*".to_string()];
        let scanned = |api, provider, model: Option<&str>| {
            Route::Scanned(ScannedRoute {
                api,
                provider,
                default_model: model.map(String::from),
            })
        };

        assert_eq!(route(&Method::GET, "/healthz", &allow), Route::Health);
        assert_eq!(route(&Method::GET, "/metrics", &allow), Route::Metrics);
        assert_eq!(
            route(&Method::POST, "/v1/chat/completions", &allow),
            scanned(Api::ChatCompletions, Provider::OpenAI, None)
        );
        assert_eq!(
            route(&Method::POST, "/v1/messages", &allow),
            scanned(Api::Messages, Provider::Anthropic, None)
        );
        assert_eq!(
            route(&Method::POST, "/v1/responses", &allow),
            scanned(Api::Responses, Provider::OpenAI, None)
        );
        assert_eq!(
            route(
                &Method::POST,
                "/openai/deployments/prod/chat/completions",
                &allow
            ),
            scanned(Api::ChatCompletions, Provider::Azure, Some("prod"))
        );
        assert_eq!(
            route(
                &Method::POST,
                "/openai/deployments//chat/completions",
                &allow
            ),
            Route::NotFound
        );
        assert_eq!(
            route(&Method::GET, "/v1/chat/completions", &allow),
            Route::MethodNotAllowed
        );
        assert_eq!(
            route(&Method::GET, "/v1/models", &allow),
            Route::Passthrough
        );
        assert_eq!(
            route(&Method::GET, "/v1/models/gpt-4o", &allow),
            Route::Passthrough
        );
        assert_eq!(route(&Method::POST, "/v1/models", &allow), Route::NotFound);
        assert_eq!(
            route(&Method::POST, "/v1/embeddings", &allow),
            Route::NotFound
        );
        assert_eq!(route(&Method::POST, "/anything/else", &[]), Route::NotFound);
    }

    #[test]
    fn test_provider_label_mapping() {
        assert_eq!(provider_label(&Provider::OpenAI), "openai");
        assert_eq!(provider_label(&Provider::Anthropic), "anthropic");
        assert_eq!(provider_label(&Provider::Azure), "azure");
        assert_eq!(provider_label(&Provider::Other("custom".into())), "other");
    }

    #[tokio::test]
    async fn test_fail_open_stage_error_is_counted_and_reported() {
        use guardrail_core::pipeline::{FailMode, PipelineBuilder};
        use guardrail_core::test_helpers::{clean_request, ErrorStage};

        let metrics = Metrics::new();
        let pipeline = PipelineBuilder::default()
            .stage_with_fail_mode(ErrorStage, FailMode::Open)
            .build();

        let mut failed_open = Vec::new();
        let (decision, _) = pipeline
            .run_with_events(clean_request(), |e| {
                observe_stage_event(&metrics, e, &mut failed_open)
            })
            .await
            .unwrap();

        assert_eq!(decision, Decision::Allow);
        assert_eq!(failed_open, vec!["error_stage".to_string()]);
        let allowed = metrics
            .pipeline_errors_total
            .with_label_values(&["error_stage", "allowed"]);
        assert_eq!(allowed.get(), 1);
        assert!(metrics.render().unwrap().contains(
            r#"guardrail_pipeline_errors_total{outcome="allowed",stage="error_stage"} 1"#
        ));
    }

    #[tokio::test]
    async fn test_fail_closed_stage_error_is_counted_but_not_failed_open() {
        use guardrail_core::pipeline::{FailMode, PipelineBuilder};
        use guardrail_core::test_helpers::{clean_request, ErrorStage, PassthroughStage};

        let metrics = Metrics::new();
        let pipeline = PipelineBuilder::default()
            .stage(PassthroughStage)
            .stage_with_fail_mode(ErrorStage, FailMode::Closed)
            .build();

        let mut failed_open = Vec::new();
        let (decision, _) = pipeline
            .run_with_events(clean_request(), |e| {
                observe_stage_event(&metrics, e, &mut failed_open)
            })
            .await
            .unwrap();

        assert!(matches!(decision, Decision::Block { .. }));
        assert!(failed_open.is_empty());
        let counter = &metrics.pipeline_errors_total;
        assert_eq!(
            counter.with_label_values(&["error_stage", "blocked"]).get(),
            1
        );
        assert_eq!(
            counter.with_label_values(&["error_stage", "allowed"]).get(),
            0
        );
        // Healthy stages record latency only.
        assert_eq!(
            counter.with_label_values(&["passthrough", "allowed"]).get(),
            0
        );
        assert_eq!(
            metrics
                .stage_duration_seconds
                .with_label_values(&["passthrough"])
                .get_sample_count(),
            1
        );
    }

    #[tokio::test]
    async fn test_oversize_is_counted_per_stage_and_outcome() {
        use guardrail_core::pipeline::{FailMode, OversizeAction, PipelineBuilder};
        use guardrail_core::test_helpers::{clean_request, TooLongStage};

        let metrics = Metrics::new();
        for action in [OversizeAction::Allow, OversizeAction::Block] {
            let pipeline = PipelineBuilder::default()
                .stage_with_fail_mode(TooLongStage(action), FailMode::Open)
                .build();
            let mut failed_open = Vec::new();
            pipeline
                .run_with_events(clean_request(), |e| {
                    observe_stage_event(&metrics, e, &mut failed_open)
                })
                .await
                .unwrap();
            assert!(failed_open.is_empty(), "oversize is not a stage error");
        }

        let counter = &metrics.stage_oversize_total;
        assert_eq!(counter.with_label_values(&["too_long", "allowed"]).get(), 1);
        assert_eq!(counter.with_label_values(&["too_long", "blocked"]).get(), 1);
        assert_eq!(
            metrics
                .pipeline_errors_total
                .with_label_values(&["too_long", "allowed"])
                .get(),
            0
        );
    }

    #[test]
    fn test_healthz() {
        let resp = healthz();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_maybe_redact_response_without_redactor_passthrough() {
        let toml_str = r#"
            [server]
            host = "127.0.0.1"
            port = 0

            [upstream]
            url = "https://api.openai.com"
        "#;
        let f = write_temp_config(toml_str);
        let config = Arc::new(ConfigHandle::load(f.path()).unwrap());
        let state = Arc::new(AppState {
            config,
            http_client: reqwest::Client::new(),
            metrics: Metrics::new(),
        });

        let body = Bytes::from(
            r#"{"choices":[{"message":{"role":"assistant","content":"Email me at user@example.com"}}]}"#,
        );

        let out = maybe_redact_response(&state, "req-1", &body, Some("application/json"), false);

        // Response redaction is disabled by default; body passes through unchanged.
        assert_eq!(out, body);
        assert_eq!(state.metrics.response_redacted_total.get(), 0);
    }

    #[tokio::test]
    async fn test_maybe_redact_response_with_redactor_enabled() {
        let toml_str = r#"
            [server]
            host = "127.0.0.1"
            port = 0

            [upstream]
            url = "https://api.openai.com"

            [stages.pii_redactor]
            enabled = true
            redact_responses = true
        "#;
        let f = write_temp_config(toml_str);
        let config = Arc::new(ConfigHandle::load(f.path()).unwrap());
        let state = Arc::new(AppState {
            config,
            http_client: reqwest::Client::new(),
            metrics: Metrics::new(),
        });

        let body = Bytes::from(
            r#"{"choices":[{"message":{"role":"assistant","content":"Email me at user@example.com"}}]}"#,
        );

        let out = maybe_redact_response(&state, "req-2", &body, Some("application/json"), false);

        let text = String::from_utf8(out.to_vec()).unwrap();
        assert!(text.contains("[EMAIL]"));
        assert!(!text.contains("user@example.com"));
        assert_eq!(state.metrics.response_redacted_total.get(), 1);
    }

    #[tokio::test]
    async fn test_maybe_redact_response_counts_unparseable_body() {
        let toml_str = r#"
            [server]
            host = "127.0.0.1"
            port = 0

            [upstream]
            url = "https://api.openai.com"

            [stages.pii_redactor]
            enabled = true
            redact_responses = true
        "#;
        let f = write_temp_config(toml_str);
        let config = Arc::new(ConfigHandle::load(f.path()).unwrap());
        let state = Arc::new(AppState {
            config,
            http_client: reqwest::Client::new(),
            metrics: Metrics::new(),
        });

        let body = Bytes::from_static(b"{\"choices\": [truncated");
        let out = maybe_redact_response(&state, "req-4", &body, Some("application/json"), false);

        assert_eq!(out, body);
        let skipped = &state.metrics.response_redaction_skipped_total;
        assert_eq!(skipped.with_label_values(&["unparseable_body"]).get(), 1);

        // A clean JSON body is not a skip.
        let clean = Bytes::from_static(br#"{"choices":[]}"#);
        maybe_redact_response(&state, "req-5", &clean, Some("application/json"), false);
        assert_eq!(skipped.with_label_values(&["unparseable_body"]).get(), 1);
    }

    #[tokio::test]
    async fn test_maybe_redact_response_skips_streaming() {
        let toml_str = r#"
            [server]
            host = "127.0.0.1"
            port = 0

            [upstream]
            url = "https://api.openai.com"

            [stages.pii_redactor]
            enabled = true
            redact_responses = true
        "#;
        let f = write_temp_config(toml_str);
        let config = Arc::new(ConfigHandle::load(f.path()).unwrap());
        let state = Arc::new(AppState {
            config,
            http_client: reqwest::Client::new(),
            metrics: Metrics::new(),
        });

        let body = Bytes::from(r#"{"choices":[{"message":{"content":"user@example.com"}}]}"#);

        // is_streaming = true -> passthrough even though redaction is enabled.
        let out = maybe_redact_response(&state, "req-3", &body, Some("application/json"), true);
        assert_eq!(out, body);
        assert_eq!(state.metrics.response_redacted_total.get(), 0);
    }
}
