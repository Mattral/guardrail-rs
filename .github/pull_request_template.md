## Summary

<!-- What does this PR change, and why? -->

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes (including doc tests)
- [ ] New public items have doc comments
- [ ] If adding an injection rule: positive + benign test cases added to `injection.rs`
- [ ] If adding a PII entity: schema, validation, and docs updated (see `CONTRIBUTING.md`)
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
