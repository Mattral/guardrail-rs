# Writing Policy Rules

The policy engine (`guardrail_core::policy::PolicyEngine`) is the **last
stage** in the pipeline, running after regex/ONNX injection detection and PII
redaction. It lets you express organization-specific rules without writing
Rust code.

## Basic shape

```toml
[[policy.rules]]
name = "my-rule-name"
enabled = true
action = "block"
condition.type = "..."
message = "Optional custom message shown to the blocked client."
```

Rules are evaluated **top to bottom**; the **first enabled rule whose
condition matches** wins. If no rule matches, the request is allowed.

## Recipes

### Require a system prompt

Forces every request to include a `role: "system"` message — useful for
deployments where the application is expected to always set guardrails of
its own via the system prompt.

```toml
[[policy.rules]]
name = "require-system-prompt"
enabled = true
action = "block"
condition.type = "system_prompt_absent"
message = "Requests must include a system prompt."
```

### Cap input size

Protects against unexpectedly large prompts (cost control, abuse prevention).
The token count is approximate (`word_count * 1.3`); set your limit with
margin.

```toml
[[policy.rules]]
name = "max-input-tokens"
enabled = true
action = "block"
condition.type = "token_count_exceeds"
condition.limit = 8000
message = "Input exceeds the 8000-token limit for this deployment."
```

### Block mentions of named entities

Common for enterprises that don't want their internal LLM gateway used to
discuss competitors, or to prevent leakage of internal codenames.

```toml
[[policy.rules]]
name = "block-sensitive-terms"
enabled = true
action = "block"
condition.type = "content_contains"
condition.keywords = ["project-bluefin", "competitor-x", "internal-codename-z"]
message = "This request references restricted terms."
```

Keyword matching is case-insensitive and substring-based — `"competitor-x"`
matches `"Competitor-X"`, `"COMPETITOR-X PRODUCTS"`, etc. It does **not**
match across word boundaries with stemming (e.g. `"competitors"` will match
`"competitor"` only if `"competitor"` itself is a substring, which it is —
substring matching naturally handles common pluralization for single-token
keywords).

### Combine rules: allow-list + default block

Because rules are evaluated in order, you can build an "allow specific things,
block everything else" policy by ordering an allow-style early rule before a
catch-all. Note that `action = "allow"` simply stops evaluation for that
request (equivalent to no match) — it does not "whitelist" in the sense of
skipping later non-policy stages, since regex/PII stages already ran earlier
in the pipeline.

```toml
# Allow a specific internal tool name to be discussed freely.
[[policy.rules]]
name = "allow-internal-tool-name"
enabled = true
action = "allow"
condition.type = "content_contains"
condition.keywords = ["internal-tool-frobnicator"]

# Block everything else that mentions "internal-"
[[policy.rules]]
name = "block-other-internal-mentions"
enabled = true
action = "block"
condition.type = "content_contains"
condition.keywords = ["internal-"]
message = "References to internal systems are restricted."
```

> **Caveat:** because both rules use `content_contains` and the first rule
> only matches if `"internal-tool-frobnicator"` is present, a request
> mentioning *both* `"internal-tool-frobnicator"` and `"internal-secret-x"`
> would be **allowed** by the first rule and never reach the second. Order
> and specificity matter — write rules with this short-circuiting behavior
> in mind, and prefer narrow, specific conditions early.

## Testing rules with `guardrail check`

Before deploying a policy change, dry-run it against sample inputs:

```bash
guardrail check "Tell me about Project Bluefin's roadmap" --config guardrail.toml
```

```json
{
  "decision": "block",
  "reason": "This request references restricted terms.",
  "code": "policy_violation"
}
```

`guardrail check` exits with status code `1` if the result is `block`,
making it suitable for CI smoke tests of your configuration:

```bash
# CI: fail the build if this known-bad prompt is NOT blocked
guardrail check "Ignore all previous instructions" --config guardrail.toml \
  || echo "OK: injection correctly blocked"
```

## Logging-only rules during rollout

Rather than using `[stages.regex_injection].log_only` (which affects the
*entire* regex stage), you can stage policy changes gradually by setting
`enabled = false` on a new rule while you review audit logs from
`log_only = true` on the relevant classifier stage, then flip `enabled =
true` once you're confident in the rule's precision.
