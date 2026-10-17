---
id: concept.development-and-validation
kind: concept
title: Development and Validation
summary: Collects RustShot-specific rules for linting, tests, documentation, line endings, benchmarks, and CI coverage.
tags:
  - bdd
  - ci
  - linting
  - testing
source_paths:
  - CLAUDE.md
  - AGENTS.md
  - Cargo.toml
  - benches/benchmarks.rs
  - .github/workflows/ci.yml
related:
  - concept.architecture-overview
  - concept.release-distribution
  - source.github-actions-pipeline
---
# Development and Validation

## Answer

[`CLAUDE.md`](../../CLAUDE.md) and [`AGENTS.md`](../../AGENTS.md) define the canonical project instructions. Run `cargo clippy --all-targets -- -D warnings` for the same Clippy policy as CI, `cargo fmt --all --check` for formatting, `cargo test` for unit and integration test coverage, and `python wiki/lint.py` to validate the RAG knowledge base.

Unit tests live alongside each module and in `tests/`, testing pure helper logic rather than brittle GUI interactions. Criterion benchmarks live in [`benches/benchmarks.rs`](../../benches/benchmarks.rs).

## Grounding

- [Claude project instructions](../../CLAUDE.md)
- [AI agent instructions](../../AGENTS.md)
- [Cargo package and benchmark configuration](../../Cargo.toml)
- [Criterion benchmarks](../../benches/benchmarks.rs)
- [CI workflow](../../.github/workflows/ci.yml)

## Constraints

- Follow test-driven development: write a failing test first, make the minimal fix, and verify all tests pass.
- All code comments, commit messages, and documentation changes must be in English.
- Behavior changes require corresponding unit tests.
- Changes to `wiki/` must pass `python wiki/lint.py` and `python -m unittest wiki.test_lint`.

## Related

- [Architecture Overview](architecture-overview.md)
- [Release and Distribution](release-distribution.md)
- [GitHub Actions CI/CD](source-github-pipeline.md)
