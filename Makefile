# Convenience targets for local development.
#
# Usage: make <target>

.PHONY: build build-onnx test test-doc bench lint fmt fmt-check deny \
        run validate check docker docker-up docker-down clean

CONFIG ?= guardrail.toml

build:
	cargo build --workspace

build-onnx:
	cargo build --workspace --features onnx

test:
	cargo test --workspace

test-doc:
	cargo test --workspace --doc

bench:
	cargo bench -p guardrail-classifiers

lint:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

deny:
	cargo deny check

run:
	cargo run -p guardrail-cli -- run --config $(CONFIG)

validate:
	cargo run -p guardrail-cli -- validate --config $(CONFIG)

check:
	cargo run -p guardrail-cli -- check "$(TEXT)" --config $(CONFIG)

docker:
	docker build -t guardrail-rs:latest .

docker-up:
	docker compose up -d

docker-down:
	docker compose down

clean:
	cargo clean
