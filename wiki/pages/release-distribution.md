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
  - tests/features/first_run_install.feature
related:
  - concept.development-and-validation
  - source.github-actions-pipeline
  - source.inno-setup-pipeline
  - source.release-is-a-feature
---
# Release and Distribution

## Answer

A release produces a portable `rustshot.exe` and `RustShotSetup.exe`. The GitHub release workflow builds the
`x86_64-pc-windows-msvc` target, copies the binary into the flat path expected by Inno Setup, substitutes the package
version from `Cargo.toml`, publishes both artifacts, and creates release notes for a `v*` tag.

The installer uses `PrivilegesRequired=lowest` for a per-user installation. It creates Start Menu entries, manages the
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\RustShot` autorun value, starts the tray application after
installation, and removes its registry value during uninstall.

## Grounding

- [Package version and Windows dependencies](../../Cargo.toml)
- [Installer definition](../../installer.iss)
- [Release automation](../../.github/workflows/release.yml)
- [Installer build and behavior reference](../../docs/INSTALLERS.md)
- [Executable first-run installation behavior](../../tests/features/first_run_install.feature)

## Constraints

- `installer.iss` is a template; the release job must replace `{{VERSION}}` before invoking Inno Setup.
- Inno Setup reads `target\release\rustshot.exe`, while the target build initially writes below
  `target\x86_64-pc-windows-msvc\release\`.
- Creating a GitHub Release requires workflow `contents: write` permission.

## Related

- [Development and Validation](development-and-validation.md)
- [GitHub Actions CI/CD](source-github-pipeline.md)
- [Inno Setup Pipeline](source-inno-pipeline.md)
- [Release as a Product](source-release-feature.md)
