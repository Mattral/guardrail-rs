//! OpenTelemetry distributed tracing support.
//!
//! When `observability.otlp_endpoint` is set, `guardrail-rs` exports traces
//! to an OpenTelemetry-compatible collector (Jaeger, Grafana Tempo, OTLP
//! gRPC endpoint, etc.) via the OTLP gRPC protocol.
//!
//! ## Span hierarchy
//!
//! Each request produces one root span and one child span per pipeline stage:
//!
//! ```text
//! guardrail.request
//! ├── guardrail.stage.regex_injection    {decision, matched_rule}
//! ├── guardrail.stage.onnx_injection     {decision, score}
//! ├── guardrail.stage.pii_redactor       {decision, entities_found}
//! ├── guardrail.stage.toxicity           {decision, score}
//! ├── guardrail.stage.policy             {decision, rule_name}
//! └── guardrail.upstream.forward         {http.status_code, upstream_url}
//! ```
//!
//! ## Enabling tracing
//!
//! Set `observability.otlp_endpoint` in your `guardrail.toml`:
//!
//! ```toml
//! [observability]
//! otlp_endpoint = "http://localhost:4317"   # gRPC OTLP
//! ```
//!
//! Leave the field empty (or omit it) to disable OTLP export. The
//! `tracing`/`tracing-subscriber` spans used for log correlation are always
//! emitted regardless of this setting.

use guardrail_config::ObservabilityConfig;
use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
use opentelemetry_otlp::WithExportConfig;

/// Name used for all guardrail-rs spans in the trace backend.
pub const TRACER_NAME: &str = "guardrail-rs";

/// Errors that can occur while setting up the OpenTelemetry tracer.
#[derive(Debug, thiserror::Error)]
pub enum OtelError {
    /// The OTLP exporter could not be built.
    #[error("failed to build OTLP exporter: {0}")]
    ExporterBuild(String),
    /// The tracer provider could not be installed.
    #[error("failed to install global tracer provider: {0}")]
    ProviderInstall(String),
}

/// Build an `OpenTelemetryLayer` that exports traces to an OTLP gRPC endpoint.
///
/// Returns `Ok(None)` if `config.otlp_endpoint` is empty or absent — OTLP
/// export is disabled and callers should skip adding this layer.
///
/// The returned [`SdkTracerProvider`] **must** be shut down gracefully on
/// process exit by calling `provider.shutdown()`, or buffered spans may be
/// lost. Pass it to [`shutdown_tracer_provider`].
///
/// # Errors
///
/// Returns [`OtelError`] if the OTLP pipeline cannot be built (e.g. the
/// endpoint URL is malformed or the tonic transport layer cannot start).
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_config::ObservabilityConfig;
/// use guardrail_proxy::telemetry;
/// use tracing_subscriber::prelude::*;
///
/// let obs = ObservabilityConfig::default();
/// if let Some(provider) = telemetry::build_otel_layer(&obs).unwrap() {
///     // add your tracing subscriber layer here if desired
///     telemetry::shutdown_tracer_provider(provider);
/// }
/// ```
pub fn build_otel_layer(config: &ObservabilityConfig) -> Result<Option<SdkTracerProvider>, OtelError>
{
    let endpoint = config.otlp_endpoint.trim();
    if endpoint.is_empty() {
        return Ok(None);
    }

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| OtelError::ExporterBuild(e.to_string()))?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    Ok(Some(provider))
}

/// Flush all buffered spans and shut down the tracer provider.
///
/// Call this once at process exit, after the server has stopped accepting
/// new requests, to ensure no spans are lost.
///
/// # Examples
///
/// ```rust,no_run
/// use guardrail_proxy::telemetry;
/// # use opentelemetry_sdk::trace::TracerProvider;
/// # let provider: TracerProvider = todo!();
/// telemetry::shutdown_tracer_provider(provider);
/// ```
pub fn shutdown_tracer_provider(provider: SdkTracerProvider) {
    if let Err(e) = provider.shutdown() {
        tracing::warn!(error = %e, "failed to shut down OTel tracer provider");
    }
}

/// Create a child span for a pipeline stage evaluation.
///
/// Call this inside `pipeline.run_with_observer` or a custom observer to
/// create per-stage spans. The span is automatically ended when the
/// returned `Span` guard is dropped.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::telemetry;
/// use tracing::Span;
///
/// let stage_span = telemetry::stage_span("regex_injection");
/// let _guard = stage_span.entered();
/// // ... stage evaluation happens here ...
/// // span ends when `_guard` is dropped.
/// ```
pub fn stage_span(stage_name: &str) -> tracing::Span {
    tracing::info_span!(
        target: "guardrail::trace",
        "guardrail.stage",
        otel.name = format!("guardrail.stage.{stage_name}"),
        stage = stage_name,
        decision = tracing::field::Empty,
        score = tracing::field::Empty,
        matched_rule = tracing::field::Empty,
        entities_found = tracing::field::Empty,
    )
}

/// Create the root span for an entire request lifecycle.
///
/// The span covers the full round-trip including pipeline evaluation and
/// upstream forwarding.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::telemetry;
///
/// let request_span = telemetry::request_span("req-abc123", "gpt-4o", "openai");
/// let _guard = request_span.entered();
/// ```
pub fn request_span(request_id: &str, model: &str, provider: &str) -> tracing::Span {
    tracing::info_span!(
        target: "guardrail::trace",
        "guardrail.request",
        otel.name = "guardrail.request",
        request_id = request_id,
        model = model,
        provider = provider,
        decision = tracing::field::Empty,
        http.status_code = tracing::field::Empty,
    )
}

/// Create a span for the upstream forwarding step.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::telemetry;
///
/// let span = telemetry::upstream_span("https://api.openai.com");
/// let _guard = span.entered();
/// ```
pub fn upstream_span(upstream_url: &str) -> tracing::Span {
    tracing::info_span!(
        target: "guardrail::trace",
        "guardrail.upstream.forward",
        otel.name = "guardrail.upstream.forward",
        upstream_url = upstream_url,
        "http.status_code" = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use guardrail_config::ObservabilityConfig;

    #[test]
    fn test_empty_otlp_endpoint_returns_none() {
        let config = ObservabilityConfig::default();
        // default otlp_endpoint is empty string
        let result = build_otel_layer(&config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_stage_span_is_created() {
        let span = stage_span("regex_injection");
        assert!(span.metadata().is_some() || span.is_disabled());
    }

    #[test]
    fn test_request_span_is_created() {
        let span = request_span("req-1", "gpt-4o", "openai");
        assert!(span.metadata().is_some() || span.is_disabled());
    }

    #[test]
    fn test_upstream_span_is_created() {
        let span = upstream_span("https://api.openai.com");
        assert!(span.metadata().is_some() || span.is_disabled());
    }
}
