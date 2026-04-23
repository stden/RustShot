# RustShot - AI Agent Instructions

> Compatible with OpenAI Codex, Cursor, GitHub Copilot, and other AI coding assistants.
> Claude Code-specific instructions are in `CLAUDE.md`.

## Project Overview

`RustShot` is a screenshot utility for all platforms and a screen recording utility (currently **Windows only**).

## Key Files

- `src/main.rs` — entry point, CLI parsing, tray event loop
- `src/capture.rs` — screenshot capture and file saving
- `src/selection.rs` — interactive region selection overlay (multi-monitor, DPI-aware)
- `src/tray.rs` — system tray icon and context menu
- `src/record.rs` — screen recording to MP4 (Windows only, H.264 via Media Foundation)
- `src/settings.rs` — JSON settings persistence (manual parser, no serde)

## Language

All descriptions, comments, commit messages, code reviews, and development discussions must be in English.

## Core Rules

- Do not change behavior without a reason.
- Do not do large refactoring for a small task.
- Extract complex logic into small testable functions.
- If you change behavior, add a test.
- The project uses `Result<_, String>` for error handling throughout.

## Commands

```bash
cargo check          # Verify compilation
cargo test           # Run all tests
cargo fmt            # Format code
cargo clippy         # Lint
cargo build --release # Release build
cargo bench          # Run benchmarks
```

## Development Workflow

1. Write a test first.
2. Verify it fails.
3. Make the minimal fix.
4. Verify the test passes.
5. Run all tests with `cargo test`.

## Testing Guidelines

- Prefer small, focused unit tests.
- Test pure logic functions, not GUI interactions.
- For CLI, capture, selection, and tray: extract logic into helper functions and test those.
- If a bug is fixed, add a regression test.

## Commit Convention

- `[+]` — additions (new features, new tests)
- `[-]` — deletions (removed features, removed code)
- `[!]` — refactoring, fixes, improvements

## Dependencies

- `screenshots` — screen capture
- `image` — image encoding/saving
- `clap` — CLI argument parsing
- `chrono` — timestamp-based filenames
- `winit` + `softbuffer` — selection overlay window
- `tray-icon` — system tray
- `global-hotkey` — hotkey registration
- `arboard` — clipboard access

## Platform Notes

- Linux requires X11 libraries: `libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`
- Screen recording is Windows-only (Media Foundation).
- Windows has tray and hotkey support — easy to break accidentally.

## Lessons Learned

- For GitLab CI failures, fetch the raw job log with `glab api projects/stden%2FRustShot/jobs/<job_id>/trace` when the web page is not enough.
- The CI clippy job runs `cargo clippy --tests -- -D warnings`; reproduce that exact command for clippy failures and keep it green before pushing.
- Do not deny entire `clippy::pedantic` or `clippy::nursery` groups at workspace level unless the whole codebase is already clean for those groups.
- The repository config uses `sccache` as the Rust compiler wrapper. If local validation fails because `sccache` cannot run or is sandboxed, retry verification with `RUSTC_WRAPPER` unset while preserving the checked-in config.
- Keep platform-specific state behind the same `#[cfg(...)]` as the code that reads it. Dummy values for other platforms can become `dead_code` under `-D warnings`.
- After pushing, query the exact pushed commit SHA with `git rev-parse HEAD`, then check GitLab with `glab api "projects/stden%2FRustShot/pipelines?ref=<branch>&sha=<sha>"`.
- Pipeline jobs can be inspected with `glab api projects/stden%2FRustShot/pipelines/<pipeline_id>/jobs`; if a job fails, fetch its trace before guessing.

## Codev Methodology

This project uses the Codev SPIR protocol for structured development.

- Specifications: `codev/specs/`
- Plans: `codev/plans/`
- Reviews: `codev/reviews/`
- Protocol: `codev/protocols/spir/protocol.md`
