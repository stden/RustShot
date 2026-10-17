# RustShot — Kiro Steering

## What this project is

RustShot is a Windows screenshot and screen-recording utility written in Rust.
It runs as a system-tray application with a global hotkey and supports
interactive region selection.

## Source layout

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, CLI argument parsing, tray bootstrap |
| `src/capture.rs` | Screenshot capture and file saving |
| `src/selection.rs` | Mouse-driven region selection overlay |
| `src/tray.rs` | System-tray icon and menu |
| `src/record.rs` | Screen recording |

## Build system

This is a standard Cargo project. Use these commands:

```bash
cargo check          # verify compilation
cargo test           # run all tests
cargo fmt            # format code
cargo clippy         # lint (treat warnings as errors with -D warnings)
cargo build --release  # production binary
```

## Coding conventions

- Error handling uses `Result<_, String>`. Error messages must be human-readable.
- Never silently swallow errors unless that is explicitly the intended behavior.
- Keep functions small. Extract helper functions for testable logic.
- No commented-out code in commits. No debug `println!` left in production paths.
- Follow standard Rust naming: `snake_case` for variables/functions, `PascalCase` for types.
- Prefer `match` over chains of `if let` when there are multiple arms.

## Testing approach

- TDD order: write the test first, see it fail, make the minimal fix, verify green, run all tests.
- Prefer unit tests on small pure helper functions over GUI/system integration tests.
- Good test targets: CLI argument parsing, filename/path construction, error message text,
  selection geometry calculations, format detection.
- Avoid heavy GUI automation tests unless there is no other option.
- Every bug fix must be accompanied by a regression test.

## Platform notes

- Primary platform is Windows. The tray icon and global hotkey are Windows-only.
- Linux requires X11 libraries: `libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`.
- Do not break Windows tray/hotkey behavior when making cross-platform changes.

## Key dependencies

| Crate | Role |
|-------|------|
| `screenshots` | Low-level screen capture |
| `image` | Image encoding and saving |
| `clap` | CLI argument parsing |
| `chrono` | Timestamp-based filename generation |

## Change discipline

- Make small, focused changes. One logical concern per commit.
- Do not refactor unrelated code while fixing a bug or adding a feature.
- If you change observable behavior, add or update a test.
- Do not introduce large architectural restructuring for a small task.

## Codev methodology

This project uses the SPIR protocol for structured feature development.

| Phase | Document location | Template |
|-------|-------------------|---------|
| Specify | `codev/specs/####-name.md` | `codev/protocols/spir/templates/spec.md` |
| Plan | `codev/plans/####-name.md` | `codev/protocols/spir/templates/plan.md` |
| Review | `codev/reviews/####-name.md` | `codev/protocols/spir/templates/review.md` |

- Use SPIR for new features, architecture changes, and complex refactors.
- Skip SPIR for simple bug fixes (fewer than ~10 lines) and docs-only changes.
- Commit message format for SPIR work: `[Spec ####][Phase: name] type: description`
- Each SPIR phase must end with a git commit before the next phase begins.
- Never include time estimates in plans. Track progress by phase completion only.

## What Kiro should do by default

- Always run `cargo check` before suggesting code changes are complete.
- Always run `cargo test` after any code modification.
- When creating a new function, suggest a corresponding unit test.
- When editing `selection.rs` or `capture.rs`, check that existing tests still pass.
- Prefer editing existing files over creating new ones unless a new module is clearly needed.
- Surface compiler warnings — do not suppress them with `#[allow(...)]` without a comment explaining why.
