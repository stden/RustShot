# GEMINI.md

This document gives instructions for Gemini when working with RustShot.

## About the Project

RustShot is a screenshot and screen recording application.

Main source files:

- `src/lib.rs`
- `src/main.rs`
- `src/cli.rs`
- `src/capture.rs`
- `src/selection.rs`
- `src/tray.rs`
- `src/record.rs`
- `src/settings.rs`

## Language

Write all descriptions, comments, commit messages, code reviews, and development discussions in English.

## Rules

- Keep solutions simple. Do not add complexity without a reason.
- Do not change program behavior without a reason.
- Write code that is easy to test.
- If you change program behavior, add a test.

## Commands

```bash
cargo build
cargo check
cargo test
cargo fmt
cargo clippy
cargo bench
python wiki/lint.py
```

## Change Procedure

Follow this order when you make changes:

1. Write a test.
2. Run the test. It must fail.
3. Fix the code.
4. Run the test. It must pass.
5. Run `cargo test`. All tests must pass.

## Focus Areas

- Small helper functions
- Clear error messages
- Tests for CLI, capture, selection, and tray logic

## What to Avoid

- Large refactoring that the task does not require
- New dependencies that do not add value
- Complex GUI tests when you can test the logic directly

## Documentation Rules

- Check that the examples in `README.md` match the real program behavior.
- Do not write fixed numbers (for example, a test count) in the documentation.
- If the development rules change, update this file too.

## Retrieval Knowledge Base

The knowledge base for RustShot is in the `wiki/` directory.

### Structure

- Canonical sources stay in code, `articles/`, and `docs/`. The corpus does not duplicate raw files.
- `wiki/pages/`: Retrieval chunks with stable IDs, tags, canonical source paths, and related-page IDs.
- `wiki/index.md`: Task-oriented catalog of every corpus page.
- `wiki/templates/`: Metadata and section contracts for concepts, entities, and source summaries.

### Operations

- **Ingest**: Start from a template. Summarize only RustShot-specific facts. List every canonical source in `source_paths`. Add the page once to `index.md`.
- **Query**: Match the question to index descriptions and page tags. Read the relevant chunks. Verify claims in their canonical sources.
- **Lint**: Run `python wiki/lint.py`. CI rejects malformed metadata, duplicate IDs, missing sources, broken links, invalid relationships, and unindexed pages.
