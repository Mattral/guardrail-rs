# Configuration Reference

`guardrail-rs` is configured via a single TOML file (default:
`guardrail.toml`). See [`guardrail.example.toml`](../guardrail.example.toml)
for a complete, annotated example. This document is the field-by-field
reference.

## `[server]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `listen_addr` | string | *(required)* | Socket address the proxy binds to, e.g. `"0.0.0.0:8080"`. |
| `upstream_url` | string | *(required)* | Base URL of the upstream provider. Must start with `http://` or `https://`. |
| `upstream_timeout_secs` | integer | `60` | Per-request timeout to the upstream. |
| `max_body_size_bytes` | integer | `10485760` (10 MiB) | Maximum accepted client request body size. Larger bodies receive `413 Payload Too Large`. |

## `[pipeline]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `on_error` | `"allow"` \| `"block"` | `"allow"` | Behavior when the pipeline itself errors (not the same as a stage `Block` decision — see [architecture.md](architecture.md#error-handling-fail-open-vs-fail-closed)). |

## `[stages.regex_injection]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable/disable this stage. |
| `custom_rules_path` | string \| absent | absent | Path to a custom rule file (one regex per line; `#` for comments). Replaces the bundled rule set entirely. |
| `log_only` | bool | `false` | If `true`, matches are logged via `tracing::warn!` but the request is **allowed**. Use this to dry-run new rules. |

## `[stages.pii_redaction]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Enable/disable this stage. |
| `entities` | array of strings | all seven types (see below) | Which entity types to detect and redact. |
| `validate_luhn` | bool | `true` | Apply Luhn checksum validation to 13–19 digit sequences before treating them as credit card numbers. |

### Valid entity types

| Entity type | Replacement token | Notes |
|-------------|-------------------|-------|
| `email` | `[EMAIL]` | RFC 5321-style pattern. |
| `phone` | `[PHONE]` | US-style formats: `+1-555-867-5309`, `(555) 867-5309`, `555.867.5309`, `5558675309`. |
| `credit_card` | `[CARD]` | 13–19 digit sequences; subject to `validate_luhn`. |
| `ssn` | `[SSN]` | `123-45-6789` or `123 45 6789`. |
| `ip_address` | `[IP_ADDRESS]` | IPv4 and simplified IPv6. |
| `api_key` | `[API_KEY]` | OpenAI (`sk-...`), Anthropic (`sk-ant-...`), GitHub (`ghp_`/`gho_`/`ghs_`), and `Bearer <token>`. |
| `aws_key` | `[AWS_KEY]` | AWS access key IDs (`AKIA...`). |

## `[stages.onnx_injection]` and `[stages.toxicity]`

Both require building with `cargo build --features onnx` (or
`cargo build -p guardrail-cli --features onnx`) and providing model files —
see [`docs/onnx-models.md`](onnx-models.md).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable/disable this stage. |
| `model_path` | string | absent | Path to the `.onnx` model file. **Required if `enabled = true`.** |
| `tokenizer_path` | string | absent | Path to the HuggingFace `tokenizer.json`. **Required if `enabled = true`.** |
| `threshold` | float | `0.85` (injection) / `0.90` (toxicity) | Decision threshold in `[0.0, 1.0]`. Scores at or above this value result in `Block`. |

## `[[policy.rules]]`

An array of tables, evaluated **in order**. The first **enabled** rule whose
`condition` matches determines the outcome; if no rule matches, the request
is allowed.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | *(required, non-empty)* | Human-readable identifier, used in audit logs. |
| `enabled` | bool | `true` | Whether this rule is active. |
| `condition` | table | *(required)* | See below. |
| `action` | `"allow"` \| `"redact"` \| `"block"` \| `"log_only"` | *(required)* | Action taken when `condition` matches. |
| `message` | string | absent | Custom message returned in the `403` body when `action = "block"`. |

> **Note:** `"redact"` and `"log_only"` actions are currently treated as
> `"allow"` by the policy engine (they do not yet perform redaction
> themselves — use `[stages.pii_redaction]` for redaction). They are
> reserved for future use and are accepted for forward-compatibility.

### Condition types

#### `content_contains`

```toml
condition.type = "content_contains"
condition.keywords = ["competitor-x", "competitor-y"]
```

Matches if any keyword appears (case-insensitive) anywhere in the request's
message content (system + user + assistant + tool messages).

#### `system_prompt_absent`

```toml
condition.type = "system_prompt_absent"
```

Matches if the request contains no message with `role = "system"`.

#### `token_count_exceeds`

```toml
condition.type = "token_count_exceeds"
condition.limit = 8000
```

Matches if the approximate token count (`word_count * 1.3`, rounded down)
across all messages exceeds `limit`. This is a fast heuristic, not an exact
tokenizer count — pad your limit accordingly if you need precision.

#### `always`

```toml
condition.type = "always"
```

Always matches. Useful as a final catch-all rule (e.g. a default-deny policy
for system prompts that don't otherwise match an allow rule).

## `[observability]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `metrics_addr` | string | `"0.0.0.0:9090"` | **Reserved for future use.** `/metrics` is currently served on `server.listen_addr`. |
| `log_level` | `"trace"` \| `"debug"` \| `"info"` \| `"warn"` \| `"error"` | `"info"` | Log verbosity, passed to `tracing_subscriber::EnvFilter`. |
| `json_logs` | bool | `false` | Emit logs as JSON instead of human-readable text. Recommended for production log aggregation. |

## Validation errors

`guardrail validate --config guardrail.toml` (and `ConfigHandle::load`) run
the checks in
[`guardrail-config/src/validate.rs`](../crates/guardrail-config/src/validate.rs),
including:

- `server.listen_addr` must parse as a `SocketAddr`.
- `server.upstream_url` must start with `http://` or `https://`.
- `server.max_body_size_bytes` must be `> 0`.
- `stages.pii_redaction.entities` must be non-empty (if enabled) and contain
  only recognized entity names.
- `stages.onnx_injection` / `stages.toxicity`, if enabled, must specify both
  `model_path` and `tokenizer_path`, and `threshold` must be in `[0.0, 1.0]`.
- Each `policy.rules[i].name` must be non-empty.
- `content_contains` conditions must have a non-empty `keywords` list.
- `token_count_exceeds` conditions must have `limit > 0`.
- `observability.metrics_addr` must parse as a `SocketAddr`.
- `observability.log_level` must be one of the five valid levels.

All errors are collected and reported together (not just the first one).
