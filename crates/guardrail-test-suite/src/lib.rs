//! `guardrail-test-suite`
//!
//! This crate contains no production code. It exists solely to host
//! end-to-end integration tests under `tests/`, which exercise the full
//! stack: [`guardrail_config`] → [`guardrail_proxy`] → a mocked upstream
//! LLM API (via `wiremock`).
//!
//! Run with:
//!
//! ```text
//! cargo test -p guardrail-test-suite
//! ```
