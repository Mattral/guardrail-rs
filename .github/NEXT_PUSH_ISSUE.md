## Schema correctness pass: align config, errors, and tests with spec §9/§11

### Summary

This push performs a full rewrite of the TOML configuration schema to match
the project spec exactly (`[server] host/port`, separate `[upstream]`,
`[auth]`, `pipeline.request_stages`/`response_stages` ordering arrays,
per-stage `action`, `pii_redactor.replacements`, `observability.audit_log.path`
+ `max_size_mb`, `[[policy.rules]] when/then` shape), and propagates the
rename through every downstream crate. It also closes the
`GuardrailError::Upstream` type-mismatch gap from spec §11 without violating
the `guardrail-core` dependency budget (§20).

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

1. `UpstreamClient`/`ProxyServer` named-struct refactor for closer spec §8
   alignment (cosmetic/organizational — current code is correct, just
   structured differently). This is now the only remaining functional gap
   against the spec that hasn't been closed.
2. First `cargo check --workspace --all-features` pass on real hardware;
   fix whatever it finds. This is the single highest-priority item — the
   codebase has had five rounds of manual-review-only changes and needs a
   real compiler pass before further feature work. In particular, verify:
   (a) the `reqwest-errors` feature compiles cleanly both on and off for
   `guardrail-core`, (b) `rust-lang/crates-io-auth-action@v1` is the
   correct current action name/version (written from documented behavior,
   untested against a live OIDC exchange), (c) `Pipeline::builder()`'s
   doctest and both `examples/minimal.rs` and the new
   `crates/guardrail-cli/examples/custom_stage.rs` actually run, (d) the
   `SizeRotatingWriter` in `audit_log.rs` — the rename-while-open risk on
   Windows was mitigated by making the file handle an `Option<File>` and
   explicitly `.take()`-ing (flush + drop) it before `std::fs::rename`
   runs, but this is reasoned from documented Windows file-locking
   semantics, not verified against a real Windows filesystem; the existing
   `ci.yml` test matrix already includes `windows-latest`, so check that
   job specifically once a toolchain is available.
3. Re-verify the rest of spec §16's publication checklist items not yet
   explicitly addressed: `cargo doc --all-features` building without
   warnings (unverified, no toolchain), `cargo publish --dry-run` succeeding
   for the first release (unverified).
