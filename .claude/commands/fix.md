Auto-fix formatting and lint issues, then verify the project still passes checks.

Run these steps in order:

1. `cargo fmt --all`
2. `cargo clippy --fix --all-targets --allow-dirty --allow-staged -- -D warnings`
3. `cargo check`
4. `cargo clippy --all-targets -- -D warnings`
5. `cargo test`

Report what changed. If a step fails or a lint cannot be auto-fixed cleanly, show the
blocking output and stop.
