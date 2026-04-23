[← Back to Index](../index.md)

# Source: Inno Setup Pipeline

* **Type**: Technical Documentation / Lesson
* **Original Title**: Inno Setup And Release Pipeline Lesson
* **File**: `wiki/raw/INNO_SETUP_PIPELINE.md`
* **Topic**: Installer creation, CI/CD pipeline, Windows deployment.

## Key Takeaways
1. **Deliverables**: Project produces both a portable `rustshot.exe` and a `RustShotSetup.exe` installer.
2. **Inno Setup Role**: Handles file copying, Start Menu shortcuts, registry-based autorun, and uninstallation.
3. **Privilege Model**: Uses `PrivilegesRequired=lowest` (per-user install), which avoids admin prompts and allows clean `HKCU` autorun registration.
4. **Path Normalization**: CI and local flows must copy the binary from the target-specific directory (`target\x86_64-pc-windows-msvc\release\`) to a flat `target\release\` directory expected by `installer.iss`.
5. **CI Trigger**: Triggered by `v*` tags or manual dispatch; uses GitHub Actions to build MSVC targets.
6. **Maintenance Risks**: Hardcoded version in `installer.iss` creates risk of version drift with `Cargo.toml`.

## Linked Concepts
* [Release and Distribution](release-distribution.md)
* [Architecture Overview](architecture-overview.md)
