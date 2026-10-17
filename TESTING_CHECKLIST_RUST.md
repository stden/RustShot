# TESTING_CHECKLIST_RUST.md

## Before merge
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test`
- [ ] New business logic has unit tests
- [ ] Bug fixes include regression tests
- [ ] Integration tests cover DB / HTTP / queue boundaries where relevant
- [ ] No ignored flaky tests left behind
- [ ] Error paths are tested, not only happy paths

## Rust-specific best practices
- [ ] Small unit tests live near code with `#[cfg(test)]`
- [ ] Cross-module / black-box tests live in `tests/`
- [ ] Prefer deterministic assertions over log-based checks
- [ ] Avoid network access in unit tests
- [ ] Use temp dirs / fixtures for filesystem tests
- [ ] Use table-driven cases for parser / validator logic
- [ ] Avoid `unwrap()` in production paths unless justified
- [ ] Test serde / parsing round-trips when serialization matters

## CI minimum
- [ ] fmt
- [ ] clippy
- [ ] unit/integration tests
- [ ] release/debug build for critical crates
