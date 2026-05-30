# Configuration Reference

`guardrail-rs` is configured via a single TOML file (default:
`guardrail.toml`). See [`guardrail.example.toml`](../guardrail.example.toml)
for a complete, annotated example. This document is the field-by-field
reference, matching `crates/guardrail-config/src/schema.rs` exactly.

## `[server]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | string | `"127.0.0.1"` | Bind host. |
| `port` | integer | `8080` | Bind port. |
| `workers` | integer | `0` | Number of Tokio worker threads. `0` = number of logical CPUs. |
| `max_body_size_bytes` | integer | `10485760` (10 MiB) | Maximum accepted client request body size. Larger bodies receive `413 Payload Too Large`. |

`server.host` and `server.port` are combined into a socket address via
`ServerConfig::listen_addr()` (`"<host>:<port>"`).

## `[upstream]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | string | `"https://api.openai.com"` | Base URL of the upstream provider. Must start with `http://` or `https://`. |
| `timeout_secs` | integer | `120` | Per-request timeout to the upstream. |
| `connect_timeout` | integer | `10` | TCP connection timeout. |

## `[auth]`

Optional caller authentication, separate from the upstream provider's
`Authorization` header (which is always forwarded opaquely and never
inspected by guardrail-rs).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `require_key` | bool | `false` | If `true`, every proxy request must present a matching key in `X-Guardrail-Key`. `/healthz` and `/metrics` are exempt. |
| `keys` | array of strings | `[]` | Accepted keys. If `require_key = true` and this is empty, **all** requests are rejected — validation will flag this as an error. |

