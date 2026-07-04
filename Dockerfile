# syntax=docker/dockerfile:1.7

# ── Builder ────────────────────────────────────────────────────────────────
# `1-slim-bookworm` (not a pinned patch version): floats to the latest
# stable Rust release, same philosophy as `rust-toolchain.toml`
# (`channel = "stable"`) and every GitHub Actions workflow in this repo
# (`dtolnay/rust-toolchain@stable`). This used to be pinned to
# `rust:1.81-slim-bookworm`, which broke the `docker` release job outright:
# transitive dependencies (e.g. `time-core`) started requiring the
# `edition2024` Cargo feature, stabilized in Cargo 1.85, so `cargo build`
# failed at the manifest-parsing stage before compiling anything. Pinning a
# specific version here re-introduces the same class of bug the moment any
# dependency bumps its MSRV past whatever was pinned — floating on `1`
# avoids that, at the cost of the build environment being able to change
# under you between builds (acceptable for a from-source build stage that
# doesn't get published as its own image).
FROM rust:1-slim-bookworm AS builder

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
# dependencies before the real source is copied in. This has to include a
# stub for every `[[bench]]` target declared in any workspace member's
# Cargo.toml too (guardrail-classifiers' `classifier_benchmarks` and
# guardrail-test-suite's `pipeline`) — Cargo parses every workspace
# member's manifest just to load the workspace, even when only building
# one package with `-p`, and a `[[bench]]` entry pointing at a
# not-yet-existing file fails that parse immediately. Without these two
# stubs, the `cargo build ... || true` below silently fails before ever
# fetching a single dependency (visible in `docker build` output as
# `error: failed to parse manifest ... can't find classifier_benchmarks
# bench`), which defeats the entire point of this layer: every build was
# re-downloading and re-compiling the full dependency tree from scratch
# instead of hitting the Docker layer cache.
RUN for crate in guardrail-core guardrail-classifiers guardrail-config guardrail-proxy guardrail-test-suite; do \
        mkdir -p crates/$crate/src && echo "fn main() {}" > crates/$crate/src/lib.rs; \
    done && \
    mkdir -p crates/guardrail-cli/src && \
    echo "fn main() {}" > crates/guardrail-cli/src/main.rs && \
    mkdir -p crates/guardrail-classifiers/benches && \
    echo "fn main() {}" > crates/guardrail-classifiers/benches/classifier_benchmarks.rs && \
    mkdir -p crates/guardrail-test-suite/benches && \
    echo "fn main() {}" > crates/guardrail-test-suite/benches/pipeline.rs

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
