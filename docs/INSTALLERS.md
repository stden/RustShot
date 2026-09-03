# Rust Installer Tutorial

This short tutorial explains how to package a Rust desktop application as a Windows installer, using `RustShot` as the concrete example.

## 1. What Rust does and what the installer does

Rust builds your application binary:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

An installer tool then:

- copies the built `.exe` into the target install directory
- creates Start Menu shortcuts
- optionally writes registry keys
- optionally launches the app after install
- provides uninstall support

In `RustShot`, Rust builds `rustshot.exe`, and Inno Setup turns it into `RustShotSetup.exe`.

## 2. Why this project uses Inno Setup

`RustShot` uses [installer.iss](../installer.iss) and Inno Setup because it is a good fit for a small Windows desktop app:

- simple script format
- easy Start Menu and registry integration
- easy silent install support
- no need to write installer logic in Rust

If you need MSI specifically, WiX is a better fit. If you need a straightforward `.exe` installer, Inno Setup is usually simpler.

## 3. Files involved in RustShot

The installer flow in this repository is built around these files:

- [installer.iss](../installer.iss) — the installer definition
- [.github/workflows/release.yml](../.github/workflows/release.yml) — CI pipeline that builds the binary and installer
- [README.md](../README.md) — user-facing download instructions

## 4. Minimal local workflow

Install the Rust target:

```powershell
rustup target add x86_64-pc-windows-msvc
```

Build the release binary:

```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

Prepare the path expected by Inno Setup:

```powershell
New-Item -ItemType Directory -Force -Path target\release | Out-Null
Copy-Item target\x86_64-pc-windows-msvc\release\rustshot.exe target\release\rustshot.exe
```

Generate a versioned copy of the installer script, then build that copy:

```powershell
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

Output:

- portable binary: `target\x86_64-pc-windows-msvc\release\rustshot.exe`
- installer: `Output\RustShotSetup.exe`

## 5. Anatomy of `installer.iss`

### `[Setup]`

Defines product metadata and installer behavior:

- `AppName`
- `AppVersion`
- `DefaultDirName`
- `DisableDirPage`
- `UsePreviousAppDir`
- `OutputBaseFilename`
- `PrivilegesRequired`

In `RustShot`, `PrivilegesRequired=lowest` is important because the app writes
autorun under `HKCU`, not machine-wide `HKLM`. The directory page is disabled so
the installer uses the same fixed per-user path as first-run self-installation.

### `[Files]`

Lists what gets installed:

```iss
[Files]
Source: "target\release\rustshot.exe"; DestDir: "{app}"; Flags: ignoreversion
```

For a larger app, this section may also include:

- icons
- config templates
- helper DLLs
- license/readme files

### `[Icons]`

Creates Start Menu entries:

```iss
[Icons]
Name: "{group}\RustShot"; Filename: "{app}\rustshot.exe"
Name: "{group}\Uninstall RustShot"; Filename: "{uninstallexe}"
```

### `[Registry]`

Writes registry values during install:

```iss
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "RustShot"; ValueData: """{app}\rustshot.exe"""; Flags: uninsdeletevalue
```

This is how `RustShot` auto-starts from the tray after login.

The installer is not the only writer of that value: tray mode re-synchronises it
on every start from the `auto_start` setting (`src/app/autorun.rs::sync_autorun`,
`HKCU` only — never `HKLM`, which is what keeps RustShot elevation-free). So
deleting the value by hand does not stick, and the supported off switch is the
tray's Settings dialog — see
[`docs/TUTORIAL.md`](TUTORIAL.md#-step-8--auto-start-and-where-settings-live).

The installer is not the only writer of that value: tray mode re-synchronises it
on every start from the `auto_start` setting (`src/app/autorun.rs::sync_autorun`,
`HKCU` only — never `HKLM`, which is what keeps RustShot elevation-free). So
deleting the value by hand does not stick, and the supported off switch is the
tray's Settings dialog — see
[`docs/TUTORIAL.md`](TUTORIAL.md#-step-8--auto-start-and-where-settings-live).

### `[Run]`

Runs the app after install:

```iss
[Run]
Filename: "{app}\rustshot.exe"; Description: "Launch RustShot (system tray)"; Flags: nowait postinstall skipifsilent
```

## 6. A minimal installer template for another Rust app

```iss
[Setup]
AppName=MyRustApp
AppVersion=0.1.0
DefaultDirName={autopf}\MyRustApp
DefaultGroupName=MyRustApp
OutputDir=Output
OutputBaseFilename=MyRustAppSetup
Compression=lzma
SolidCompression=yes
PrivilegesRequired=lowest
WizardStyle=modern

[Files]
Source: "target\release\my_rust_app.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\MyRustApp"; Filename: "{app}\my_rust_app.exe"
Name: "{group}\Uninstall MyRustApp"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\my_rust_app.exe"; Description: "Launch MyRustApp"; Flags: nowait postinstall skipifsilent
```

That is enough for many utility-style Rust apps.

## 7. How to evolve the installer

Typical next steps:

- add app icon
- add `AppPublisher`, `AppSupportURL`, `AppPublisherURL`
- add file associations
- add desktop shortcut as an optional task
- add per-user vs per-machine install choice
- sign both the `.exe` and the installer

## 8. How CI builds the installer

The release workflow in [release.yml](../.github/workflows/release.yml):

1. checks out the repo
2. installs the MSVC target
3. builds the Rust binary
4. copies it into `target\release`
5. substitutes the `Cargo.toml` version into `installer.iss` in the CI checkout
6. invokes ISCC only after that in-place substitution
7. uploads both the portable `.exe` and `RustShotSetup.exe`

That split is useful because many users want both:

- a portable executable
- a full installer

## 9. How to test an installer correctly

Do not stop at “it builds”.

Check these cases:

1. Fresh install on a clean Windows user profile.
2. Launch from Start Menu.
3. App writes files into expected user directories.
4. Autorun entry is created correctly.
5. Silent install does not break.
6. Uninstall removes shortcuts and uninstall metadata.
7. Upgrade over an older version behaves cleanly.

For `RustShot`, also verify:

- tray app launches after install
- `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry is correct
- uninstall removes that autorun entry

## 10. Common mistakes

- Building GNU target but packaging as if it were MSVC.
- Pointing `[Files]` at the wrong output directory.
- Using admin install privileges when the app is designed for per-user startup.
- Forgetting to test uninstall.
- Assuming `cargo build --release` alone creates an installer.
- Passing the unresolved `{{VERSION}}` template to Inno Setup.

## 11. Version injection in RustShot

[`installer.iss`](../installer.iss) contains `AppVersion={{VERSION}}`. The
release workflow replaces that placeholder with the version from `Cargo.toml`
before invoking Inno Setup, so the package and executable stay aligned. A local
installer build must perform the same substitution on a temporary copy.

## 12. Next lesson

For a deeper walkthrough focused specifically on `installer.iss` structure and the GitHub Actions release flow, see [INNO_SETUP_PIPELINE.md](INNO_SETUP_PIPELINE.md).
