//! A request through the proxy produces the documented OpenTelemetry spans.

use std::io::Write;
use std::sync::Arc;

use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};
use serde_json::json;
use tracing_subscriber::prelude::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn attr(span: &opentelemetry_sdk::trace::SpanData, key: &str) -> Option<String> {
    span.attributes
        .iter()
        .find(|kv| kv.key.as_str() == key)
        .map(|kv| kv.value.to_string())
}

// Current-thread runtime: the subscriber set below is thread-local, and
// every task the server spawns runs on this thread.
#[tokio::test(flavor = "current_thread")]
async fn request_produces_request_stage_and_upstream_spans() {
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let subscriber =
        tracing_subscriber::registry().with(guardrail_proxy::telemetry::otel_layer(&provider));
    let _guard = tracing::subscriber::set_default(subscriber);

    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "x", "choices": []})))
        .mount(&upstream)
        .await;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        "[server]\nhost = \"127.0.0.1\"\nport = 0\n\n[upstream]\nurl = \"{}\"\n",
        upstream.uri()
    )
    .unwrap();
    let config = Arc::new(guardrail_config::ConfigHandle::load(f.path()).unwrap());
    let handle = guardrail_proxy::run_server(config).await.unwrap();
    let addr = handle.local_addr();

    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/v1/chat/completions"))
        .json(&json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "mail alice@example.com"}]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.bytes().await.unwrap();
    handle.shutdown();
    provider.force_flush().unwrap();

    let spans = exporter.get_finished_spans().unwrap();
    let names: Vec<&str> = spans.iter().map(|s| s.name.as_ref()).collect();
    let request = spans
        .iter()
        .find(|s| s.name == "guardrail.request")
        .unwrap_or_else(|| panic!("no guardrail.request span in {names:?}"));
    let request_id = request.span_context.span_id();

    assert_eq!(attr(request, "decision").as_deref(), Some("redact"));
    assert_eq!(attr(request, "http.status_code").as_deref(), Some("200"));
    assert_eq!(attr(request, "entities_found").as_deref(), Some("1"));
    assert_eq!(attr(request, "model").as_deref(), Some("gpt-4o"));

    let children: Vec<_> = spans
        .iter()
        .filter(|s| s.parent_span_id == request_id)
        .collect();
    let child_names: Vec<&str> = children.iter().map(|s| s.name.as_ref()).collect();
    for stage in ["regex_injection", "pii_redactor"] {
        let name = format!("guardrail.stage.{stage}");
        let span = children
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no {name} under the request span: {child_names:?}"));
        assert!(attr(span, "decision").is_some(), "{name} has no decision");
    }
    let pii = children
        .iter()
        .find(|s| s.name == "guardrail.stage.pii_redactor")
        .unwrap();
    assert_eq!(attr(pii, "decision").as_deref(), Some("redact"));
    assert_eq!(attr(pii, "entities_found").as_deref(), Some("1"));

    let upstream_span = children
        .iter()
        .find(|s| s.name == "guardrail.upstream.forward")
        .unwrap_or_else(|| panic!("no upstream span: {child_names:?}"));
    assert_eq!(
        attr(upstream_span, "http.status_code").as_deref(),
        Some("200")
    );
}
