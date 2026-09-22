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

- CI separates formatting, Rust lints and rustdoc, dependency policy, platform tests, Linux coverage, and pull-request
  performance checks.
- Tests run on Windows, Linux, and macOS; the Linux job installs the native display and tray dependencies and uses Xvfb.
- Release automation builds the Windows MSVC target and publishes both the portable executable and Inno Setup installer.
- Workflow files are canonical when the guide and automation differ.

## Related

- [Development and Validation](development-and-validation.md)
- [Release and Distribution](release-distribution.md)
