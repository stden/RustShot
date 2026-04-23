# 🤖 CLAUDE.md

Instructions for Claude when working with `RustShot`.

## 🌍 Language

All descriptions, comments, commit messages, code reviews, and development discussions must be in English.

## ✅ Core principles

- Make small, clear changes.
- Do not break existing behavior without a reason.
- If you change something, add a test.
- Do not do large architectural refactoring for a small task.

## 📁 Project structure

The project is small. Key modules:

- `src/main.rs` — entry point and CLI
- `src/capture.rs` — screenshot capture and file saving
- `src/selection.rs` — mouse-driven region selection
- `src/tray.rs` — system tray and menu
- `src/record.rs` — screen recording
- `src/settings.rs` — persisted settings and settings-file workflow

## ⚡ Commands

```bash
cargo check          # Verify compilation
cargo test           # Run all tests
cargo fmt            # Format code
cargo clippy         # Lint
cargo bench          # Run benchmarks
cargo build --release # Release build
```

## ✍️ Commit Convention

- `[+]` — additions (new features, new tests)
- `[-]` — deletions (removed features, removed code)
- `[!]` — refactoring, fixes, improvements

## 📐 Workflow

1. ✏️ Write a test first.
2. ❌ Verify it fails.
3. ✅ Make the minimal fix.
4. ✅ Verify the test passes.
5. 🧪 Run all tests.

## 🧪 Most useful tests

- CLI argument parsing tests
- File path and format selection tests
- Error message tests
- Small helper function tests

Avoid heavy GUI tests unless absolutely necessary.

## ⚠️ Error handling

- The project uses `Result<_, String>` throughout.
- Error messages must be human-readable.
- Do not silently swallow errors unless that is the intended behavior.

## 🌍 Platform notes

- 🐧 Linux requires X11 libraries.
- 🎬 Screen recording is currently Windows-only.
- 🪟 Windows has tray and hotkey support — easy to break accidentally.
- 🧪 Test selection and window logic through small pure-logic units, not only manual testing.

## 📚 Codev Methodology

This project uses the Codev context-driven development methodology.

### 🔄 Active Protocol
- Protocol: SPIR
- Location: codev/protocols/spir/protocol.md

### 📂 Directory Structure
- Specifications: codev/specs/
- Plans: codev/plans/
- Reviews: codev/reviews/
- Resources: codev/resources/

See codev/protocols/spir/protocol.md for full protocol details.

---

[Версия на русском языке (CLAUDE.ru.md)](CLAUDE.ru.md)
