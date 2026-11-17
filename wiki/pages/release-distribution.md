---
id: concept.release-distribution
kind: concept
title: Release and Distribution
summary: Maps RustShot's portable Windows binary, per-user installer, version substitution, and GitHub release pipeline.
tags:
  - github-actions
  - inno-setup
  - release
  - windows
source_paths:
  - Cargo.toml
  - installer.iss
  - .github/workflows/release.yml
  - docs/INSTALLERS.md
  - src/main.rs
related:
  - concept.development-and-validation
  - source.github-actions-pipeline
  - source.inno-setup-pipeline
  - source.release-is-a-feature
---
# Release and Distribution

## Answer

A release produces a portable `rustshot.exe` and `RustShotSetup.exe`. The GitHub release workflow triggers on pushes to `main`, `v*` tags, and `workflow_dispatch` with `contents: write` permission: it resolves `VERSION` and `RELEASE_TAG` (`v<version>`) from `Cargo.toml`, builds the `x86_64-pc-windows-msvc` target, copies the binary into `target\release\rustshot.exe`, substitutes `{{VERSION}}` in `installer.iss`, updates the `v<version>` release tag, and publishes both artifacts to GitHub Releases.

The installer uses `PrivilegesRequired=lowest` for a per-user installation. It creates Start Menu entries, manages the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\RustShot` autorun value, starts the tray application after installation, and removes its registry value during uninstall.

## Grounding

- [Package version and Windows dependencies](../../Cargo.toml)
- [Installer definition](../../installer.iss)
- [Release automation](../../.github/workflows/release.yml)
- [Installer build and behavior reference](../../docs/INSTALLERS.md)
- [First-run installation and autorun registration](../../src/main.rs)

## Constraints

- `installer.iss` is a template; the release job must replace `{{VERSION}}` before invoking Inno Setup.
- Inno Setup reads `target\release\rustshot.exe`, while the target build writes `target\x86_64-pc-windows-msvc\release\RustShot.exe`.
- Creating a GitHub Release and updating tags requires workflow `contents: write` permission.

## Related

- [Development and Validation](development-and-validation.md)
- [GitHub Actions CI/CD](source-github-pipeline.md)
- [Inno Setup Pipeline](source-inno-pipeline.md)
- [Release as a Product](source-release-feature.md)
