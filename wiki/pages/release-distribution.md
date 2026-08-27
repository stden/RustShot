[← Back to Index](../index.md)

# Release and Distribution

The release process is treated as a core part of the product, ensuring consistent delivery to users.

## Release Components
- **Target Builds**: Compiling for specific architectures, primarily **Windows MSVC (`x86_64-pc-windows-msvc`)**.
- **Installer (`Setup.exe`)**: Created using Inno Setup (`installer.iss`). 
    - Uses **`PrivilegesRequired=lowest`** for per-user installation without admin prompts.
    - Manages **Autorun** via `HKCU` registry keys.
- **Portable Version**: Standalone `rustshot.exe` binary.
- **CI/CD Pipeline**: Automated via **GitHub Actions** (`release.yml`).
    - Normalizes paths by copying binary to `target\release\` before running `iscc`.
    - Publishes artifacts to GitHub Releases on `v*` tags.

## Best Practices
* **Zero Manual Steps**: Aim for full automation of the build and release process.
* **Installer Hygiene**: Proper installation paths, registry cleanup, and autorun management.
* **Artifact Naming**: Predictable and clear naming conventions for binaries.
* **Fast Feedback**: A robust pipeline allows for quicker fixes and updates.

## 📋 Project Specifications (Codev)
* [Spec 0005: Release and Distribution Hardening](../../codev/specs/0005-release-and-distribution-hardening.md)

## See Also
* [Source: Release is a Feature](../pages/source-release-feature.md)
* [Source: Inno Setup Pipeline](../pages/source-inno-pipeline.md)
