---
description: Verify code health without changing files - cargo check, then cargo lint (as CI lints), then cargo fmt --all --check.
---

Run these checks in order and stop on the first failure:

1. `cargo check` - verify compilation.
2. `cargo lint` - lint exactly as CI does (the alias in `.cargo/config.toml` for `clippy --all-targets --all-features -- -D warnings`). Use the alias, not re-typed flags: it is the single source of truth and `tests/lint_policy.rs` guards it.
3. `cargo fmt --all --check` - verify formatting without modifying files.

For each step, report pass/fail briefly. After all three pass, end with:
`check, clippy, fmt: all clean.`

Do NOT auto-fix issues unless the user explicitly asks.
