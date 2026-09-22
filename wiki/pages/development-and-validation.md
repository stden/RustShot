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
  - .claude/rules/testing.md
  - .claude/rules/benchmarks.md
  - Cargo.toml
  - .github/workflows/ci.yml
related:
  - concept.architecture-overview
  - concept.release-distribution
  - source.github-actions-pipeline
---
# Development and Validation

## Answer

`CLAUDE.md` is the canonical project instruction file. Run `cargo lint` for the same Clippy policy as CI; bare
`cargo clippy` can report findings while exiting successfully. Run `cargo test` for unit, integration, end-to-end,
and Cucumber coverage. Gherkin files under `tests/features/` are the executable behavior specification.

Unit test bodies live under `tests/unit/` and compile as private child modules. Architecture rationale stays in rustdoc
beside the code. General Rust and testing advice belongs in the central `iopm` repository rather than this corpus.

## Grounding

- [Canonical project instructions](../../CLAUDE.md)
- [Test rules and environment boundaries](../../.claude/rules/testing.md)
- [Benchmark evidence and budget rules](../../.claude/rules/benchmarks.md)
- [Cargo lint and executable-spec targets](../../Cargo.toml)
- [Windows, Linux, macOS, lint, format, dependency, coverage, and benchmark jobs](../../.github/workflows/ci.yml)

## Constraints

- Repository text files use CRLF; restore it after multiline edits.
- Do not hardcode test counts in documentation.
- Behavior changes need a Gherkin scenario and an implementation-level test.
- Performance claims require measured evidence; benchmark IDs and budgets must move together.
- The pull-request performance job uses `benches/performance_gate.rs`; exploratory live-capture benchmarks are separate.

## Related

- [Architecture Overview](architecture-overview.md)
- [Release and Distribution](release-distribution.md)
- [GitHub Actions CI/CD](source-github-pipeline.md)
