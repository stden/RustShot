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
cargo lint
```

`cargo lint` is the alias in `.cargo/config.toml` for
`clippy --all-targets --all-features -- -D warnings`, and it is exactly what CI runs
(`.github/workflows/ci.yml`). Do not invoke clippy directly: `Cargo.toml`'s `[lints]`
table is entirely `warn`, so without the alias's `-D warnings` the same findings print
and the command still exits 0 — green locally, red in CI.

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

## 🧠 Retrieval knowledge base

A persistent knowledge base for RustShot, located in `wiki/`.

### Structure

- Canonical sources stay in code, `tests/features/`, `articles/`, and `docs/`; the corpus does not duplicate raw files.
- `wiki/pages/`: Retrieval chunks with stable IDs, tags, canonical source paths, and related-page IDs.
- `wiki/index.md`: Task-oriented catalog of every corpus page.
- `wiki/templates/`: Metadata and section contracts for concepts, entities, and source summaries.

### Operations

- **Ingest**: Start from a template, summarize only RustShot-specific facts, list every canonical source in `source_paths`, and add the page once to `index.md`.
- **Query**: Match the question to index descriptions and page tags, read the relevant chunks, then verify claims in their canonical sources.
- **Lint**: Run `python wiki/lint.py`; CI rejects malformed metadata, duplicate IDs, missing sources, broken links, invalid relationships, and unindexed pages.
