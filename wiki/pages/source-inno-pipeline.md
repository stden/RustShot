---
id: source.inno-setup-pipeline
kind: source
title: Inno Setup Pipeline
summary: Summarizes the technical guide for building RustShot's per-user Windows installer locally and in GitHub Actions.
tags:
  - inno-setup
  - installer
  - windows
source_paths:
  - docs/INNO_SETUP_PIPELINE.md
related:
  - concept.release-distribution
---
# Inno Setup Pipeline

## Canonical source

- [Inno Setup and Release Pipeline](../../docs/INNO_SETUP_PIPELINE.md)

## Retrieval summary

- The installer consumes a flat `target\release\rustshot.exe`, so target-specific builds must copy the executable
  before invoking Inno Setup.
- The build replaces `{{VERSION}}` in a temporary installer script using the version from `Cargo.toml`.
- The per-user installer avoids elevation, owns the `HKCU` autorun entry, creates shortcuts, and cleans up on uninstall.
- The guide documents equivalent local and CI commands for producing the portable executable and installer.

## Related

- [Release and Distribution](release-distribution.md)
