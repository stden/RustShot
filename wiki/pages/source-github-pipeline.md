[← Back to Index](../index.md)

# Source: GitHub Actions CI/CD

* **Type**: Technical Documentation / Tutorial
* **Original Title**: GitHub Actions: A CI/CD Pipeline for Rust
* **File**: `docs/github-pipeline.md`
* **Topic**: CI/CD, GitHub Actions, Rust automation.

## Key Takeaways

1. **CI vs CD**: CI handles code quality (fmt, clippy, tests) on every push/PR. CD handles automated releases on tags.
2. **Parallel Jobs**: Jobs like `fmt`, `clippy`, and `tests` run in parallel by default to save time.
3. **Platform-Specific Testing**:
    - **Windows**: Primary platform, used for clippy and main tests.
    - **Linux**: Requires specific X11/GTK dev libraries (`libxcb1-dev`, etc.) to compile GUI-related code.
4. **Caching**: `actions/cache` is used in release builds for `~/.cargo/registry` and `target` to speed up compilation.
5. **Release Workflow**:
    - Triggered by `v*` tags.
    - Uses `softprops/action-gh-release` to attach binaries and installers to GitHub Releases.
    - Automatically generates release notes.
6. **Common Issues**: Version drift between `Cargo.toml` and workflows, missing Linux dependencies, and rustfmt version
   mismatches.

## Linked Concepts

* [Release and Distribution](release-distribution.md)
* [Rust Development Practices](rust-development-practices.md)
