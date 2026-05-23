# syntax=docker/dockerfile:1.7

# ── Builder ────────────────────────────────────────────────────────────────
FROM rust:1.81-slim-bookworm AS builder

ARG FEATURES=""

WORKDIR /build

# Install build dependencies. `pkg-config` + `libssl-dev` are required by
# transitive deps even though guardrail-rs itself uses rustls; `protobuf`
# headers may be needed by the `onnx` feature's `ort` crate on some platforms.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first to leverage Docker layer caching for dependencies.
COPY Cargo.toml Cargo.lock ./
COPY crates/guardrail-core/Cargo.toml crates/guardrail-core/Cargo.toml
COPY crates/guardrail-classifiers/Cargo.toml crates/guardrail-classifiers/Cargo.toml
COPY crates/guardrail-config/Cargo.toml crates/guardrail-config/Cargo.toml
COPY crates/guardrail-proxy/Cargo.toml crates/guardrail-proxy/Cargo.toml
COPY crates/guardrail-cli/Cargo.toml crates/guardrail-cli/Cargo.toml
COPY crates/guardrail-test-suite/Cargo.toml crates/guardrail-test-suite/Cargo.toml

# Create dummy source files so `cargo build` can resolve and cache
# dependencies before the real source is copied in.
RUN for crate in guardrail-core guardrail-classifiers guardrail-config guardrail-proxy guardrail-test-suite; do \
        mkdir -p crates/$crate/src && echo "fn main() {}" > crates/$crate/src/lib.rs; \
    done && \
    mkdir -p crates/guardrail-cli/src && \
    echo "fn main() {}" > crates/guardrail-cli/src/main.rs

RUN cargo build --release -p guardrail-cli ${FEATURES:+--features "$FEATURES"} || true

# Now copy the real source and build for real.
COPY crates ./crates

RUN touch crates/*/src/*.rs crates/*/src/**/*.rs 2>/dev/null || true
RUN cargo build --release -p guardrail-cli ${FEATURES:+--features "$FEATURES"}

# ── Runtime ───────────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /etc/guardrail

COPY --from=builder /build/target/release/guardrail /usr/local/bin/guardrail
COPY guardrail.example.toml /etc/guardrail/guardrail.example.toml

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/guardrail"]
CMD ["run", "--config", "/etc/guardrail/guardrail.toml"]
