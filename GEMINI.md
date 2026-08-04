# 💎 GEMINI.md

This file provides instructions for Gemini when working on `RustShot`.

## 📸 Project at a glance

`RustShot` is a screenshot and screen-recording utility.

The main files are:

- `src/main.rs`
- `src/capture.rs`
- `src/selection.rs`
- `src/tray.rs`
- `src/record.rs`
- `src/settings.rs`

## 🌍 Language

All descriptions, comments, commit messages, code reviews, and development discussions must be in English.

## ✅ Simple rules

- Do not make a solution more complex than necessary.
- Do not change application behavior without a reason.
- Write code that is easy to test.
- Add a test whenever behavior changes.

## ⚡ Useful commands

```bash
cargo build
cargo check
cargo test
cargo fmt
cargo clippy
```

## 📐 Preferred change workflow

Use this order:

1. ✏️ Write a test.
2. ❌ Verify that the test fails.
3. ✅ Fix the code.
4. ✅ Verify that the test passes.
5. 🧪 Run `cargo test`.

## 🎯 Areas of emphasis

- Small helper functions
- Clear error messages
- Tests for CLI, capture, selection, and tray logic

## 🚫 Things to avoid

- Large, unnecessary refactorings
- New dependencies without a clear benefit
- Complex GUI tests when the underlying logic can be tested more simply

## 📝 Documentation changes

- Verify that examples in `README.md` match the application's actual behavior.
- Update this file when development rules change.

## 🧠 LLM Wiki

A persistent knowledge base for RustShot, located in `wiki/`.

### Structure

- Raw source documents live in their canonical homes: `articles/` for long-form essays and `docs/` for technical and pipeline documentation. The wiki no longer keeps a duplicate `wiki/raw/` copy; each `wiki/pages/source-*.md` page links to its origin.
- `wiki/pages/`: Synthesized wiki pages containing concepts, entities, and summaries.
- `wiki/index.md`: Content-oriented catalog of all pages.
- `wiki/log.md`: Chronological log of wiki operations.

### Operations

- **Ingest**: Process a source file from `articles/` or `docs/`. Read, discuss, and summarize it in `wiki/pages/`, then update `index.md` and `log.md`.
- **Query**: Search `index.md`, read relevant pages, synthesize an answer, and optionally save it as a new page.
- **Lint**: Check for contradictions, stale claims, orphan pages, and missing cross-references.
