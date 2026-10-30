# Threat Model

`guardrail-rs` defends against threats at the boundary between a **caller**
(your application) and an **upstream LLM provider** (OpenAI, Anthropic, etc.).
This document describes what guardrail-rs protects against, its explicit trust
assumptions, and the threats it does **not** address.

## Trust boundaries

```text
[Caller / Application]
        │  HTTP (can be untrusted)
        ▼
┌─────────────────────────────┐
│       guardrail-rs           │  ← trust boundary
│                              │
│  regex_injection             │
│  onnx_injection (optional)   │
│  pii_redactor                │
│  toxicity (optional)         │
│  policy_engine               │
└──────────────┬───────────────┘
               │  HTTPS (TLS to upstream)
               ▼
      [Upstream LLM Provider]
```

## In scope: what guardrail-rs protects against

### 1. Prompt injection (request side)

**Threat:** An end-user or attacker crafts a message that overrides the
application's system prompt, coercing the model into performing unintended
actions — leaking secrets, bypassing filters, role-playing as an unconstrained
persona.

**Mitigations:**

- `RegexInjectionScanner` (always on) — single-pass `RegexSet` over 30+
  bundled patterns covering jailbreaks, system-prompt extraction,
  delimiter-injection, and capability-elicitation attempts. Catches the
  vast majority of known attack payloads in < 50 µs.
- `OnnxInjectionClassifier` (optional, `onnx` feature) — DeBERTa-based
  semantic classifier that catches paraphrased or novel injection attempts
  that evade the regex layer. Operates at the sentence-embedding level so
  adversarial rephrasing has diminishing returns.

**Residual risk:** Highly novel injection techniques (new jailbreak styles,
language-level obfuscation, multi-turn attacks across separate requests) may
evade both layers. Defense-in-depth at the application layer — careful system
prompt design, output validation, tool sandboxing — remains essential.

### 2. PII leakage (request side)

**Threat:** A user (intentionally or accidentally) pastes personally
identifiable information — email addresses, phone numbers, credit card
numbers, SSNs, API keys — into a prompt. This PII is sent to an external
provider, creating a data-governance and compliance risk.

**Mitigations:**

- `PiiRedactor` (always on by default) — regex-based detection of 7 entity
  types (email, phone, credit_card with Luhn validation, SSN, IP address,
  API key patterns, AWS access key IDs). Replaces matched spans with typed
  placeholders (`[EMAIL]`, `[PHONE]`, etc.) before forwarding.
- `redact_responses = true` (opt-in) — redacts PII the model may have
  echoed back from retrieved documents or training data, in complete JSON
  responses and, incrementally, in streamed (`stream: true`) responses.

**Residual risk:**

- Regex-only approach cannot detect names, addresses, or organization names
  (NER-based detection is planned via `onnx-pii` feature).
- Response redaction is off by default.
- Responses that response redaction can't inspect (body not valid JSON,
  content type other than JSON or SSE) are forwarded unredacted. They are
  counted in `guardrail_response_redaction_skipped_total{reason}`; alert on
  it. Compressed bodies are decoded first, and an encoding the proxy can't
  decode is refused with `502`.
