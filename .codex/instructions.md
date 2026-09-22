# RustShot - Codex Instructions

You are working on RustShot, a screenshot and screen recording utility written in Rust.

## Project Structure

- `src/main.rs` — entry point, CLI, tray event loop
- `src/capture.rs` — screenshot capture and file saving
- `src/selection.rs` — region selection overlay (multi-monitor, DPI-aware)
- `src/tray.rs` — system tray icon and menu
- `src/record.rs` — screen recording (Windows only, H.264/Media Foundation)
- `src/settings.rs` — JSON settings (manual parser, no serde)
- `benches/benchmarks/` — Criterion benchmark suite (`main.rs` plus one module per area)
- `tests/bench_functions/` — integration tests proving the benchmarked helpers stay public
- `benches/performance_gate.rs` — the stable, deterministic pull-request performance gate

## Rules

1. All code, comments, and commit messages must be in English.
2. Do not change behavior without a reason.
3. If you change behavior, add a test.
4. Do not do large refactoring for a small task.
5. Error handling uses `Result<_, String>` throughout.
6. Error messages must be human-readable.

## Verification Commands

Always verify your changes with:

```bash
cargo check          # Compilation
cargo lint           # Lint exactly as CI does (alias in .cargo/config.toml)
cargo fmt --check    # Formatting
cargo test           # All tests
```

`cargo lint` expands to `clippy --all-targets --all-features -- -D warnings`. Invoking
clippy directly is not equivalent: `Cargo.toml`'s `[lints]` table is all `warn`, so
without `-D warnings` every finding prints and the command still exits 0.

## Commit Convention

- `[+]` — additions
- `[-]` — deletions
- `[!]` — refactoring/fixes

## Testing

- Write small, focused unit tests.
- Test pure logic, not GUI.
- Extract logic into helper functions for testability.
- TDD workflow: write failing test -> minimal fix -> verify.
