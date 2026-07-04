## Schema correctness pass: align config, errors, and tests with spec §9/§11

> **⚠️ Notable fix in this push:** `Pipeline::run` previously could never
> actually return `Decision::Redact` to its caller — every successful
> redaction was collapsed to `Decision::Allow` by the time the pipeline
> returned, which (under the spec's own §6 code sample) made the audit
> log's redact case, the `redacted_total` metric, and `guardrail check`'s
> redact output all dead code. **This matches the spec's own illustrative
> `Pipeline::run` example exactly** — so this is flagged as resolving a
> tension between §6's simplified code sample and §10's audit-log
> requirements, not as an unambiguous spec bug. See "Critical fix" below
> for the full reasoning and an explicit note on how to revert if that
> reasoning turns out to be wrong.

> **🏗️ Also in this push:** `guardrail-proxy/src/server.rs` (1135 lines,
> mixing listener lifecycle, request routing, auth, response building, and
> error mapping into one file) was decomposed into 5 single-responsibility
> modules (`state`, `auth`, `error`, `handler`, `server`), following the
> layered-service pattern used by production Rust HTTP services. No file
> in the crate now exceeds ~520 lines. The auth-check logic in particular
> went from only having HTTP-round-trip test coverage to also having fast,
> dependency-free unit tests on the underlying decision function. See
> "Repository hygiene" / the `### Changed` section in `CHANGELOG.md` for
> the full breakdown.

### Summary

This push performs a full rewrite of the TOML configuration schema to match
the project spec exactly (`[server] host/port`, separate `[upstream]`,
`[auth]`, `pipeline.request_stages`/`response_stages` ordering arrays,
per-stage `action`, `pii_redactor.replacements`, `observability.audit_log.path`
+ `max_size_mb`, `[[policy.rules]] when/then` shape), and propagates the
rename through every downstream crate. It also closes the
`GuardrailError::Upstream` type-mismatch gap from spec §11 without violating
the `guardrail-core` dependency budget (§20).

### Critical fix: `Decision::Redact` was unreachable from `Pipeline::run` — resolving a tension between spec §6 and spec §10

**Severity: high.** This is a functional correctness fix in the core
request-handling path, not a tooling/deployment issue like the rest of
this document. Found during a line-by-line re-read of `server.rs` end to
end (specifically: a comment that said `// populated below` next to a
`Vec::new()` that nothing ever populated — pulling that thread led to the
root cause one layer down in `guardrail-core`).

**Important nuance, checked directly against the spec before "fixing"
anything:** the spec's own §6 reference implementation of `Pipeline::run`
(the illustrative code sample, not necessarily meant to be copied
verbatim) has the identical shape — its loop applies a `Redact`'s
`mutated` request and continues, then **unconditionally** returns
`Ok((Decision::Allow, req))` after the loop, with no path back to
`Decision::Redact` at all. So this isn't a typo or oversight unique to
this implementation; the question is whether that §6 code sample is
authoritative, or whether it's a simplified teaching example that
undersells what §10 actually requires.

**Why I concluded §10 wins:** §10's own audit-log JSON example includes a
field literally named `pii_entities_found`, and the OTel trace spec
explicitly calls for a per-stage `entities_found` attribute on the
`guardrail.stage.pii_redactor` span. A field named `pii_entities_found`
that can — by construction, under the §6 code sample — *never be
non-empty for any record whose `decision` field made it out of
`Pipeline::run`* is hard to read as intentional; the far more natural
reading is that §10 expects redaction outcomes to be observable in the
audit trail, and the §6 code sample is just simplified illustrative code
that didn't carry that requirement through. I'm flagging this reasoning
explicitly rather than silently diverging from a literal spec code
sample, since "the spec's example code does X" is normally a strong
signal I'd defer to.

**The fix:**
1. `Decision::Redact` gained a new `entities: Vec<String>` field (mirroring
   `Block`'s `reason`+`code` pairing — this part is a clean, additive
   extension of the spec's `Decision` enum, not a contradiction of it),
   since `PiiRedactor` was already computing a structured entity-type list
   internally and discarding it before it could reach the audit log.
2. `Pipeline::run_with_observer` now accumulates every redacting stage's
   `reason` (joined with `"; "`) and `entities` (de-duplicated union)
   across the whole loop, and returns `Decision::Redact` — not `Allow` —
   as the final decision whenever at least one stage redacted. This is
   the one place this implementation deliberately diverges from §6's
   literal code sample, in favor of what §10 appears to require.
3. Propagated the new `entities` field through all 9 files in the
   workspace that construct or destructure `Decision::Redact` (verified
   via exhaustive `grep -rn "Decision::Redact {"` / `"Redact {"` across
   every `.rs` file, not just the ones touched directly).
4. `AuditRecord::from_decision` simplified from a 5-argument to a
   4-argument function: it now reads `entities` directly off the
   `Decision::Redact` it's given instead of taking a separately-threaded
   `pii_entities: &[String]` parameter — the exact pattern that let the
   original `server.rs` bug exist (a parameter that's trivially easy to
   forget to populate, vs. data that lives on the type itself and can't
   be forgotten).
5. Added `test_helpers::RedactingStage` (configurable reason/entities, with
   a `with_name` variant for composing multiple redacting stages
   distinguishably) and 6 new tests in `pipeline.rs`: single redacting
   stage returns `Redact` not `Allow`; multiple stages accumulate reasons
   and entities; duplicate entities across stages are de-duplicated;
   redact-then-block returns `Block` (blocking always wins — it's the
   stronger guarantee, matching §6's stated short-circuit-on-Block
   behavior); the mutated request correctly flows through to later
   stages. `Pipeline::run`'s rustdoc gained a full worked doctest
   exercising the fixed behavior and explicitly documenting it as a
   deliberate divergence from the simplified §6 code sample.

**If this reasoning is wrong** (i.e. if `Decision::Redact` reaching the
caller was never intended, and `pii_entities_found` is meant to be sourced
some other way — e.g. purely from per-stage OTel span attributes rather
than the final aggregated `Decision`) — this is a one-line revert in
`pipeline.rs` back to the unconditional `Ok((Decision::Allow, req))`, with
the `entities` field on `Decision::Redact` and all its plumbing left in
place as harmless, unused-by-the-final-decision metadata. Flagging this
explicitly so a human reviewer can make the final call with full context
rather than this decision being buried in a diff.

### Changes

**`guardrail-config` (breaking schema rewrite)**
- [x] `schema.rs` rewritten: `ServerConfig{host,port,workers,max_body_size_bytes}`
      + `.listen_addr()` helper, `UpstreamConfig{url,timeout_secs,connect_timeout}`,
      `AuthConfig{require_key,keys}`, `PipelineConfig{request_stages,response_stages,on_error}`,
      `StagesConfig` with per-stage `action: StageAction`, `PiiReplacements`
      custom tokens, `PolicyRuleConfig{name,enabled,when,then}` using the
      spec's `when`/`then` TOML nesting, `AuditLogConfig{enabled,path,max_size_mb}`.
- [x] `validate.rs` rewritten with full coverage of the new shape (unknown
      stage IDs, unknown PII entity types, auth key requirements, ONNX model
      path existence, policy rule "no condition" detection, otlp scheme
      validation, audit log path/size checks).
- [x] `loader.rs`: `build_pipeline` now iterates `pipeline.request_stages` in
      order and dispatches by stage-id string match; `convert_policy_rules`
      maps the new `when`/`then` shape to `guardrail_core::policy` types;
      env-var overlay (`GUARDRAIL_UPSTREAM`, `GUARDRAIL_PORT`,
      `GUARDRAIL_LOG_LEVEL`, `GUARDRAIL_OTLP_ENDPOINT`) updated for new field
      paths; removed ~250 lines of orphaned duplicate test code left over
      from an earlier incomplete rewrite.
- [x] Fixed a corrupted doctest in `build_response_redactor` (stray
      sed-mangled line breaking the TOML literal).
- [x] Removed dead imports (`PolicyActionConfig`, `PolicyConditionConfig` at
      crate level) and a redundant shadowed `use` inside `convert_policy_rules`.

**`guardrail-core` (error model)**
- [x] `GuardrailError::Upstream` is now `#[from] reqwest::Error` when the new
      `reqwest-errors` feature is enabled, matching spec §11 exactly, while
      staying `Upstream(String)` by default so `guardrail-core` does not pull
      in `reqwest` for consumers who only need the core types (preserves the
      §20 dependency budget). `guardrail-proxy` enables the feature.
- [x] Added `GuardrailError::upstream(msg)` constructor that works
      identically regardless of which feature configuration is active.

**`guardrail-proxy`**
- [x] `forward.rs`: `forward_request`/`read_body` now use
      `.map_err(GuardrailError::from)` instead of manual string formatting,
      taking advantage of the new `#[from] reqwest::Error` impl.
- [x] `server.rs`: `classify_upstream_error` rewritten to inspect the
      structured `reqwest::Error` (`.is_timeout()` / `.is_connect()`) instead
      of matching on `Display` text — more reliable across locales/versions.
      Updated its tests to trigger *real* `reqwest` errors (non-routable
      address, connection-refused on port 1) rather than constructing fake
      string-based errors that no longer type-check.
- [x] Fixed remaining stale field references in `run_server` /
      `forward_to_upstream` (`config.server.listen_addr` →
      `config.server.listen_addr()`, `server.upstream_url` → `upstream.url`,
      `server.upstream_timeout_secs` → `upstream.timeout_secs`, added
      `connect_timeout` wiring into the `reqwest::Client` builder).
- [x] Rewrote `audit_log.rs` to use the new `AuditLogConfig{path,max_size_mb}`
      shape (previously `directory`/`file_name_prefix`/`rotation`); rotation
      is currently `tracing_appender::rolling::never` against the configured
      path — **true size-based rotation honoring `max_size_mb` is not yet
      implemented** (see Known Gaps below).
- [x] Bulk-fixed test TOML literals in `server.rs` test module
      (`[stages.pii_redaction]` → `[stages.pii_redactor]`, old
      `listen_addr`/`upstream_url` → `host`/`port`/`[upstream]`).

**`guardrail-cli`**
- [x] `commands.rs` `validate` output and test fixtures updated for the new
      field names and `auth`/`audit_log` reporting lines.

**`guardrail-test-suite`**
- [x] Integration test TOML literals bulk-fixed to the new schema shape.

**Workspace**
- [x] `edition` deliberately kept at `2021` (not `2024` as the spec
      requests) with an explicit comment explaining why: no Rust toolchain
      is available in this environment to verify the 2024-edition migration
      (RPITIT capture rules, `if let` temporary scoping, reserved `gen`
      keyword), and bumping blind on a ~9k-line workspace is not an
      acceptable risk. Flagged as the recommended first step once a
      toolchain is available.

**Repository hygiene (found by direct user report, not the spec re-read)**
- [x] Removed a literal directory named `{crates` (with nested
      literal-brace subdirectory names like
      `{guardrail-core/src,guardrail-classifiers/src,...}`), a relic from
      the project's very first scaffolding command — an
      `mkdir -p {crates/{...},...}` brace-expansion pattern run under a
      shell that doesn't perform brace expansion, so it created one
      literal nested path instead of expanding into ~15 real directories.
      Zero files anywhere in the tree.
- [x] Removed an orphaned empty `tests/integration/` directory from the
      same original scaffolding; the real integration tests live at
      `crates/guardrail-test-suite/tests/proxy_e2e.rs`.
- [x] Added `.dockerignore` (had never existed) — every `docker build`
      was tarring up and transmitting the entire repo, including
      `target/`, to the daemon before the `Dockerfile` got a chance to
      ignore anything. Verified every exclusion pattern against every
      `COPY` instruction in the `Dockerfile` to confirm nothing actually
      needed by the build was excluded.
- [x] Added `guardrail-audit.ndjson*` / `audit-logs/` to `.gitignore` —
      the default audit-log path writes to the project root, and its
      rotated backups would otherwise be one `git add .` away from being
      committed.
- [x] **Consolidated `CHANGELOG.md`**: discovered, while doing this
      cleanup, that the file had accumulated six separate, never-merged
      `### Added`/`### Changed`/`### Fixed` blocks (and six duplicate
      `[Unreleased]: ...compare...` link-reference lines) across earlier
      sessions, including substantial duplicate prose describing the same
      features twice (PII response redaction, NDJSON audit log, SIGHUP
      hot-reload, Prometheus metrics, and config-schema changes were each
      described in two different places). Fully rewritten into one clean
      `## [Unreleased]` section with each heading appearing exactly once,
      content deduplicated and organized by spec section, one link
      reference at the true end of file. Spot-checked topic coverage
      against the pre-rewrite version to confirm no real content was lost.

### Known gaps carried into this push (not yet done)

- [x] ~~`tests/fixtures/` directory~~ — **done**: `clean_prompts.json`,
      `injection_prompts.json`, `pii_prompts.json`, `policy_cases.json`, plus
      a `README.md` explaining shape and usage.
- [x] ~~`[auth]` enforcement~~ — **done**: `server.rs` now checks
      `X-Guardrail-Key` against `config.auth.keys` before the body is even
      read, exempts `/healthz`/`/metrics`, strips the header before
      forwarding upstream, and has 7 new tests covering missing-key,
      wrong-key, correct-key, exemption, and default-disabled paths.
- [x] ~~`GuardrailError::Upstream` type~~ — **done**: now
      `#[from] reqwest::Error` behind an optional `reqwest-errors` feature
      (on by default for `guardrail-proxy`), preserving `guardrail-core`'s
      dependency budget for consumers who don't need it.
- [x] ~~`docs/configuration.md`, `docs/policy-rules.md`,
      `guardrail.example.toml`~~ — **done**: all three fully rewritten to
      the new schema (`[auth]`, `[upstream]`, `when`/`then` policy shape,
      `pii_redactor.replacements`, `audit_log.path`/`max_size_mb`).
- [x] ~~Ollama in `docker-compose.yml`~~ — **done**, plus a Grafana service
      wired to the existing dashboard JSON.
- [x] ~~`benches/pipeline.rs` (spec §13)~~ — **done**, but relocated to
      `crates/guardrail-test-suite/benches/pipeline.rs` since the workspace
      root is virtual (no `[package]`, so no crate to attach a `[[bench]]`
      target to). Includes `bench_regex_stage`,
      `bench_full_pipeline_regex_only` (matches the spec's example almost
      verbatim), plus two additional cases: input-size scaling
      (`bench_full_pipeline_by_size`) and the blocked-short-circuit path.
      Wired into a new CI job (`pipeline-latency-gate`) that **hard-fails**
      if any case exceeds 5ms, separate from the soft 150%-regression alert
      on classifier microbenchmarks.
- [x] ~~`crates/guardrail-cli/examples/minimal.rs` (spec §14)~~ — **done**:
      demonstrates embedding the pipeline directly as a library with zero
      HTTP/network usage, distinct from `examples/minimal.py` (which talks
      to a *running proxy* over HTTP). Added `Pipeline::builder()` as a
      convenience constructor in `guardrail-core` to match the spec's
      example code exactly.
- [x] ~~crates.io OIDC publish workflow correctness~~ — **done, and found a
      real bug while auditing**: the previous workflow claimed "Trusted
      Publishing" in its comments/permissions but actually used a
      long-lived `CARGO_REGISTRY_TOKEN` secret via `cargo publish
      --no-verify` — the opposite of what Trusted Publishing means. Fixed
      to use `rust-lang/crates-io-auth-action@v1` to mint a short-lived
      OIDC-derived token per run, scoped `id-token: write` only on the
      `publish` job (removed from the workflow-level `permissions:` block
      for least privilege), and removed `--no-verify` since the `verify`
      job already gates `build-binaries`/`docker`/`publish` via `needs:`.
- [x] ~~Internal path dependencies missing `version`~~ — **found and fixed,
      not originally on this list**: every internal
      `guardrail-* = { path = "..", ... }` dependency across all 5
      publishable crates (14 occurrences) was missing the `version = "..."`
      field that crates.io *requires* for path dependencies at publish
      time. This would have made every `cargo publish` step in the release
      workflow fail on the very first real release attempt. Added a
      CI job (`version-pin-check`) that fails the build if any of these 14
      version pins drifts from `[workspace.package].version`, since Cargo
      has no `version.workspace = true` shorthand for dependency
      requirements (only for the package's own `[package].version`) — this
      is a manual sync point that needed an automated guard, not just a
      comment.
- [x] ~~True size-based rotation in `audit_log.rs`~~ — **done**: replaced
      `tracing_appender::rolling::never` with a custom `SizeRotatingWriter`
      (`Write` impl wrapping a `Mutex<SizeRotatingState>`) that checks the
      configured `max_size_mb` before each write and renames-and-reopens on
      threshold, preserving old content in a `<path>.<unix_timestamp>`
      backup file. The file handle field is `Option<File>` specifically so
      `rotate()` can `.take()` (flush + drop) the old handle *before* the
      `std::fs::rename` call — renaming a file that's still open under a
      held handle is fine on POSIX but can fail with access-denied on
      Windows, since `std::fs::File` doesn't request `FILE_SHARE_DELETE` by
      default. 6 new unit tests cover: no rotation under threshold,
      rotation on exceeding it, old content preserved in the backup, never
      rotating an empty file (a single oversized record doesn't trigger a
      pointless rotation), resuming the correct size on restart against a
      pre-existing file, and `max_size_mb = 0` not panicking. Updated the
      stale "known limitation" callouts in `docs/configuration.md` and
      `guardrail.example.toml` accordingly.
- [x] ~~`GUARDRAIL_CONFIG` env var~~ — **done, and found while re-reading
      the spec**: §14's example (`GUARDRAIL_CONFIG=examples/minimal.toml
      cargo run -p guardrail-cli`) was never actually implemented — only
      documented as a caveat in a loader.rs comment. Added
      `env = "GUARDRAIL_CONFIG"` to the `--config` flag on all three
      subcommands (`run`, `validate`, `check`) via clap's `env` feature
      (added to the workspace `clap` dependency). Flag still wins over the
      env var if both are given; env var wins over the hardcoded
      `guardrail.toml` default if no flag is given. Tests consolidated into
      one function for the same env-var-parallel-test-race reason described
      below.
- [x] ~~`examples/python_client.py` naming~~ — **done, and found while
      re-reading the spec**: §14 names this file literally
      `examples/python_client.py`; it had been built as `examples/minimal.py`
      in an earlier session. Renamed and fixed all internal/cross-file
      references (`justfile`, the file's own docstring, `examples/README.md`).
- [x] ~~`crates/guardrail-cli/examples/custom_stage.rs` location~~ — **done,
      and found while re-reading the spec**: §14's directory tree lists
      `custom_stage.rs` alongside `minimal.rs` under
      `crates/guardrail-cli/examples/`; it had been built under
      `crates/guardrail-classifiers/examples/` instead. Moved (costs
      nothing — `guardrail-cli` already depends on both `guardrail-core`
      and `guardrail-classifiers` directly for its `check` subcommand) and
      fixed all references (`justfile`, `examples/README.md`,
      `docs/stage-api.md`).
- [x] ~~Crate-level doc links to config/threat-model/changelog (spec
      §17 item 4)~~ — **done, and found while re-reading the spec**: none
      of the four library crates (`guardrail-core`, `guardrail-classifiers`,
      `guardrail-config`, `guardrail-proxy`) had the explicit "Further
      reading" links the spec requires, despite otherwise meeting the
      other three §17 requirements (description, working example, feature
      flags table). `guardrail-classifiers` and `guardrail-config` were
      also missing a feature-flags table and/or a working doctest example
      entirely — added both. `guardrail-proxy` was additionally missing
      `#![deny(missing_docs)]` (present on the other three crates) — added
      it after a heuristic scan found no undocumented public items that
      would break the build under the new lint.
- [x] ~~Test-isolation race condition in `cli.rs`'s new `GUARDRAIL_CONFIG`
      tests~~ — **found and fixed during this same round**: initially wrote
      5 separate `#[test]` functions each mutating the process-global
      `GUARDRAIL_CONFIG` env var; since `cargo test`/`nextest` run different
      test functions in parallel by default, this would have been flaky.
      Consolidated into one sequential test function. The same pre-existing
      pattern was found in `guardrail-config/src/loader.rs`'s 5 separate
      env-override tests (added in an earlier session) and fixed the same
      way.
- [ ] `UpstreamClient` / `ProxyServer` named structs (spec §8) still don't
      exist as distinct types — functionality is correct but organized as
      `AppState` + free functions instead.
- [ ] `edition = "2024"` bump deliberately deferred — see workspace
      `Cargo.toml` comment for rationale (no toolchain available to verify).

### Compile-correctness caveat

No `cargo`/`rustc` toolchain is available in the sandbox this was built in
(`network: false`, no `~/.cargo/bin`). Every change above was verified by
careful manual review — import correctness, trait bounds, signature
matching, borrow-checker reasoning, doctest validity — but **none of this
has been run through an actual compiler**. The very first action on a
machine with a toolchain should be `cargo check --workspace --all-features`
followed by `cargo test --workspace`, before trusting this as a working
build.

### Suggested next push

**Two items below are confirmed decisions with owner sign-off (2026-06-21,
from Min), not open questions — listed first since they're settled and
actionable, unlike the rest of this list.**

1. **[CONFIRMED] Public API surface widening in `guardrail-proxy` —
   treat as a load-bearing semver commitment.** `auth::is_authorized`,
   `error::error_response`, `error::error_body_response`,
   `error::internal_error_response`, and `error::classify_upstream_error`
   are now `pub` (previously private to the old monolithic `server.rs`,
   unreachable outside the crate). The project owner has explicitly
   signed off on this as the intended design — these are well-documented,
   independently tested, reusable primitives for anyone embedding
   `guardrail-proxy`, in the spirit of how `tower`/`axum`/`hyper` expose
   composable pieces rather than one opaque entry point. **Any future
   change to these five signatures is a breaking change requiring a major
   version bump, the same as any other stable public item in this crate.**
   If a future decision reverses this (these five become `pub(crate)`
   again), that reversal must happen **before** any 1.0 release — once
   1.0 ships, removing `pub` from an item is itself a breaking change, so
   "make it private later" stops being free after that point. `handler`
   itself correctly stayed `pub(crate)` throughout (none of its functions
   are reachable externally, by design). Still worth running
   `cargo public-api diff` (or equivalent) once a toolchain exists, purely
   as a mechanical completeness check against manual review.

2. **[CONFIRMED, ACTION REQUIRED FROM REPO OWNER] crates.io Trusted
   Publishing setup — outside what an agent in this sandbox can do.**
   Each of the 5 publishable crates (`guardrail-core`,
   `guardrail-classifiers`, `guardrail-config`, `guardrail-proxy`,
   `guardrail-cli`) must be **individually** registered for Trusted
   Publishing in its own crates.io package settings — this links the
   package to this GitHub repository plus the specific workflow file
   (`.github/workflows/release.yml`) authorized to mint OIDC tokens on
   its behalf. Registration is per-crate, not once for the whole repo.
   The workflow already calls `rust-lang/crates-io-auth-action@v1`
   correctly, but that call will fail at runtime until this registration
   exists for every crate.
   - **Bootstrap caveat:** crates.io's Trusted Publishing UI, for a
     brand-new crate name that has never been published before, requires
     the crate to not yet exist on the registry at setup time. If any of
     these 5 names need a first-ever manual publish to "claim" them, that
     one-time bootstrap publish needs the legacy `cargo login` +
     long-lived API-token flow once — after that, Trusted Publishing
     takes over for every subsequent release.
   - **Validation step before trusting a real release:** a `publish-dry-run`
     CI job has been added to `.github/workflows/release.yml`, running on
     every push/PR to `main` (not just tagged releases) via
     `cargo publish -p <crate> --dry-run` for all 5 crates. **Important:**
     this job is expected to fail for every crate except `guardrail-core`
     until `guardrail-core` (and each crate's other unpublished siblings)
     has actually been published at least once — `cargo publish --dry-run`
     resolves path-dependencies via their `version` requirement against
     the live registry, the same way a real publish does, so it cannot
     succeed for a crate whose sibling dependencies don't exist on
     crates.io yet. The job is marked `continue-on-error: true` for
     exactly this reason; treat a passing `guardrail-core` dry-run plus
     failing dry-runs for the other 4 as the *expected, healthy* signal
     before the first release, not as something to debug. After the first
     real release, all 5 should pass and a failure becomes meaningful again.
   - This is implemented as a matrix job (parallel, `fail-fast: false`) so
     every crate's dry-run result is visible in one CI run rather than
     stopping at the first failure — useful for confirming the *expected*
     failure pattern above (only `guardrail-core` passing) actually
     matches reality, rather than masking a real, unrelated manifest bug
     in one of the other 4 crates behind the "expected to fail" assumption.

---

**Everything below this line is still open / unconfirmed — normal
priority-ordered follow-up work, not owner-confirmed decisions.**

3. `UpstreamClient`/`ProxyServer` named-struct refactor for closer spec §8
   *literal naming* alignment — note this push's `server.rs` decomposition
   (`state`/`auth`/`error`/`handler`/`server`) already delivers the
   underlying separation-of-concerns spec §8 is really asking for, just
   under different module/type names than the spec's literal
   `UpstreamClient`/`ProxyServer`. Worth a judgment call on whether
   renaming to match the spec's exact nouns adds real value at this point
   or is just nominal alignment — the current names (`handler`, `auth`,
   `error`, `state`) arguably read more clearly for a Rust audience than
   `ProxyServer` would.
4. Watch `guardrail-config/src/loader.rs` (733 lines) and
   `guardrail-config/src/schema.rs` (665 lines) — not large enough to
   force a split today, but if either keeps growing, the same
   single-responsibility decomposition applied to `guardrail-proxy` this
   push (e.g. splitting `loader.rs`'s TOML-loading, env-var-overlay, and
   pipeline-building responsibilities into separate files) would pay off
   the same way. Not urgent; flagging so it doesn't sneak past 1000+ lines
   unnoticed the way `server.rs` did.
5. First `cargo check --workspace --all-features` pass on real hardware;
   fix whatever it finds. This is the single highest-priority *technical*
   item — the codebase has had six rounds of manual-review-only changes
   and needs a real compiler pass before further feature work. In
   particular, verify:
   (a) the `reqwest-errors` feature compiles cleanly both on and off for
   `guardrail-core`,
   (b) `rust-lang/crates-io-auth-action@v1` is the correct current action
   name/version (written from documented behavior, untested against a
   live OIDC exchange — this becomes directly testable once item 2 above
   is done),
   (c) `Pipeline::builder()`'s doctest and both `examples/minimal.rs` and
   `crates/guardrail-cli/examples/custom_stage.rs` actually run,
   (d) the `SizeRotatingWriter` in `audit_log.rs` — the rename-while-open
   risk on Windows was mitigated by making the file handle an
   `Option<File>` and explicitly `.take()`-ing (flush + drop) it before
   `std::fs::rename` runs, but this is reasoned from documented Windows
   file-locking semantics, not verified against a real Windows
   filesystem; the existing `ci.yml` test matrix already includes
   `windows-latest`, so check that job specifically once a toolchain is
   available.
6. Re-verify the rest of spec §16's publication checklist items not yet
   explicitly addressed: `cargo doc --all-features` building without
   warnings (unverified, no toolchain), `cargo publish --dry-run`
   succeeding for the first release (now directly actionable as part of
   item 2 above).