- In streamed responses, only text deltas are scanned, as in complete
  responses: tool-call arguments are not. A run of more than 8 KB without
  whitespace is released without waiting for a word boundary, so a PII value
  straddling that point could be missed. Deployments that need redaction
  applied to complete responses only can set
  `stages.pii_redactor.streaming_responses = "reject"`. See
  [`architecture.md`](architecture.md#streaming-responses).
- Luhn validation reduces false positives for credit card numbers, but cannot
  catch all edge cases of arbitrary numeric strings.
- Redaction is one-way: the original value is not stored and cannot be
  recovered from the audit log (by design, per our privacy model).

### 3. Toxic content (request side)

**Threat:** Users submitting hate speech, harassment, self-harm, or other
harmful content to the LLM, using it as a relay or amplifier.

**Mitigations:**

- `ToxicityClassifier` (optional, `onnx` feature) — unbiased-toxic-roberta
  classifier with configurable threshold (default 0.90 to minimize
  false positives in ambiguous cases).

**Residual risk:** High threshold reduces false positives but increases false
negatives for borderline content. Creative writing, fiction, and academic
discussion of sensitive topics may require a per-deployment threshold tuning.

### 4. Policy violations (configurable rules)

**Threat:** Use-case-specific risks not covered by the generic classifiers —
e.g. competitor mentions in a corporate deployment, requests exceeding token
budgets, or missing required system prompts.

**Mitigations:**

- `PolicyEngine` — user-defined rules evaluated last in the pipeline.
  Conditions: `content_contains`, `system_prompt_absent`,
  `token_count_exceeds`, `always`. See `docs/policy-rules.md`.

### 5. API key exposure in prompts

**Threat:** A developer accidentally embeds an API key or secret token in a
prompt during testing.

**Mitigations:**

- PII redactor covers OpenAI (`sk-*`), Anthropic (`sk-ant-*`), GitHub
  (`ghp_*`, `gho_*`, `ghs_*`), and AWS access keys (`AKIA*`). `Bearer`
  tokens of ≥ 20 characters are also redacted.

**Residual risk:** Custom/proprietary token formats not matching the bundled
patterns are not detected. Add a `custom_rules_path` with targeted regexes for
your environment's secret formats.

---

### 6. Unauthorized callers (optional, `[auth]`)

**Threat:** Any process that can reach the proxy's listen address can send
requests through it (and, if `require_key` is off, on to the upstream
provider using whatever credentials the caller supplies).

**Mitigations:**

- `[auth] require_key = true` + `keys = [...]` — every request to a proxy
  endpoint must present a matching key in the `X-Guardrail-Key` header or is
  rejected with HTTP 401 before the pipeline runs and before the upstream is
  contacted. `/healthz` and `/metrics` are exempt so monitoring keeps working
  without a key. `X-Guardrail-Key` is stripped before forwarding upstream —
  the LLM provider never sees it.

**Residual risk:** this is a simple shared-secret check, not OAuth/JWT/mTLS.
Keys are compared in constant time (`subtle::ConstantTimeEq`, checking every
configured key), so response timing doesn't reveal how much of a guess was
right or which key matched. A key's *length* can still be inferred, so use
long random keys. There is no key
revocation list or expiry; rotating a compromised key requires editing
`auth.keys` and reloading (SIGHUP) or restarting. For internet-facing
deployments, pair this with a proper API gateway or WAF.

## Out of scope: what guardrail-rs does NOT protect against

### Response-side injection / indirect prompt injection

Tool results and tool-call arguments in the request history are scanned,
because they are the main route for indirect injection: every stage that
scans all messages (regex injection, PII, policy) sees them, and
`onnx_injection` scores user messages and tool results (OpenAI `tool`
messages; Anthropic `tool_result` blocks, which sit in user messages).
Content the upstream fetches itself (server-side tools, web search,
retrieval) never passes through the proxy and is not inspected, and
neither is the model's output, apart from PII redaction with
`redact_responses = true`. Mitigate with careful tool design and output
validation at the application layer.

### Multi-turn context injection

Guardrail-rs evaluates each HTTP request independently. A multi-turn attack
that spreads across separate requests (each individually appearing benign) is
not detected. Application-level conversation state analysis is required.

### Data exfiltration via model outputs

The model's response may leak data that was present in the system prompt or
retrieved context — guardrail-rs only redacts PII patterns in the structured
response JSON fields it understands. Semantic data exfiltration (the model
paraphrasing a secret rather than quoting it verbatim) is not detected.

### Upstream provider compromise

guardrail-rs trusts the upstream LLM provider (OpenAI, Anthropic, etc.) to
return honest responses. If the provider is compromised or the TLS connection
is intercepted, guardrail-rs provides no protection.

### Denial of service (resource exhaustion)

`server.max_body_size_bytes` (default 10 MiB) is enforced while the body is
read. Reading stops once the limit is passed, and a declared
`Content-Length` over the limit is rejected before anything is read, so a
client can't make the proxy buffer more than that per request.
`upstream.timeout_secs` bounds the wait for the provider. The ONNX stages are
the most expensive part per request (tens to hundreds of ms per inference on
CPU; see [`benchmarks.md`](benchmarks.md#onnx-classifiers-measured-and-far-over-target)).
They score at most `max_windows` windows of 512 tokens (16 by default) and
block longer input with `input_too_long_for_classifier` unless
`on_oversize = "allow"`. These are basic limits: guardrail-rs is not designed to defend against
volumetric DoS, and there is no per-client rate limiting. A reverse proxy or
WAF in front of guardrail-rs is recommended in public deployments.

#### Long input holds ONNX sessions

The most expensive request a client can send is one just under the
`max_windows` limit. Each ONNX stage scores one request per session, and
with the defaults a few such requests occupy every session. Measured on an
8-core machine with `max_windows = 16` and the automatic pool, 4 sessions
per stage, at `intra_threads = 2` (the default) and, in brackets, 1 (the
default before 0.4.0) ([benchmarks](benchmarks.md#long-input)):

- 16 windows is 6,272 tokens, about 38 KB of plain English. One such
  request holds an `onnx_injection` session for 11.7 s (21 s), then a
  `toxicity` session for 7.6 s (15 s).
- 4 of them sent together hold all 4 `onnx_injection` sessions for about
  27 s (31 s). Every other request waits behind them. A client that keeps 4 such
  requests in flight (about 150 KB every 30 s) stalls the ONNX stages for
  everyone. On a smaller machine the pool is smaller, and fewer requests
  are needed.
- `pipeline.stage_timeout_ms` is `0` (no limit) by default, so the waiting
  requests wait as long as it takes.

A stage timeout doesn't free a session: inference that has started runs to
the end in the background. What it bounds is how long other requests wait.
They time out and are handled by the stage's `on_error`, and with the
default `"allow"` that means **forwarded without the check**. With a
timeout, set `on_error = "block"` on the ONNX stages, so that they answer
`403 stage_error` instead. That turns a stall into quick errors for
everyone else; it doesn't give the capacity back.

**Lowering `max_windows` doesn't help.** The cost is per window, about the
same whatever the limit. With 4 such requests at once on 4 sessions, at
`intra_threads = 1`:

| `max_windows` | longest input scored | `onnx_injection` | `toxicity` |
|---:|---|---:|---:|
| 16 (default) | 6,272 tokens, about 38 KB | 31 s | 19 s |
| 8 | 3,200 tokens, about 19 KB | 15.5 s | 9.4 s |
| 4 | 1,664 tokens, about 10 KB | 7.6 s | 4.5 s |

A lower limit shortens how long one request holds a session, but a client
keeps the pool just as busy with twice as many requests of half the size,
for the same bytes per second. Meanwhile every legitimate request between
the new limit and the old one is refused: at `max_windows = 8`, input of
3,200–6,272 tokens gets `403 input_too_long_for_classifier` (or skips the
check with `on_oversize = "allow"`). The default stays at 16. What would
bound this is admission control in front of each pool (a bounded wait,
then `503` with `Retry-After`), which guardrail-rs doesn't have yet.

Input over the limit is resolved by `on_oversize` without running the
model, and without taking a session. It is tokenized first, and never
further than could fit the limit: 32 bytes for each token the limit covers,
196 KiB at `max_windows = 16`, where text runs at 2–8 bytes per token.
Longer input whose first 196 KiB fit within the limit is oversize as well:
it takes more than 32 bytes per token, which some input does (below), and
measuring all of it would cost what the limit is there to bound. It is
counted in `guardrail_stage_oversize_total` with `reason="unmeasured"`
(`"over_limit"` otherwise). So turning away a large input costs about as
much as a small one. Up to 0.3.1 the stages tokenized all of the input,
and nothing bounded how many requests did that at once. Through the proxy
with both stages, at the default body limit of 10 MiB, 3 runs (16 at once:
one run):

| 10 MiB messages at once | 0.3.1: `403` after | 0.3.1: peak memory rise | now: `403` after | now: peak memory rise |
|---:|---:|---:|---:|---:|
| 1 | 3.4–3.5 s | 0.8 GB | 0.11–0.13 s | 50 MB |
| 4 | 3.8–4.5 s | 2.5 GB | 0.11–0.19 s | 0.25 GB |
| 8 | 5.5–6.7 s | 3.9 GB | 0.16–0.21 s | 0.43 GB |
| 16 | not recorded | 11.9 GB | 0.30–0.40 s | 1.1 GB |

0.3.1 locked its one session per window, after tokenizing, so other
requests were held up little (a short request sent alongside took
0.55–0.78 s, 1.4 s next to 16, against about 0.55 s alone), but a few dozen
such requests at once could run the proxy out of memory. Images and base64
documents in a request are not tokenized by either stage.

Input built to take few tokens per byte, as 4 MiB bodies (3 runs):

| 4 MiB of | bytes per token, `onnx_injection` / `toxicity` | `403` after, both stages | `toxicity` alone |
|---|---|---:|---:|
| `-` or `=` | 1 / 64 | 0.12–0.17 s | 0.09–0.12 s |
| spaces | no tokens / 1 | 0.02–0.03 s | 0.11–0.12 s |
| newlines | no tokens / 2 | 0.03 s | 0.09 s |
| U+E000 (private use) | 2 tokens in all / 1.5 | 0.03 s | 0.82–0.91 s |
| `ÃÂ` (mojibake) | 4 / 128 | 0.05–0.06 s | 0.90–1.01 s |

The DeBERTa tokenizer drops whitespace and fuses a run of unknown
characters into one token; the RoBERTa one has single tokens for 64 dashes
and for 32 `ÃÂ` pairs (its longest, 128 bytes). Peak memory rose by at most
83 MiB for one request and 131 MiB for 4 at once. A short request sent
alongside 4 of them was answered in under 0.1 s with both stages, and in
up to 0.65 s with `toxicity` alone. That, and the 0.8–1 s for one request
with `toxicity` alone, is the PII redactor, which runs before it, scanning
4 MiB of non-ASCII text.

For a deployment that accepts traffic it doesn't trust:

1. **Limit requests in front of the proxy.** A per-client limit on
   concurrent requests or request rate at the reverse proxy is what keeps
   one client from holding the pool. A request size limit there, or a
   lower `server.max_body_size_bytes`, caps what one request can carry; a
   body also includes the conversation history and JSON, so it is a coarse
   bound.
2. **Set `pipeline.stage_timeout_ms` with `on_error = "block"`** on both
   ONNX stages, so that requests stuck behind long ones are rejected rather
   than kept waiting or let through.

The timeout applies to each stage, and it has to cover the longest input
you accept at your load, or that input is rejected too. A 16-window
request took 27 s in `onnx_injection` with all 4 sessions busy on long
input, on that machine (31 s with 1 thread); measure yours with the `onnx_concurrency` benchmark
(`GUARDRAIL_BENCH_WINDOWS`):

```toml
[pipeline]
stage_timeout_ms = 40000

[stages.onnx_injection]
on_error = "block"

[stages.toxicity]
on_error = "block"
```

More `intra_threads` also shortens a long request (16 windows in
`onnx_injection`: 21 s with 1 thread, 11.7 s with 2, 9.5 s with 4), at the
cost of throughput per core; see [`configuration.md`](configuration.md#onnx).

---

## Security properties

### Fail-open by default (`pipeline.on_error = "allow"`)

If a stage returns an error while handling a request (for example, ONNX
inference fails), the request continues as if that stage had allowed it, so
it reaches the upstream without that stage's check. This keeps a failing
model from taking down production traffic. Set `pipeline.on_error = "block"`
to fail closed for every stage, or set `on_error = "block"` on an individual
ONNX stage to fail closed only for that stage. Fail-closed errors are
returned as `403` with code `stage_error`.

Fail-open is only a defensible default if you can see it happening. Every
stage error increments
`guardrail_pipeline_errors_total{stage, outcome="allowed"|"blocked"}`, and
the request's audit record lists the stages that failed open in
`failed_open_stages`. [`contrib/prometheus-alerts.yml`](../contrib/prometheus-alerts.yml)
has an example alert that fires on any fail-open in the last 5 minutes.

### No raw content in logs

Audit records (both `tracing` events and NDJSON file) never contain message
content, API keys, or PII values. Only metadata (request ID, model, decision,
entity types found, latency) is logged. The `PiiRedactor` itself runs before
the audit event is emitted, so even the redacted text is not logged.

### Immutable pipeline per request

Each in-flight request holds a snapshot `Arc<Pipeline>` taken at request
arrival. Hot-reloads (SIGHUP) swap in a new pipeline atomically; in-flight
requests are never affected by configuration changes mid-flight.

### No outbound connections from classifiers

The regex and ONNX classifiers make no network calls — they are entirely
in-process. Only the `guardrail.upstream.forward` step makes an outbound
connection, and only to the configured `upstream_url`.

---

## Reporting vulnerabilities

See [`SECURITY.md`](../SECURITY.md) for the responsible disclosure process.
In particular, **bypass techniques for the injection or PII detectors** are
in-scope security reports even if the underlying patterns are publicly known,
because effective detection coverage is a security guarantee of this project.
