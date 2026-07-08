//! `guardrail-test-suite`
//!
//! This crate contains no production code. It exists solely to host
//! end-to-end integration tests under `tests/`, which exercise the full
//! stack: `guardrail_config` → `guardrail_proxy` → a mocked upstream LLM
//! API (via `wiremock`). (Plain code text, not links: both are
//! dev-dependencies only, so they aren't available to rustdoc when it
//! builds this crate's own lib docs — only when compiling the tests
//! themselves.)
//!
//! Run with:
//!
//! ```text
//! cargo test -p guardrail-test-suite
//! ```
