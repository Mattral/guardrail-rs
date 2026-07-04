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
- `redact_responses = true` (opt-in) — post-processes non-streaming JSON
  responses to redact PII the model may have echoed back from retrieved
  documents or training data.

**Residual risk:**

- Regex-only approach cannot detect names, addresses, or organization names
  (NER-based detection is planned via `onnx-pii` feature).
- Response redaction does not apply to streaming (`stream: true`) responses.
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

**Residual risk:** this is a simple shared-secret check, not OAuth/JWT/mTLS,
and key comparison is not constant-time — don't rely on it against a
timing-attack-capable adversary on the same network segment. There is no key
revocation list or expiry; rotating a compromised key requires editing
`auth.keys` and reloading (SIGHUP) or restarting. For internet-facing
deployments, pair this with a proper API gateway or WAF.

## Out of scope: what guardrail-rs does NOT protect against

### Response-side injection / indirect prompt injection

If the upstream LLM processes external content (retrieved documents, web
results, tool outputs) that contains injection payloads, guardrail-rs does
not inspect those payloads — only the **incoming request** messages are
scanned. Mitigate with careful tool design, `redact_responses = true` for
PII, and output validation at the application layer.

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

The `max_body_size_bytes` and `upstream_timeout_secs` settings provide basic
limits, but guardrail-rs is not designed to defend against sophisticated
volumetric DoS attacks. A reverse proxy or WAF in front of guardrail-rs is
recommended in public deployments.

---

## Security properties

### Fail-open by default (`pipeline.on_error = "allow"`)

If a stage crashes or returns an error, the request is allowed through rather
than blocked. This ensures that a broken ONNX model or transient regex engine
issue does not take down production traffic. Set `pipeline.on_error = "block"`
for high-security deployments where fail-closed behavior is preferred.

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
