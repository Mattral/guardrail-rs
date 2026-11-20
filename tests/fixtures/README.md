# Test Fixtures

Sample request payloads used across the test suite and for manual smoke
testing with `guardrail check`.

| File | Purpose |
|------|---------|
| `clean_prompts.json` | Benign requests, labelled `allow`. |
| `injection_prompts.json` | Prompt-injection examples, labelled `block`. Not all of them are caught by the bundled rules; [`eval/RESULTS.md`](../../eval/RESULTS.md) lists the results. |
| `pii_prompts.json` | Requests containing PII, with expected entity types and redacted text. |
| `policy_cases.json` | Request + policy-rule pairs exercising every `PolicyCondition` variant. |

## Shape

Each file is a JSON object with a `description` and a `cases` array. Every
case has at minimum:

- `name` — a short, stable identifier (used in test names).
- `messages` — an array of `{role, content}` objects, directly usable as the
  `messages` field of an OpenAI-style chat completion request.
- `expected_decision` — one of `"allow"`, `"redact"`, `"block"`.

Depending on the file, additional fields appear: `expected_code` (for
blocks), `expected_entities` (for redactions), `policy_rule` (for policy
cases), and an optional `note` explaining any subtlety.

## Using these in tests

`injection_prompts.json` and `clean_prompts.json` are loaded by the
detection evaluation (`crates/guardrail-test-suite/tests/detection_eval.rs`,
see [`eval/`](../../eval/README.md)). The other files are reference data:
the unit and integration tests construct `GuardrailRequest`/TOML literals
inline for locality of reference. They are provided so that:

1. New tests can be written by copying a case's `messages` array directly.
2. The expected behavior of every bundled rule and entity type is documented
   in one place, independent of the Rust source.
3. `guardrail check` can be smoke-tested manually against a known-good set
   of inputs after a rule-set change:

   ```bash
   # Example: re-validate every injection fixture still blocks after editing
   # crates/guardrail-classifiers/src/rules/injection.rules
   python3 -c "
   import json, subprocess
   data = json.load(open('tests/fixtures/injection_prompts.json'))
   for case in data['cases']:
       text = case['messages'][-1]['content']
       result = subprocess.run(
           ['cargo', 'run', '-p', 'guardrail-cli', '--', 'check', text,
            '--config', 'guardrail.toml'],
           capture_output=True, text=True,
       )
       status = 'OK' if 'block' in result.stdout else 'FAIL'
       print(f'{status}: {case[\"name\"]}')
   "
   ```

A future improvement could add a `#[rstest]`-based loader that deserializes
these files directly and parameterizes the existing test functions —
tracked as follow-up work.
