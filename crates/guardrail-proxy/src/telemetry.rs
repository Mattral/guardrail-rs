//! OpenTelemetry distributed tracing support.
//!
//! When `observability.otlp_endpoint` is set, `guardrail-rs` exports traces
//! to an OpenTelemetry-compatible collector (Jaeger, Grafana Tempo, OTLP
//! gRPC endpoint, etc.) via the OTLP gRPC protocol.
//!
//! ## Span hierarchy
//!
//! Each scanned request produces one root span, one child span per pipeline
//! stage, and one for the upstream call:
//!
//! ```text
//! guardrail.request              {request_id, model, provider, decision, http.status_code, entities_found}
//! ├── guardrail.stage.<stage>    {stage, decision, entities_found}   (one per stage, in order)
//! └── guardrail.upstream.forward {upstream_url, http.status_code}
//! ```
//!
//! Stage spans are opened by [`guardrail_core::Pipeline`] itself, around
//! each stage's `evaluate`. All three use the `tracing` target
//! [`TRACE_TARGET`]; [`otel_layer`] exports only spans with that target.
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
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
pub use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::{filter::filter_fn, registry::LookupSpan, Layer};

/// Name used for all guardrail-rs spans in the trace backend, and the
/// `service.name` of the exported resource.
pub const TRACER_NAME: &str = "guardrail-rs";

/// `tracing` target of every span exported to OpenTelemetry.
pub const TRACE_TARGET: &str = "guardrail::trace";

/// A `tracing-subscriber` layer that exports guardrail-rs's spans (target
/// [`TRACE_TARGET`]) through `provider`. Add it to the subscriber next to
/// the log layers.
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
///     tracing_subscriber::registry()
///         .with(telemetry::otel_layer(&provider))
///         .init();
/// }
/// ```
pub fn otel_layer<S>(provider: &SdkTracerProvider) -> impl Layer<S> + Send + Sync
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
{
    tracing_opentelemetry::layer()
        .with_tracer(provider.tracer(TRACER_NAME))
        .with_filter(filter_fn(|metadata| {
            metadata.is_span() && metadata.target() == TRACE_TARGET
        }))
}

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

/// Build the tracer provider that exports traces to an OTLP gRPC endpoint,
/// with `service.name = "guardrail-rs"`. Pass it to [`otel_layer`].
///
/// Returns `Ok(None)` if `config.otlp_endpoint` is empty or absent — OTLP
/// export is disabled and callers should skip adding the layer.
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
///     tracing_subscriber::registry()
///         .with(telemetry::otel_layer(&provider))
///         .init();
///     // ... run ...
///     telemetry::shutdown_tracer_provider(provider);
/// }
/// ```
pub fn build_otel_layer(
    config: &ObservabilityConfig,
) -> Result<Option<SdkTracerProvider>, OtelError> {
    let endpoint = config.otlp_endpoint.trim();
    if endpoint.is_empty() {
        return Ok(None);
    }

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| OtelError::ExporterBuild(e.to_string()))?;

    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(TRACER_NAME)
        .build();
    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
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
/// # let provider: telemetry::SdkTracerProvider = todo!();
/// telemetry::shutdown_tracer_provider(provider);
/// ```
pub fn shutdown_tracer_provider(provider: SdkTracerProvider) {
    if let Err(e) = provider.shutdown() {
        tracing::warn!(error = %e, "failed to shut down OTel tracer provider");
    }
}

/// The span for one pipeline stage evaluation. The pipeline opens one
/// around every stage itself; this is the same span, for custom pipelines.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::telemetry;
///
/// let stage_span = telemetry::stage_span("regex_injection");
/// let _guard = stage_span.entered();
/// ```
pub fn stage_span(stage_name: &str) -> tracing::Span {
    guardrail_core::pipeline::stage_span(stage_name)
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
        entities_found = tracing::field::Empty,
    )
}

/// The request span as the proxy opens it, before the body is parsed:
/// `request_id` and `model` are recorded once they are known.
pub(crate) fn incoming_request_span(provider: &str) -> tracing::Span {
    tracing::info_span!(
        target: "guardrail::trace",
        "guardrail.request",
        otel.name = "guardrail.request",
        request_id = tracing::field::Empty,
        model = tracing::field::Empty,
        provider = provider,
        decision = tracing::field::Empty,
        http.status_code = tracing::field::Empty,
        entities_found = tracing::field::Empty,
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
        http.status_code = tracing::field::Empty,
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
    fn test_whitespace_only_otlp_endpoint_returns_none() {
        // `.trim()` in build_otel_layer should treat a whitespace-only
        // endpoint the same as an absent one, rather than attempting to
        // build an exporter against an empty address.
        let config = ObservabilityConfig {
            otlp_endpoint: "   ".to_string(),
            ..Default::default()
        };
        let result = build_otel_layer(&config).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_otel_layer_with_endpoint_returns_provider() {
        // Building the tonic gRPC exporter is lazy — it does not connect
        // eagerly — so this succeeds even with no collector listening on
        // the endpoint. This exercises the `Some` path (the tracer provider
        // is actually constructed) and the `shutdown_tracer_provider` call.
        //
        // Requires an active Tokio runtime: `Endpoint::connect_lazy()`
        // spawns a background connection-management task via
        // `tokio::task::spawn` even though it doesn't block on an actual
        // handshake, so a plain `#[test]` panics with "there is no reactor
        // running".
        //
        // Requires a MULTI-THREADED runtime specifically: this test body
        // has no `.await` points, and `shutdown_tracer_provider` below
        // blocks synchronously waiting for that same background task to
        // acknowledge shutdown. On the default current-thread runtime, the
        // one and only executor thread is the thread doing that blocking
        // wait — it can never also poll the background task to let it make
        // progress, so the test hangs and times out rather than panicking.
        // A multi-thread runtime has a separate worker thread free to drive
        // the background task while the main thread blocks.
        let config = ObservabilityConfig {
            otlp_endpoint: "http://localhost:4317".to_string(),
            ..Default::default()
        };
        let provider = build_otel_layer(&config)
            .expect("building the OTLP exporter should not require a live collector")
            .expect("a non-empty endpoint should produce Some(provider)");
        shutdown_tracer_provider(provider);
    }

    #[test]
    fn test_otel_error_display() {
        let exporter_err = OtelError::ExporterBuild("bad endpoint".to_string());
        assert_eq!(
            exporter_err.to_string(),
            "failed to build OTLP exporter: bad endpoint"
        );

        let provider_err = OtelError::ProviderInstall("already installed".to_string());
        assert_eq!(
            provider_err.to_string(),
            "failed to install global tracer provider: already installed"
        );
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
