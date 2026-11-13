# Inno Setup And Release Pipeline Lesson

This lesson explains two concrete parts of the `RustShot` release process:

1. how the Windows installer is built with Inno Setup
2. how the GitHub Actions release pipeline turns a tag into downloadable artifacts

It is intentionally tied to this repository, not written as a generic article.

## 1. Release outputs in this project

`RustShot` publishes two Windows deliverables:

- portable binary: `rustshot.exe`
- installer: `RustShotSetup.exe`

That split is visible in [README.md](../README.md), [installer.iss](../installer.iss), and [.github/workflows/release.yml](../.github/workflows/release.yml).

## 2. What Inno Setup is responsible for

Inno Setup does not compile Rust code.

Its job starts after the release `.exe` already exists. It is responsible for:

- copying the compiled binary into the install directory
- creating Start Menu entries
- adding autorun registry entries
- creating uninstall metadata
- optionally launching the app after installation
- packaging everything into one installer executable

In this repository, that logic lives in [installer.iss](../installer.iss).

## 3. Reading `installer.iss`

### `[Setup]`

This is the installer header and global behavior:

- `AppName=RustShot`
- `AppVersion={{VERSION}}`
- `DefaultDirName={autopf}\RustShot`
- `OutputDir=Output`
- `OutputBaseFilename=RustShotSetup`
- `PrivilegesRequired=lowest`

The key design choice here is `PrivilegesRequired=lowest`.

That means:

- install happens in user context
- no admin prompt is required
- the app can register `HKCU` autorun cleanly

For `RustShot`, this matches the tray-style per-user startup model.

### `[Languages]`

The installer UI is available in:

- English
- Russian

This is useful because the app docs are also bilingual.

### `[Files]`

The installer currently ships one main payload:

```iss
Source: "target\release\rustshot.exe"; DestDir: "{app}"; Flags: ignoreversion
```

That line is why the release workflow copies the MSVC build into `target\release` before running `iscc`.

### `[Icons]`

These create the Start Menu entries:

- launch `RustShot`
- run the uninstaller

### `[Registry]`

The installer adds:

`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\RustShot`

That is what causes the tray app to start after login.

The `uninsdeletevalue` flag is important because it removes the autorun entry on uninstall.

### `[Run]`

After installation, the installer can launch the app immediately:

- useful for tray applications
- skipped in silent mode because of `skipifsilent`

## 4. Local installer build flow

The local flow is:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
New-Item -ItemType Directory -Force -Path target\release | Out-Null
Copy-Item target\x86_64-pc-windows-msvc\release\RustShot.exe target\release\rustshot.exe
$version = (Get-Content Cargo.toml | Select-String '^version = "(.*)"').Matches.Groups[1].Value
(Get-Content installer.iss).Replace('{{VERSION}}', $version) | Set-Content installer.iss
iscc installer.iss
```

Important detail:

`installer.iss` expects `target\release\rustshot.exe` and `AppVersion={{VERSION}}`, while the Rust build step produces `target\x86_64-pc-windows-msvc\release\RustShot.exe`.

That path normalization and version substitution are deliberate and mirrored in CI.

## 5. Reading the release pipeline

The release workflow is in [release.yml](../.github/workflows/release.yml).

It triggers on:

- pushes to `main`
- tags matching `v*`
- manual `workflow_dispatch`

And declares `permissions: contents: write` so GitHub Actions can push/update the version tag and publish the GitHub Release.

The main steps are:

1. checkout source (`actions/checkout@v5`)
2. install stable Rust with the `x86_64-pc-windows-msvc` target
3. restore Cargo cache (`Cargo.lock` / `Cargo.toml` hash)
4. resolve `VERSION` and `RELEASE_TAG` (`v<version>`) from `Cargo.toml`
5. build the release binary (`target\x86_64-pc-windows-msvc\release\RustShot.exe`)
6. copy the binary into `target\release\rustshot.exe`
7. replace `{{VERSION}}` in `installer.iss` and run `iscc installer.iss`
8. upload the portable `.exe` and installer artifacts
9. update the `v<version>` release tag and publish both files to GitHub Releases

## 6. Why the pipeline uses the MSVC target

The workflow explicitly builds:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

This matters because:

- MSVC is the normal Windows target for shipping desktop apps
- mixing GNU and MSVC outputs can create packaging confusion
- CI is standardized around one output path and one runtime expectation

If local development uses a different target, you need to be careful not to package the wrong binary.

## 7. Artifact flow

The data flow is:

```text
Cargo build
  -> target\x86_64-pc-windows-msvc\release\RustShot.exe
  -> copied to target\release\rustshot.exe
  -> {{VERSION}} substituted in installer.iss from Cargo.toml
  -> consumed by installer.iss -> Output\RustShotSetup.exe
  -> both files uploaded to GitHub Actions artifacts
  -> both files attached to GitHub Release (tag v<version>)
```

That means there are two different “final” files:

- the direct binary from Cargo (`rustshot.exe`)
- the installer generated by Inno Setup (`RustShotSetup.exe`)

## 8. Typical failure points

### Wrong path in `[Files]`

If `installer.iss` points at the wrong path, `iscc` fails even though Rust compiled correctly.

### Unsubstituted `{{VERSION}}` template

`installer.iss` uses `AppVersion={{VERSION}}` so the version is always derived from `Cargo.toml`. Running `iscc` locally without replacing `{{VERSION}}` embeds the literal placeholder unless the substitution step is run first.

### Missing workflow permissions

Creating a GitHub Release and updating tags requires `permissions: contents: write` in `.github/workflows/release.yml`.

### Silent installer surprises

A `[Run]` section that feels correct interactively may behave differently under silent installation.

### Per-user vs per-machine mismatch

If you later switch to admin installs but keep `HKCU` startup assumptions, install behavior becomes inconsistent.

## 9. How to debug installer failures

When the installer is broken, isolate the stage:

1. Check whether Cargo built the binary.
2. Check whether the binary was copied to `target\release`.
3. Check whether `iscc installer.iss` succeeds locally.
4. Check whether the generated installer contains the expected files and shortcuts.
5. Check whether GitHub Release attached both artifacts.

That order matters because it separates Rust build failures from packaging failures.

## 10. How to improve this pipeline later

Reasonable next steps for `RustShot`:

- add code signing for both `rustshot.exe` and `RustShotSetup.exe`
- add installer smoke tests on CI
- verify registry autorun behavior after install
- add upgrade-path testing from older versions
- publish checksums alongside release artifacts

## 11. Practical rule of thumb

Treat the installer and the release pipeline as first-class product code.

If the app works but install, autorun, upgrade, or uninstall is broken, the release is still broken.
