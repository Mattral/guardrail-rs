# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

`guardrail-rs` sits on the request path between your application and your
LLM provider, and is explicitly designed to handle untrusted input
(prompt-injection payloads). We take security issues seriously.

**Please do not open a public GitHub issue for security vulnerabilities.**

Instead, report vulnerabilities privately via GitHub's
["Report a vulnerability"](https://github.com/Mattral/guardrail-rs/security/advisories/new)
feature on the Security tab of this repository. This creates a private
advisory visible only to maintainers.

Please include:

- A description of the vulnerability and its impact (e.g. "a crafted request
  body causes a panic in the X stage, resulting in denial of service").
- Steps to reproduce, ideally including a minimal request payload.
- The affected version(s) and build configuration (e.g. with/without the
  `onnx` feature).

## Response Process

1. We will acknowledge receipt within 5 business days.
2. We will investigate and aim to provide an initial assessment within
   10 business days.
3. If confirmed, we will work on a fix and coordinate a disclosure timeline
   with you. We credit reporters in the release notes unless you prefer
   to remain anonymous.

## Scope Notes

- **Regex denial-of-service (ReDoS):** all bundled regex patterns are
  compiled with the `regex` crate, which guarantees linear-time matching and
  is not susceptible to catastrophic backtracking. If you add custom rules
  via `custom_rules_path`, this guarantee still holds because `regex` never
  backtracks. Reports of pathological *performance* (not correctness) on
  custom rule sets are still welcome.
- **Bypass of detection stages:** if you find an input that should be
  blocked/redacted by a stage but isn't, this is a valid security report,
  especially for the bundled `injection.rules` and PII patterns.
- **Fail-open behavior** (`pipeline.on_error = "allow"`, the default) is a
  documented design tradeoff, not a vulnerability in itself — but if you
  find a way to *force* a stage error as a bypass technique, that is in
  scope.
