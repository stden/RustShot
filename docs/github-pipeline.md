# GitHub Actions: A CI/CD Pipeline for Rust

A practical guide based on the RustShot project.

## Table of contents

1. [What CI/CD means](#1-what-cicd-means)
2. [Workflow structure](#2-workflow-structure)
3. [CI pipeline: checking the code](#3-ci-pipeline-checking-the-code)
4. [Release pipeline: building and publishing](#4-release-pipeline-building-and-publishing)
5. [Caching](#5-caching)
6. [Cross-platform builds](#6-cross-platform-builds)
7. [Common problems](#7-common-problems)

---

## 1. What CI/CD means

**CI (Continuous Integration)** automatically checks every commit:

- Does the code compile?
- Do the tests pass?
- Is the formatting correct?
- Does the linter report any problems?

**CD (Continuous Delivery)** automatically builds and publishes releases.

Workflow files live in `.github/workflows/`:

```text
.github/
  workflows/
    ci.yml       # Checks every push and pull request
    release.yml  # Builds when a tag is created
```

---

## 2. Workflow structure

```yaml
name: CI                    # Name shown in the GitHub UI

on:                         # Events that start the workflow
  push:
    branches: ["main"]      # A push to main
  pull_request:             # Any pull request

env:                        # Environment variables
  CARGO_TERM_COLOR: always

jobs:                       # List of jobs
  fmt:                      # Arbitrary job identifier
    name: Format            # Display name
    runs-on: ubuntu-latest  # Runner operating system
    steps:                  # Ordered steps
      - uses: actions/checkout@v4
      - run: cargo fmt --all --check
```

### Key fields

| Field | Description |
|------|-------------|
| `on.push.branches` | Branches that trigger the workflow |
| `on.push.tags` | Tags that trigger the workflow |
| `on.pull_request` | Run for pull requests |
| `on.workflow_dispatch` | Allow a manual run from the GitHub UI |
| `runs-on` | Runner OS: `ubuntu-latest`, `windows-latest`, or `macos-latest` |
| `steps` | Ordered job steps |
| `uses` | A reusable action from the marketplace |
| `run` | A Bash or PowerShell command |

---

## 3. CI pipeline: checking the code

The examples below walk the jobs a Rust project usually needs, using RustShot's as the worked case. They are not the complete workflow: `.github/workflows/ci.yml` has eight jobs — `fmt`, `rust-lints`, `dependency-lints`, `test-windows`, `test-macos`, `test-linux`, coverage, and the pull-request-only benchmark gate. Read the workflow as the source of truth and this file as the explanation of *why* each job looks the way it does.

### Job 1: Formatting

```yaml
fmt:
  name: Format
  runs-on: ubuntu-latest   # Formatting is platform-independent, so use the fastest runner
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt
    - run: cargo fmt --all --check
```

`cargo fmt --check` does not modify files; it only verifies formatting. It exits with code 1 when the code is not formatted.

### Job 2: Linting

```yaml
rust-lints:
  name: Rust lints
  runs-on: windows-latest  # Windows is the project's primary platform
  steps:
    - uses: actions/checkout@v5
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy
    - run: cargo lint          # alias: clippy --all-targets --all-features -- -D warnings
    - name: Rustdoc warnings
      run: cargo doc --all-features --no-deps
      env:
        RUSTDOCFLAGS: -D warnings
```

`cargo lint` is an alias defined in `.cargo/config.toml`. `-D warnings` promotes Clippy warnings to errors — without it Clippy reports the same findings and still exits 0, which is why bare `cargo clippy` cannot gate this repo: the `[lints]` table in `Cargo.toml` sets everything to `warn`.

The alias checks the library and binary targets with all features enabled; tests
and benchmarks run in their dedicated jobs.

The same job carries a second step: rustdoc run with `-D warnings`, which is what keeps intra-doc links from rotting.

### Job 3: Windows tests

```yaml
test-windows:
  name: Test (Windows)
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test
```

### Job 4: Linux tests

```yaml
test-linux:
  name: Test (Linux)
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Install X11 and GTK dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y \
          libxcb1-dev \
          libxcb-render0-dev \
          libxcb-shape0-dev \
          libxcb-xfixes0-dev \
          libglib2.0-dev \
          libgtk-3-dev \
          libxdo-dev \
          libatk1.0-dev \
          libgdk-pixbuf2.0-dev \
          libcairo2-dev \
          libpango1.0-dev

    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test
```

Linux requires system libraries for GUI dependencies. Without them, `cargo test` fails during linking.

Coverage uses `taiki-e/install-action` to download the supported prebuilt
`cargo-tarpaulin` release. Avoid `cargo install cargo-tarpaulin` in CI:
compiling the coverage tool from source on every clean runner adds several
minutes without testing RustShot.

### Job 5: Performance regression gate

The pull-request-only `Performance benchmarks` job runs the deterministic
`performance_gate` Criterion target on `windows-latest`. It creates a
detached worktree at the pull request's base SHA and measures base and head
sequentially on the same runner and target directory.

    cargo bench --bench performance_gate -- --save-baseline base
    cargo bench --bench performance_gate -- --baseline-lenient base
    pwsh -File scripts/benchmark_guard.ps1 -CriterionRoot target/criterion -RegressionThreshold 0.10

The guard uses the median confidence interval rather than one timing sample. A
relative failure uses a 10% lower-bound threshold by default, with wider
per-benchmark thresholds only for the Windows allocation-heavy entries that
proved noisy even on unchanged code. New benchmarks without a base result use
generous absolute budgets until they have history, which is what
`--baseline-lenient` buys: the strict `--baseline`
panics on an id the base branch never measured, so a pull request that adds a
benchmark used to fail before any budget was checked. The Criterion HTML and JSON
reports are retained as a workflow artifact for 14 days.

### Parallelism

All CI jobs run **in parallel** by default. Use `needs` when one job depends on another:

```yaml
test-windows:
  needs: [fmt, rust-lints]  # Starts only after fmt and rust-lints finish
```

RustShot has no dependency between these jobs, so they remain independent for speed.

---

## 4. Release pipeline: building and publishing

### Triggering with a version tag

```yaml
on:
  push:
    tags:
      - 'v*'             # v0.1.0, v1.0.0, and so on
  workflow_dispatch:     # Also allow a manual run
```

### Creating a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

### Complete release workflow

```yaml
jobs:
  build-windows:
    name: Build Windows Release
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-pc-windows-msvc

      # The cache accelerates repeat builds
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.toml') }}

      # Build the release executable
      - run: cargo build --release --target x86_64-pc-windows-msvc

      # Copy the executable to the location expected by the installer
      - name: Prepare installer source
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force -Path target\release | Out-Null
          Copy-Item target\x86_64-pc-windows-msvc\release\rustshot.exe target\release\rustshot.exe

      # Resolve the template version, then build the installer
      - name: Build installer
        shell: pwsh
        run: |
          $version = (Get-Content Cargo.toml | Select-String '^version = "(.*)"').Matches.Groups[1].Value
          (Get-Content installer.iss).Replace('{{VERSION}}', $version) | Set-Content installer.iss
          iscc installer.iss

      # Upload artifacts
      - uses: actions/upload-artifact@v4
        with:
          name: rustshot-windows-x64-portable
          path: target\x86_64-pc-windows-msvc\release\rustshot.exe

      - uses: actions/upload-artifact@v4
        with:
          name: rustshot-windows-x64-installer
          path: Output\RustShotSetup.exe

      # Create a GitHub Release containing the files
      - name: Create GitHub Release
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: |
            target/x86_64-pc-windows-msvc/release/rustshot.exe
            Output/RustShotSetup.exe
          generate_release_notes: true
```

### Artifacts

`upload-artifact` stores files that can be downloaded from the GitHub Actions UI:

- `rustshot-windows-x64-portable`: the standalone `.exe`
- `rustshot-windows-x64-installer`: the Inno Setup installer

### GitHub Release

`softprops/action-gh-release@v2` automatically creates a release when a tag is pushed:

- It attaches the configured files.
- `generate_release_notes: true` generates a changelog from commits.

---

## 5. Caching

### Why cache

Without a cache, every build downloads and compiles all dependencies from scratch, which commonly takes two to five minutes. With a cache, only the delta must be rebuilt.

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry    # Downloaded crate sources
      ~/.cargo/git         # Git dependencies
      target               # Compiled dependencies
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.toml') }}
```

The key includes a hash of the manifest. When dependencies do not change, the
cache remains valid. `Cargo.lock` would be the more precise key, but RustShot
does not track it (see `.gitignore`), and `hashFiles` over a file that is not in
the checkout yields one constant key that never invalidates.

### When not to cache

For CI checks, as opposed to release builds, caching is often not worth the added complexity:

- Parallel jobs may already be fast.
- A cache can hide dependency problems.
- Cache invalidation can introduce difficult-to-diagnose failures.

---

## 6. Cross-platform builds

### Matrix strategy

```yaml
test:
  strategy:
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]
  runs-on: ${{ matrix.os }}
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test
```

### Conditional steps

```yaml
- name: Install Linux dependencies
  if: runner.os == 'Linux'
  run: sudo apt-get install -y libxcb1-dev ...
```

### Conditional compilation in Rust

```yaml
# CI covers every platform, while the code uses cfg attributes:
```

```rust
#[cfg(target_os = "windows")]
fn register_hotkey() { ... }

#[cfg(not(target_os = "windows"))]
fn register_hotkey() {
    // No operation is needed on Linux or macOS.
}
```

---

## 7. Common problems

### Formatting

**Problem**: CI fails at `cargo fmt --check` even though formatting passes locally.

**Solution**: Different rustfmt versions may be in use. Pin the toolchain:

```yaml
- uses: dtolnay/rust-toolchain@stable
```

Or select a specific version:

```yaml
- uses: dtolnay/rust-toolchain@1.77.0
```

Run `cargo fmt` before committing.

### Linux system dependencies

**Problem**: `cargo test` fails with `cannot find -lxcb`.

**Solution**: Install the development packages:

```yaml
- run: |
    sudo apt-get update
    sudo apt-get install -y libxcb1-dev libgtk-3-dev ...
```

To identify the required packages, read the linker error and search with `apt-file search libXXX.so`.

### Windows and Inno Setup

**Problem**: `iscc` is not found.

**Solution**: `iscc` is preinstalled on GitHub's `windows-latest` runner. To request a particular version, use:

```yaml
- uses: jrsoftware/iscc-action@v1
```

### Secrets

```yaml
# Never hard-code tokens in a workflow.
- run: something --token ${{ secrets.MY_TOKEN }}
```

Configure secrets under **Settings → Secrets and variables → Actions**.

---

## New workflow checklist

- [ ] `on:` uses the correct triggers
- [ ] `runs-on:` uses the correct operating system
- [ ] `dtolnay/rust-toolchain` installs the Rust toolchain
- [ ] Linux system dependencies are installed
- [ ] `cargo fmt --check`, `cargo lint`, and `cargo test` run
- [ ] Release builds use an appropriate cache
- [ ] `upload-artifact` publishes build artifacts
- [ ] Secrets are read through `${{ secrets.XXX }}`
