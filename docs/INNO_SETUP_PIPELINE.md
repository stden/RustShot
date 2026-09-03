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
- `AppVersion={{VERSION}}` (the release workflow injects the Cargo version)
- `DefaultDirName={localappdata}\Programs\RustShot`
- `DisableDirPage=yes`
- `UsePreviousAppDir=no`
- `OutputDir=Output`
- `OutputBaseFilename=RustShotSetup`
- `PrivilegesRequired=lowest`

The key design choice here is `PrivilegesRequired=lowest`.

That means:

- install happens in user context
- no admin prompt is required
- the app can register `HKCU` autorun cleanly

For `RustShot`, this matches the tray-style per-user startup model.
`DisableDirPage=yes` and `UsePreviousAppDir=no` keep the setup package and
first-run self-installation on the same stable
`%LOCALAPPDATA%\Programs\RustShot` path instead of remembering a directory
chosen for an older setup.

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

That is what causes the tray app to start after login — on a fresh install. From
then on the running app owns the value: tray mode re-synchronises it on every
start from the `auto_start` setting (`src/app/autorun.rs::sync_autorun`, `HKCU`
only). Turning it off is a Settings-dialog operation, not a registry edit — see
[`docs/TUTORIAL.md`](TUTORIAL.md#-step-8--auto-start-and-where-settings-live).

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
Copy-Item target\x86_64-pc-windows-msvc\release\rustshot.exe target\release\rustshot.exe
$rustShotVersion = (Get-Content Cargo.toml | Select-String '^version = "(.*)"').Matches.Groups[1].Value
$versionedInstallerScript = 'installer.versioned.iss'
(Get-Content installer.iss).Replace('{{VERSION}}', $rustShotVersion) | Set-Content $versionedInstallerScript
try {
    iscc $versionedInstallerScript
    if ($LASTEXITCODE -ne 0) { throw "ISCC failed with exit code $LASTEXITCODE" }
} finally {
    Remove-Item $versionedInstallerScript -ErrorAction SilentlyContinue
}
```

Important detail:

`installer.iss` is a template: pass only a generated copy with `{{VERSION}}`
replaced to Inno Setup. It expects `target\release\rustshot.exe`, while the
Rust build step produces
`target\x86_64-pc-windows-msvc\release\rustshot.exe`.

Both the version substitution and path normalization are mirrored in CI.

## 5. Reading the release pipeline

The release workflow is in [release.yml](../.github/workflows/release.yml).

It triggers on:

- tags matching `v*`
- manual `workflow_dispatch`

The main steps are:

1. checkout source
2. install stable Rust with the `x86_64-pc-windows-msvc` target
3. restore Cargo cache
4. build the release binary
5. copy the binary into `target\release`
6. substitute the `Cargo.toml` version into `installer.iss` in the CI checkout
7. invoke ISCC only after that in-place substitution
8. upload the portable `.exe` artifact
9. upload the installer artifact
10. if running from a tag, publish both files to GitHub Releases

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
Cargo.toml version + installer.iss template
  -> version-substituted installer script
Cargo build
  -> target\x86_64-pc-windows-msvc\release\rustshot.exe
  -> copied to target\release\rustshot.exe
  -> consumed by the version-substituted script
  -> Output\RustShotSetup.exe
  -> both files uploaded to GitHub Actions artifacts
  -> both files attached to GitHub Release
```

That means there are two different “final” files:

- the direct binary from Cargo
- the installer generated by Inno Setup

## 8. Typical failure points

### Wrong path in `[Files]`

If `installer.iss` points at the wrong path, `iscc` fails even though Rust compiled correctly.

### Version drift

The release workflow reads the version from `Cargo.toml` and replaces the
`{{VERSION}}` placeholder before compiling the installer. A local build that
skips that substitution fails instead of silently packaging a different
version.

### Broken release but successful artifact build

The workflow can still build artifacts during `workflow_dispatch`, but GitHub Release creation only happens for tag refs.

### Silent installer surprises

A `[Run]` section that feels correct interactively may behave differently under silent installation.

### Per-user vs per-machine mismatch

If you later switch to admin installs but keep `HKCU` startup assumptions, install behavior becomes inconsistent.

## 9. How to debug installer failures

When the installer is broken, isolate the stage:

1. Check whether Cargo built the binary.
2. Check whether the binary was copied to `target\release`.
3. Generate the versioned script and check whether
   `iscc installer.versioned.iss` succeeds locally.
4. Check whether the generated installer contains the expected files and shortcuts.
5. Check whether GitHub Release attached both artifacts.

That order matters because it separates Rust build failures from packaging failures.

## 10. How to improve this pipeline later

Reasonable next steps for `RustShot`:

- share the existing version-substitution step between local builds and CI
- add code signing for both `rustshot.exe` and `RustShotSetup.exe`
- add installer smoke tests on CI
- verify registry autorun behavior after install
- add upgrade-path testing from older versions
- publish checksums alongside release artifacts

## 11. Practical rule of thumb

Treat the installer and the release pipeline as first-class product code.

If the app works but install, autorun, upgrade, or uninstall is broken, the release is still broken.
