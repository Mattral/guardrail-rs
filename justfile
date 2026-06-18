# guardrail-rs justfile
# Install: https://github.com/casey/just
# Usage:   just <recipe>

# Default recipe — list available commands.
default:
    @just --list

# ── Build ─────────────────────────────────────────────────────────────────────

# Build all workspace crates (debug).
build:
    cargo build --workspace

# Build all workspace crates (release).
build-release:
    cargo build --workspace --release

# Build with ONNX semantic classifiers enabled.
build-onnx:
    cargo build --workspace --features onnx

# Build the `guardrail` CLI binary (release).
bin:
    cargo build --release -p guardrail-cli
    @echo "Binary: target/release/guardrail"

# ── Test ──────────────────────────────────────────────────────────────────────

# Run all tests with cargo-nextest.
test:
    cargo nextest run --workspace

# Run doc-tests.
test-doc:
    cargo test --workspace --doc

# Run integration tests only (guardrail-test-suite).
test-e2e:
    cargo nextest run -p guardrail-test-suite

# Run unit tests for a specific crate.
test-crate crate:
    cargo nextest run -p {{ crate }}

# Run all tests including doc-tests (CI equivalent).
test-all: test test-doc

# ── Lint & Format ─────────────────────────────────────────────────────────────

# Run clippy (deny all warnings).
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format all code.
fmt:
    cargo fmt --all

# Check formatting without writing (CI equivalent).
fmt-check:
    cargo fmt --all -- --check

# ── Coverage ──────────────────────────────────────────────────────────────────

# Generate and display coverage (requires cargo-tarpaulin).
coverage:
    cargo tarpaulin --workspace --exclude guardrail-test-suite \
        --out Html --output-dir coverage/ --timeout 300 --ignore-tests
    @echo "Report: coverage/tarpaulin-report.html"

# ── Benchmarks ────────────────────────────────────────────────────────────────

# Run all classifier microbenchmarks.
bench:
    cargo bench -p guardrail-classifiers

# Run the full-pipeline integration benchmark (spec §13 latency gate).
bench-pipeline:
    cargo bench -p guardrail-test-suite --bench pipeline

# Run every benchmark in the workspace.
bench-all: bench bench-pipeline

# Run a specific classifier benchmark filter.
bench-filter filter:
    cargo bench -p guardrail-classifiers -- {{ filter }}

# Save a benchmark baseline named `before` (classifier benches).
bench-save:
    cargo bench -p guardrail-classifiers -- --save-baseline before

# Compare against the `before` baseline (classifier benches).
bench-compare:
    cargo bench -p guardrail-classifiers -- --baseline before

# ── Security ──────────────────────────────────────────────────────────────────

# Check licenses and advisories.
deny:
    cargo deny check

# Run security advisory check.
audit:
    cargo audit

# ── CLI shortcuts ─────────────────────────────────────────────────────────────

CONFIG := "guardrail.example.toml"

# Validate the config file.
validate config=CONFIG:
    cargo run -p guardrail-cli -- validate --config {{ config }}

# Start the proxy server.
run config=CONFIG:
    cargo run -p guardrail-cli -- run --config {{ config }}

# Start the proxy server (release binary).
run-release config=CONFIG:
    cargo build --release -p guardrail-cli
    target/release/guardrail run --config {{ config }}

# Check a text payload through the pipeline without starting the server.
check text config=CONFIG:
    cargo run -p guardrail-cli -- check "{{ text }}" --config {{ config }}

# ── Docker ────────────────────────────────────────────────────────────────────

# Build the Docker image.
docker-build:
    docker build -t guardrail-rs:latest .

# Build the Docker image with ONNX support.
docker-build-onnx:
    docker build -t guardrail-rs:onnx --build-arg FEATURES=onnx .

# Start the proxy + Prometheus in Docker Compose.
docker-up:
    docker compose up -d
    @echo "Proxy:      http://localhost:8080"
    @echo "Prometheus: http://localhost:9091"

# Stop Docker Compose services.
docker-down:
    docker compose down

# Tail proxy logs from Docker Compose.
docker-logs:
    docker compose logs -f guardrail-rs

# ── Model export (ONNX) ───────────────────────────────────────────────────────

# Export ONNX model files (requires Python + optimum).
models:
    bash models/export_models.sh

# ── Examples ──────────────────────────────────────────────────────────────────

# Run the minimal embedded-pipeline example (no proxy server, pure library use).
example-minimal-rs:
    cargo run --example minimal -p guardrail-cli

# Run the embedded-pipeline Rust example.
example-embedded:
    cargo run --example embedded_pipeline -p guardrail-classifiers

# Run the custom-stage Rust example.
example-custom-stage:
    cargo run --example custom_stage -p guardrail-cli

# Run the Python client example (requires: pip install openai, proxy running).
example-python:
    python3 examples/python_client.py

# Run the Node.js client example (requires: npm install openai, proxy running).
example-node:
    node examples/node_client.js

# Smoke-test a running proxy with curl.
example-curl addr="http://localhost:8080":
    bash examples/curl_test.sh {{ addr }}

# ── Docs ──────────────────────────────────────────────────────────────────────

# Open the API documentation in a browser.
doc:
    cargo doc --workspace --open --no-deps

# ── Clean ─────────────────────────────────────────────────────────────────────

# Remove build artifacts.
clean:
    cargo clean

# Remove build artifacts and generated coverage/benchmark data.
clean-all: clean
    rm -rf coverage/ target/criterion/

# ── CI (full local CI run) ────────────────────────────────────────────────────

# Run the full CI suite locally: fmt + lint + test + doc + deny.
ci: fmt-check lint test test-doc deny
    @echo ""
    @echo "✓ All CI checks passed."
