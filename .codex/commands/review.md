Review the full RustShot codebase and report findings without making changes.

Steps:
1. Read the relevant Rust modules under `src/`, plus `tests/` and `benches/` when useful.
2. Run:
   - `cargo check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo fmt --all --check`
   - `cargo test`
3. Look for:
   - Logic bugs and behavioral regressions
   - Missing error handling
   - Potential panics in non-test code
   - Dead code or stale wiring
   - Security-sensitive mistakes or unchecked assumptions
   - Missing tests for risky paths
4. Report findings first, grouped by severity, with concrete file references when possible.
5. Do NOT make changes unless the user explicitly asks. Report only.