The `X-Guardrail-Key` header is stripped before forwarding upstream; the LLM
provider never sees it. See [`docs/threat-model.md`](threat-model.md#6-unauthorized-callers-optional-auth)
for the security properties and residual risk of this mechanism.

```toml
[auth]
require_key = true
keys = ["grk-your-generated-key-here"]
```

## `[pipeline]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `request_stages` | array of strings | `["regex_injection", "onnx_injection", "pii_redactor", "toxicity", "policy"]` | Ordered list of stage IDs to run on the request side. Removing an ID disables that stage regardless of its own `enabled` flag. Order is significant. |
| `response_stages` | array of strings | `["output_pii_redactor"]` | Ordered list of stage IDs to run on the response side. |
| `on_error` | `"allow"` \| `"block"` | `"allow"` | Behavior when the pipeline itself errors unexpectedly (distinct from a stage's own `Decision::Block`). |

Valid request stage IDs: `regex_injection`, `onnx_injection`, `pii_redactor`,
`toxicity`, `policy`. Valid response stage IDs: `output_pii_redactor`.
Unknown IDs are rejected by validation.

## `[stages.regex_injection]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable/disable this stage. |
| `rules_file` | string | `""` | Path to a custom rule file (one regex per line; `#` for comments). Empty string = use the bundled rule set. |
| `extra_rules` | array of strings | `[]` | Additional regex patterns appended to whichever rule set is active (bundled or `rules_file`). |
| `action` | `"block"` \| `"redact"` \| `"log_only"` | `"block"` | What to do on a match. `"log_only"` logs via `tracing::warn!` but allows the request — use this to dry-run new rules. |

## `[stages.pii_redactor]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable/disable this stage. |
| `entities` | array of strings | all seven types (see below) | Which entity types to detect and redact. |
| `action` | `"block"` \| `"redact"` \| `"log_only"` | `"redact"` | Action when PII is found. (Redact is the sensible default — blocking on PII detection is unusual but supported.) |
| `validate_luhn` | bool | `true` | Apply Luhn checksum validation to 13–19 digit sequences before treating them as credit card numbers. |
| `replacements` | table | see below | Custom replacement tokens per entity type. |
| `redact_responses` | bool | `false` | Also scan and redact PII in non-streaming LLM **responses** before returning them to the caller. Streaming responses are never affected — see [`docs/architecture.md`](architecture.md). |

> **Note:** `[stages.pii_redaction]` (old name) is still accepted as a TOML
> alias for `[stages.pii_redactor]` for backward compatibility, but new
> configs should use `pii_redactor`.

### Valid entity types and default replacements (`[stages.pii_redactor.replacements]`)

| Entity type | Default replacement | Notes |
|-------------|---------------------|-------|
| `email` | `[EMAIL]` | RFC 5321-style pattern. |
| `phone` | `[PHONE]` | US-style formats: `+1-555-867-5309`, `(555) 867-5309`, `555.867.5309`, `5558675309`. |
| `credit_card` | `[CARD]` | 13–19 digit sequences; subject to `validate_luhn`. |
| `ssn` | `[SSN]` | `123-45-6789` or `123 45 6789`. |
| `ip_address` | `[IP_ADDRESS]` | IPv4 and simplified IPv6. |
| `api_key` | `[API_KEY]` | OpenAI (`sk-...`), Anthropic (`sk-ant-...`), GitHub (`ghp_`/`gho_`/`ghs_`), and `Bearer <token>`. |
| `aws_key` | `[AWS_KEY]` | AWS access key IDs (`AKIA...`). |

Override any of these:

```toml
[stages.pii_redactor.replacements]
email = "<redacted-email>"
credit_card = "<redacted-card>"
```

## `[stages.onnx_injection]` and `[stages.toxicity]`

Both require building with `cargo build --features onnx` (or
`cargo build -p guardrail-cli --features onnx`) and providing model files —
see [`docs/onnx-models.md`](onnx-models.md).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable/disable this stage. |
| `model_path` | string | `""` | Path to the `.onnx` model file. **Required if `enabled = true`.** |
| `tokenizer_path` | string | `""` | Path to the HuggingFace `tokenizer.json`. **Required if `enabled = true`.** |
| `threshold` | float | `0.85` (injection) / `0.90` (toxicity) | Decision threshold in `[0.0, 1.0]`. Scores at or above this value result in the configured `action`. |
| `action` | `"block"` \| `"redact"` \| `"log_only"` | `"block"` | What to do on a positive classification. |
| `on_error` | `"allow"` \| `"block"` | `"allow"` | What to do if inference itself fails (model load error, malformed input). |

`[stages.toxicity]` additionally supports:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `scan_roles` | array of strings | `["user"]` | Which message roles to run through the classifier. |

## `[[policy.rules]]`

An array of tables, evaluated **in order**. The first **enabled** rule whose
`when` condition matches determines the outcome; if no rule matches, the
request is allowed. Each rule has a `when` table (the condition) and a `then`
table (the action) — this nested shape lets a rule read naturally as
"when X, then Y".

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | *(required, non-empty)* | Human-readable identifier, used in audit logs. |
| `enabled` | bool | `true` | Whether this rule is active. |
| `when` | table | *(required)* | Condition — see below. |
| `then.action` | `"allow"` \| `"redact"` \| `"block"` \| `"log_only"` | `"block"` | Action taken when `when` matches. |
| `then.message` | string | absent | Custom message returned in the `403` body when `then.action = "block"`. |

> **Note:** `"redact"` and `"log_only"` actions are currently treated as
> `"allow"` by the policy engine (they do not yet perform redaction
> themselves — use `[stages.pii_redactor]` for redaction). They are reserved
> for future use and accepted for forward-compatibility.

### `when` condition fields

Exactly one of these should be set per rule (if multiple are set, priority
is `always` > `content_contains` > `system_prompt_absent` > `token_count_exceeds`):

#### `content_contains`

```toml
[[policy.rules]]
name = "block-competitor-mentions"
[policy.rules.when]
content_contains = ["competitor-x", "competitor-y"]
[policy.rules.then]
action = "block"
message = "Mentions of named competitors are not permitted."
```

Matches if any keyword appears (case-insensitive) anywhere in the request's
message content (system + user + assistant + tool messages).

#### `system_prompt_absent`

```toml
[[policy.rules]]
name = "require-system-prompt"
[policy.rules.when]
system_prompt_absent = true
[policy.rules.then]
action = "block"
message = "Requests must include a system prompt."
```

Matches if the request contains no message with `role = "system"`.

#### `token_count_exceeds`

```toml
[[policy.rules]]
name = "enforce-token-budget"
[policy.rules.when]
token_count_exceeds = 8000
[policy.rules.then]
action = "block"
message = "Request exceeds the configured token budget."
```

Matches if the approximate token count (`word_count * 1.3`, rounded down)
across all messages exceeds the given value. This is a fast heuristic, not an
exact tokenizer count — pad your limit accordingly if you need precision.

#### `always`

```toml
[[policy.rules]]
name = "default-deny"
[policy.rules.when]
always = true
[policy.rules.then]
action = "block"
```

Always matches. Useful as a final catch-all rule.

## `[observability]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `log_level` | `"trace"` \| `"debug"` \| `"info"` \| `"warn"` \| `"error"` | `"info"` | Log verbosity, passed to `tracing_subscriber::EnvFilter`. |
| `log_format` | `"pretty"` \| `"json"` | `"pretty"` | Log output format. `"json"` is recommended for production log aggregation (Datadog, Splunk, Loki). |
| `metrics_port` | integer | `9090` | Reserved for a future release that serves `/metrics` on a separate port. Currently `/metrics` is served on `server.listen_addr()` alongside the proxy endpoints regardless of this setting. |
| `otlp_endpoint` | string | `""` | OpenTelemetry OTLP gRPC endpoint, e.g. `"http://localhost:4317"`. Empty disables OTLP export. Must start with `http://`, `https://`, or `grpc://` if set. |
| `audit_log` | table | *(see below)* | Structured NDJSON audit log settings. |

### `[observability.audit_log]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Whether the NDJSON audit log file is enabled. |
| `path` | string | `"./guardrail-audit.ndjson"` | File path to write audit records to. Parent directory is created automatically. |
| `max_size_mb` | integer | `100` | Intended size threshold for rotation. **Current limitation:** rotation is not yet size-based (see `crates/guardrail-proxy/src/audit_log.rs`); the file currently grows unbounded at this path. Size-based rotation honoring this field is tracked as follow-up work. |

**NDJSON record shape** (see also [`docs/architecture.md`](architecture.md)
and [`docs/threat-model.md`](threat-model.md)):

```json
{
  "timestamp": "2026-06-13T10:00:00.123Z",
  "request_id": "01J9XK...",
  "decision": "block",
  "stage": "onnx_injection",
  "reason": "ONNX injection classifier score 0.97 >= threshold 0.85",
  "code": "prompt_injection",
  "score": 0.97,
  "model": "gpt-4o",
  "provider": "openai",
  "message_count": 2,
  "pii_entities_found": [],
  "latency_pipeline_ms": 4.8,
  "latency_total_ms": 4.9
}
```

## Validation errors

`guardrail validate --config guardrail.toml` (and `ConfigHandle::load`) run
the checks in
[`guardrail-config/src/validate.rs`](../crates/guardrail-config/src/validate.rs),
including:

- `server.host`/`server.port` combined must parse as a `SocketAddr`.
- `server.max_body_size_bytes` must be `> 0`.
- `upstream.url` must start with `http://` or `https://`.
- `upstream.timeout_secs` must be `> 0`.
- `auth.require_key = true` requires a non-empty `auth.keys` list; each key must be non-blank.
- `pipeline.request_stages` / `response_stages` entries must be recognized stage IDs.
- `stages.pii_redactor.entities` must be non-empty (if enabled) and contain only recognized entity names.
- `stages.onnx_injection` / `stages.toxicity`, if enabled: `model_path` (if set) must exist on disk, and `threshold` must be in `[0.0, 1.0]`.
- Each `policy.rules[i].name` must be non-empty.
- Each policy rule's `when` table must set at least one condition field.
- `observability.log_level` must be one of the five valid levels.
- `observability.log_format` must be `"pretty"` or `"json"`.
- `observability.otlp_endpoint`, if non-empty, must start with `http://`, `https://`, or `grpc://`.
- `observability.audit_log`, if enabled, requires a non-empty `path` and `max_size_mb > 0`.

All errors are collected and reported together (not just the first one).

## Environment variable overrides

Applied after the TOML file is parsed, before validation:

| Variable | Overrides |
|----------|-----------|
| `GUARDRAIL_UPSTREAM` | `upstream.url` |
| `GUARDRAIL_PORT` | `server.port` |
| `GUARDRAIL_LOG_LEVEL` | `observability.log_level` |
| `GUARDRAIL_OTLP_ENDPOINT` | `observability.otlp_endpoint` |

Empty or unparseable values are silently ignored (the TOML value is kept).

## Hot reload

Send `SIGHUP` to the running process (Unix only) to reload the configuration
file without dropping connections — see [`docs/architecture.md`](architecture.md#hot-reload).
