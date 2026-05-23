//! Prometheus metrics for `guardrail-rs`.
//!
//! Exposes a `/metrics` endpoint compatible with Prometheus scraping. All
//! metrics are registered in a process-global [`prometheus::Registry`] created
//! once at startup via [`Metrics::new`].

use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts, Registry, TextEncoder,
};

/// Container for all Prometheus metrics emitted by the proxy.
///
/// Cloning is cheap: all fields are `Arc`-backed internally by the
/// `prometheus` crate's collector types.
#[derive(Clone)]
pub struct Metrics {
    /// Total number of requests received, labeled by decision (`allow`,
    /// `redact`, `block`) and provider.
    pub requests_total: IntCounterVec,
    /// Total number of requests blocked, labeled by block code.
    pub blocked_total: IntCounterVec,
    /// Total number of requests with PII redacted.
    pub redacted_total: IntCounter,
    /// End-to-end pipeline evaluation latency, in seconds.
    pub pipeline_duration_seconds: Histogram,
    /// Per-stage evaluation latency, in seconds, labeled by stage name.
    pub stage_duration_seconds: prometheus::HistogramVec,
    /// The registry these metrics are registered in.
    registry: Registry,
}

impl Metrics {
    /// Create a new metrics registry with all collectors registered.
    ///
    /// # Panics
    ///
    /// Panics if metric registration fails (only possible on duplicate
    /// registration, which cannot happen since this is called once at startup).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_proxy::metrics::Metrics;
    ///
    /// let metrics = Metrics::new();
    /// metrics.requests_total.with_label_values(&["allow", "openai"]).inc();
    /// assert_eq!(metrics.requests_total.with_label_values(&["allow", "openai"]).get(), 1);
    /// ```
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total = IntCounterVec::new(
            Opts::new(
                "guardrail_requests_total",
                "Total number of requests evaluated, labeled by decision and provider.",
            ),
            &["decision", "provider"],
        )
        .expect("valid metric definition");

        let blocked_total = IntCounterVec::new(
            Opts::new(
                "guardrail_blocked_total",
                "Total number of requests blocked, labeled by block code.",
            ),
            &["code"],
        )
        .expect("valid metric definition");

        let redacted_total = IntCounter::new(
            "guardrail_redacted_total",
            "Total number of requests that had PII redacted.",
        )
        .expect("valid metric definition");

        let pipeline_duration_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "guardrail_pipeline_duration_seconds",
                "End-to-end pipeline evaluation latency in seconds.",
            )
            .buckets(vec![
                0.00005, 0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1,
            ]),
        )
        .expect("valid metric definition");

        let stage_duration_seconds = prometheus::HistogramVec::new(
            HistogramOpts::new(
                "guardrail_stage_duration_seconds",
                "Per-stage evaluation latency in seconds, labeled by stage name.",
            )
            .buckets(vec![
                0.00001, 0.00005, 0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025,
                0.05,
            ]),
            &["stage"],
        )
        .expect("valid metric definition");

        registry
            .register(Box::new(requests_total.clone()))
            .expect("register requests_total");
        registry
            .register(Box::new(blocked_total.clone()))
            .expect("register blocked_total");
        registry
            .register(Box::new(redacted_total.clone()))
            .expect("register redacted_total");
        registry
            .register(Box::new(pipeline_duration_seconds.clone()))
            .expect("register pipeline_duration_seconds");
        registry
            .register(Box::new(stage_duration_seconds.clone()))
            .expect("register stage_duration_seconds");

        Self {
            requests_total,
            blocked_total,
            redacted_total,
            pipeline_duration_seconds,
            stage_duration_seconds,
            registry,
        }
    }

    /// Render all registered metrics in the Prometheus text exposition format.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails (extremely unlikely; only on
    /// internal `prometheus` crate invariant violations).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_proxy::metrics::Metrics;
    ///
    /// let metrics = Metrics::new();
    /// let output = metrics.render().unwrap();
    /// assert!(output.contains("guardrail_requests_total"));
    /// ```
    pub fn render(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buf = Vec::new();
        encoder.encode(&metric_families, &mut buf)?;
        Ok(String::from_utf8(buf).expect("prometheus output is valid UTF-8"))
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_render_contains_metric_names() {
        let metrics = Metrics::new();
        let output = metrics.render().unwrap();
        assert!(output.contains("guardrail_requests_total"));
        assert!(output.contains("guardrail_blocked_total"));
        assert!(output.contains("guardrail_redacted_total"));
        assert!(output.contains("guardrail_pipeline_duration_seconds"));
        assert!(output.contains("guardrail_stage_duration_seconds"));
    }

    #[test]
    fn test_counters_increment() {
        let metrics = Metrics::new();
        metrics
            .requests_total
            .with_label_values(&["allow", "openai"])
            .inc();
        metrics
            .requests_total
            .with_label_values(&["allow", "openai"])
            .inc();
        metrics.blocked_total.with_label_values(&["toxicity"]).inc();
        metrics.redacted_total.inc();

        assert_eq!(
            metrics
                .requests_total
                .with_label_values(&["allow", "openai"])
                .get(),
            2
        );
        assert_eq!(
            metrics.blocked_total.with_label_values(&["toxicity"]).get(),
            1
        );
        assert_eq!(metrics.redacted_total.get(), 1);
    }

    #[test]
    fn test_histogram_observe() {
        let metrics = Metrics::new();
        metrics.pipeline_duration_seconds.observe(0.0012);
        metrics
            .stage_duration_seconds
            .with_label_values(&["regex_injection"])
            .observe(0.00003);

        let output = metrics.render().unwrap();
        assert!(output.contains("guardrail_pipeline_duration_seconds_bucket"));
        assert!(output.contains("stage=\"regex_injection\""));
    }
}
