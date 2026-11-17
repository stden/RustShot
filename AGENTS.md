# RustShot - AI Agent Instructions

> Compatible with OpenAI Codex, Cursor, GitHub Copilot, and other AI coding assistants.
> Claude Code-specific instructions are in `CLAUDE.md`.

## Project Overview

`RustShot` is a screenshot utility for all platforms and a screen recording utility (currently **Windows only**).

## Key Files

- `src/lib.rs` — shared library crate exporting core modules for unit/integration tests and benchmarks
- `src/main.rs` — entry point, CLI parsing, tray event loop
- `src/cli.rs` — pure CLI helper logic and application mode routing
- `src/capture.rs` — screenshot capture and file saving
- `src/selection.rs` — interactive region selection overlay (multi-monitor, DPI-aware)
- `src/tray.rs` — system tray icon and context menu
- `src/record.rs` — screen recording to MP4 (Windows only, H.264 via Media Foundation)
- `src/settings.rs` — JSON settings persistence (manual parser, no serde)
- `RustShot.typ` — illustrated textbook source; compiles to `RustShot.pdf`
- `docs/diagrams/` — editable SVG illustrations shared by the documentation and textbook
- `mise.toml` — managed tools and build, verification, benchmark, and documentation tasks

## Language

All descriptions, comments, commit messages, code reviews, and development discussions must be in English.

## Core Rules

- Do not change behavior without a reason.
- Do not do large refactoring for a small task.
- Extract complex logic into small testable functions.
- If you change behavior, add a test.
- The project uses `Result<_, String>` for error handling throughout.
- Do not hardcode dynamic numbers (such as test counts) in documentation, and avoid unnecessary documentation updates.

## Commands

Use mise for the configured toolchain and tasks. Prefix shell commands with
`rtk`; use `rtk proxy` for commands without a dedicated RTK filter.
Inspect `mise.toml` before trusting a new checkout.

```bash
rtk proxy mise install         # Install configured tools
rtk proxy mise run check       # Check all Cargo targets
rtk proxy mise run test        # Application and duplication detector tests
rtk proxy mise run fmt         # Format both Rust packages
rtk proxy mise run lint        # Clippy with warnings denied
rtk proxy mise run ci          # Format, lint, duplication, and test checks
rtk proxy mise run build       # Portable release
rtk proxy mise run build:native # Release for this CPU
rtk proxy mise run bench       # Criterion benchmarks
rtk proxy mise run coverage    # Export target/coverage.json
rtk proxy mise run docs:pdf    # Compile RustShot.typ to RustShot.pdf
rtk proxy mise exec -- python wiki/lint.py # Validate the knowledge base
```

Use `rtk proxy mise exec -- <command>` for tools without a task. Keep native
releases on compatible CPUs; use the portable release for distribution.
Update mise itself with
`rtk proxy mise self-update --yes --no-plugins --minimum-release-age 0s`,
then verify `rtk proxy mise --version`.

## Documentation and Illustrations

- Check descriptions against the current source before changing a workflow diagram.
- Keep user guidance in `README.md`, architecture details in
  `docs/architecture-tutorial.md`, and the illustrated learning material in `RustShot.typ`.
- Prefer editable SVG or Typst drawings for architecture and algorithm illustrations.
  Label synthetic UI illustrations so they are not mistaken for application screenshots.
- Keep image paths relative to the repository root or the referring document.
- Rebuild the PDF with `mise run docs:pdf` after editing the textbook or its illustrations.
  Inspect rendered pages for clipped text, unreadable labels, and unexpected blank pages.
- Generated PDFs and preview images are build artifacts. Keep the Typst source and SVG
  illustrations in Git; place temporary previews under `target/`.
- Documentation-only changes require a successful document build and relevant link/asset
  checks. Behavior changes still require the Rust test workflow below.

## Development Workflow

1. Write a test first.
2. Verify it fails.
3. Make the minimal fix.
4. Verify the test passes.
5. Run the complete suite with `rtk proxy mise run test`.

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

## Codev Methodology

This project uses the Codev SPIR protocol for structured development.

- Specifications: `codev/specs/`
- Plans: `codev/plans/`
- Reviews: `codev/reviews/`
- Protocol: `codev/protocols/spir/protocol.md`

## Retrieval Knowledge Base

- Retrieval: `wiki/index.md` → metadata-tagged chunks in `wiki/pages/` → their canonical `source_paths`.
- Run `python wiki/lint.py` and `python -m unittest wiki.test_lint` after changing the corpus.
