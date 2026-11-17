---
id: source.github-actions-pipeline
kind: source
title: GitHub Actions CI/CD
summary: Summarizes the technical guide to RustShot's cross-platform CI checks, release job, caching, and failure modes.
tags:
  - ci
  - github-actions
  - release
source_paths:
  - docs/github-pipeline.md
  - .github/workflows/ci.yml
  - .github/workflows/release.yml
related:
  - concept.development-and-validation
  - concept.release-distribution
---
# GitHub Actions CI/CD

## Canonical source

- [GitHub Actions: A CI/CD Pipeline for Rust](../../docs/github-pipeline.md)
- [Current CI workflow](../../.github/workflows/ci.yml)
- [Current release workflow](../../.github/workflows/release.yml)

## Retrieval summary

- CI runs parallel jobs for RAG knowledge base validation (`python -B wiki/lint.py` and `unittest`), formatting (`cargo fmt --all --check`), Clippy (`--all-targets -- -D warnings`), Windows tests, and Linux tests/coverage (`xvfb-run` + `cargo tarpaulin --engine llvm`).
- Both workflows clear `RUSTC_WRAPPER` so runner builds do not depend on local compiler wrappers.
- Release automation builds the Windows MSVC target, resolves `v<version>` from `Cargo.toml`, updates the release tag, and publishes both `rustshot.exe` and `RustShotSetup.exe` to GitHub Releases.
- Workflow files are canonical when the guide and automation differ.

## Related

- [Development and Validation](development-and-validation.md)
- [Release and Distribution](release-distribution.md)
