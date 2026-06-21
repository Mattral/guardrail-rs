# Contributing to guardrail-rs

Thanks for your interest in improving `guardrail-rs`! This document covers
the practical steps for setting up a development environment, the project's
coding standards, and how to add the most commonly-requested extensions:
new injection rules and new PII entity types.

## Development setup

```bash
git clone https://github.com/Mattral/guardrail-rs.git
cd guardrail-rs
rustup component add rustfmt clippy
cargo build --workspace
cargo test --workspace
```

### Before opening a PR

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check          # license & advisory checks (requires cargo-deny)
```

All four must pass. CI enforces the same checks.

## Coding standards

- **Every public item must have a doc comment.** `#![deny(missing_docs)]`
  is enforced at the crate level for `guardrail-core`, `guardrail-classifiers`,
  and `guardrail-config`.
- **No `unwrap()`/`expect()` in non-test code** outside of documented,
  validated invariants (e.g. "this regex is validated by the test suite").
  If you must use one, add a comment explaining why it cannot panic.
- **Stages must not block the async executor.** CPU-heavy work (ONNX
  inference) must run inside `tokio::task::spawn_blocking`.
- **Stages should fail open.** Prefer returning `Ok(Decision::Allow)` with a
  `tracing::warn!` over propagating an error, unless the failure indicates
  the stage is fundamentally misconfigured.
- **New dependencies** must be checked against `deny.toml` (license
  allow-list). OpenSSL is banned; use `rustls`.

## Adding a new prompt-injection rule

Bundled rules live in
[`crates/guardrail-classifiers/src/rules/injection.rules`](crates/guardrail-classifiers/src/rules/injection.rules).

1. Add your pattern as a new line, with a `#`-comment above it explaining
   what attack class it targets.
2. Patterns are case-insensitive by convention — prefix with `(?i)`.
3. Add both a **positive** test case (in `injection.rs`'s
   `test_known_injections_blocked`) and confirm it doesn't false-positive
   against the **benign** test cases (`test_benign_inputs_allowed`).
4. Run the proptest suite to confirm no panics on arbitrary input:
   ```bash
   cargo test -p guardrail-classifiers
   ```
5. Benchmark the change if you're adding many patterns:
   ```bash
   cargo bench -p guardrail-classifiers -- regex_injection
   ```
   `RegexSet` matching is roughly linear in the number of patterns; the
   < 50 µs / 8 KB target should hold for rule sets up to a few hundred
   patterns.

## Adding a new PII entity type

1. Add a new variant to `PiiEntityType` in
   [`crates/guardrail-classifiers/src/pii.rs`](crates/guardrail-classifiers/src/pii.rs),
   with a `default_replacement()` arm (convention: `[UPPER_SNAKE_CASE]`).
2. Add the corresponding regex pattern to the `entity_pattern()` function.
3. Add the string identifier (e.g. `"passport_number"`) to:
   - `default_pii_entities()` in `guardrail-config/src/schema.rs` (only if
     it should be on by default — most new entities should NOT be, to avoid
     surprising false positives in existing deployments)
   - `VALID_ENTITIES` in `guardrail-config/src/validate.rs`
   - `PiiEntityList::from_strings()` in `guardrail-config/src/schema.rs`
4. Add test cases covering both true positives and near-miss false positives.
5. Update [`guardrail.example.toml`](guardrail.example.toml) and
   [`docs/configuration.md`](docs/configuration.md).

## Adding a new pipeline stage

New stages implement `guardrail_core::Stage`. See
[`docs/architecture.md`](docs/architecture.md) for the full contract. In
summary:

- `name()` returns a stable, lowercase, snake_case identifier used in logs
  and Prometheus labels.
- `evaluate()` must be cheap to call concurrently — put expensive
  initialization (model loading, regex compilation) behind `Arc` in the
  stage's constructor, not in `evaluate()`.
- Wire the stage into `guardrail_config::loader::build_pipeline()`, in the
  documented stage order (regex → semantic → PII → toxicity → policy).
- Add a corresponding section to `StagesConfig` in
  `guardrail-config/src/schema.rs`, with validation in `validate.rs`.

## Releasing

Releases are tag-triggered: pushing a `v*.*.*` tag runs the full pipeline
in `.github/workflows/release.yml` — verify → build cross-platform
binaries → build/push the Docker image → publish all 5 crates to
crates.io → create the GitHub Release. See that file's top-of-file comment
for the exact trigger gating (tag pushes get the full pipeline; pushes/PRs
to `main` only run `verify` and the `publish-dry-run` validation job).

### One-time setup: crates.io Trusted Publishing

Before the first tagged release, each of the 5 publishable crates must be
individually registered for [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing):

1. For each crate name (`guardrail-core`, `guardrail-classifiers`,
   `guardrail-config`, `guardrail-proxy`, `guardrail-cli`), go to
   `https://crates.io/crates/<name>/settings` and add a Trusted Publisher
   entry pointing at this repository and the
   `.github/workflows/release.yml` workflow file. **This is per-crate —
   there is no single repo-level toggle.**
2. **Bootstrap caveat:** crates.io requires a crate name to not yet exist
   on the registry before you can configure Trusted Publishing for it
   through the UI. If any of these 5 names haven't been published before,
   that first publish has to go out via the legacy `cargo login` +
   long-lived API-token flow, once, per crate — after that one-time
   bootstrap, Trusted Publishing takes over for every subsequent release
   and no token needs to be stored in the repo.
3. Publish in dependency order, since each crate's `Cargo.toml` declares a
   `version` requirement on its workspace siblings that the registry must
   be able to resolve: `guardrail-core` → `guardrail-classifiers` →
   `guardrail-config` → `guardrail-proxy` → `guardrail-cli`. The `publish`
   job in `release.yml` already does this in order, with a short wait
   between each step for the registry to finish indexing.
4. Before trusting a real tagged release, check the `publish-dry-run` job's
   results on `main` — note it's expected to fail for every crate except
   `guardrail-core` until step 2's bootstrap publish has happened for each
   crate's siblings; see the job's comments in `release.yml` for why.

### Version bumps

`[workspace.package].version` in the root `Cargo.toml` is the single
source of truth for each crate's own version, but — important — it does
**not** propagate into the internal `guardrail-* = { path = "..",
version = ".." }` dependency requirements inside each crate's own
`Cargo.toml` (Cargo has no `version.workspace = true` shorthand for
dependency requirements, only for a package's own version). After bumping
`[workspace.package].version`, also update every
`guardrail-* = { path = "..", version = "X.Y.Z" }` line under `crates/*/Cargo.toml`
to match — a CI job (`version-pin-check` in `ci.yml`) fails the build if
these drift out of sync, so this isn't easy to forget silently, but it is
still a manual two-step process rather than one.

## Reporting security issues

Please do **not** open a public GitHub issue for security vulnerabilities.
See [`SECURITY.md`](SECURITY.md) for the responsible disclosure process.

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
