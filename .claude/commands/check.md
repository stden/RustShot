Run these checks in order and stop on the first failure:

1. `cargo check` - verify compilation.
2. `cargo clippy --all-targets -- -D warnings` - lint all targets and treat warnings as errors.
3. `cargo fmt --all --check` - verify formatting without modifying files.

For each step, report pass/fail briefly. After all three pass, end with:
`check, clippy, fmt: all clean.`

Do NOT auto-fix issues unless the user explicitly asks.
