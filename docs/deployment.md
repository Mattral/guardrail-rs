# Deployment

## Building a release binary

```bash
cargo build --release -p guardrail-cli
# binary at target/release/guardrail
```

For the ONNX-backed semantic classifiers:

```bash
cargo build --release -p guardrail-cli --features onnx
```

This links against the ONNX Runtime via the `ort` crate. See
[`docs/onnx-models.md`](onnx-models.md) for model setup.

## Docker

A multi-stage `Dockerfile` is provided at the repository root, producing a
minimal `distroless`-based image (~25 MB without ONNX, ~180 MB with ONNX due
to the ONNX Runtime shared library).

```bash
# CPU-only, no semantic classifiers (smallest image)
docker build -t guardrail-rs:latest .

# With ONNX support
docker build -t guardrail-rs:onnx --build-arg FEATURES=onnx .
```

Run it:

```bash
docker run -d \
  --name guardrail-rs \
  -p 8080:8080 \
  -v $(pwd)/guardrail.toml:/etc/guardrail/guardrail.toml:ro \
  -v $(pwd)/models:/etc/guardrail/models:ro \
  guardrail-rs:latest \
  run --config /etc/guardrail/guardrail.toml
```

### docker-compose

See [`docker-compose.yml`](../docker-compose.yml) for a complete example that
also runs Prometheus for scraping `/metrics`.

```bash
docker compose up -d
```

## Kubernetes

A minimal `Deployment` + `Service` + `ConfigMap` example:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: guardrail-config
data:
  guardrail.toml: |
    [server]
    listen_addr = "0.0.0.0:8080"
    upstream_url = "https://api.openai.com"

    [stages.regex_injection]
    enabled = true

    [stages.pii_redaction]
    enabled = true
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: guardrail-rs
spec:
  replicas: 3
  selector:
    matchLabels:
      app: guardrail-rs
  template:
    metadata:
      labels:
        app: guardrail-rs
    spec:
      containers:
        - name: guardrail-rs
          image: guardrail-rs:latest
          args: ["run", "--config", "/etc/guardrail/guardrail.toml"]
          ports:
            - containerPort: 8080
              name: http
          readinessProbe:
            httpGet: { path: /healthz, port: 8080 }
            initialDelaySeconds: 2
            periodSeconds: 5
          livenessProbe:
            httpGet: { path: /healthz, port: 8080 }
            initialDelaySeconds: 5
            periodSeconds: 10
          resources:
            requests: { cpu: "100m", memory: "64Mi" }
            limits:   { cpu: "500m", memory: "256Mi" }
          volumeMounts:
            - name: config
              mountPath: /etc/guardrail
              readOnly: true
      volumes:
        - name: config
          configMap:
            name: guardrail-config
---
apiVersion: v1
kind: Service
metadata:
  name: guardrail-rs
spec:
  selector:
    app: guardrail-rs
  ports:
    - port: 8080
      targetPort: http
```

Because `guardrail-rs` is stateless (each replica loads its own pipeline from
the mounted `ConfigMap`), horizontal scaling is straightforward — just
increase `replicas`. There is no shared state or coordination between
instances.

### Hot-reloading config in Kubernetes

`ConfigHandle::reload()` re-reads the config file path on disk. With a
`ConfigMap` volume mount, Kubernetes updates the mounted file (via a symlink
swap) within ~60 seconds of a `ConfigMap` change, but **`guardrail-rs` does
not currently watch the filesystem automatically** — `reload()` must be
triggered externally (e.g. a sidecar sending `SIGHUP`, or a future release
adding filesystem watching). For now, the simplest reliable approach is a
rolling restart on config change (`kubectl rollout restart deployment/guardrail-rs`).

## Resource sizing

Without `onnx`:

- **CPU:** the regex + PII stages are dominated by `RegexSet` matching,
  which is single-threaded per request but scales linearly with Tokio's
  worker pool. 1 vCPU comfortably handles several thousand requests/sec for
  typical (few-KB) prompts.
- **Memory:** ~10–20 MB RSS at idle; grows negligibly with traffic since no
  per-request state is retained beyond the request's lifetime.

With `onnx`:

- **CPU:** each ONNX inference call (`spawn_blocking`) takes ~1–5 ms on a
  modern x86_64 core for a 512-token input. Size your Tokio blocking thread
  pool (`tokio::runtime::Builder::max_blocking_threads`) according to
  expected concurrent inference load — the default of 512 is generally
  sufficient.
- **Memory:** add ~200–500 MB per loaded model (DeBERTa-v3-base and
  RoBERTa-base are both in this range in ONNX format with fp32 weights).
  Models are loaded once at startup and shared across all requests via `Arc`.

## Graceful shutdown

`guardrail run` installs SIGINT/SIGTERM handlers (Unix) or a Ctrl-C handler
(other platforms). On receipt, the accept loop stops immediately, but
already-accepted connections are given a short grace period (500ms) to
complete before the process exits. For zero-downtime deployments, ensure
your orchestrator's termination grace period is at least a few seconds longer
than your `upstream_timeout_secs` to allow in-flight upstream calls to
complete or time out.
