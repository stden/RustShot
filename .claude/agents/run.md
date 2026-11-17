---
name: run
description: Build and run RustShot in tray mode and report runtime status
---

Build RustShot in release mode and launch it as a tray application.

Steps:

1. **Build the release binary.**
   Run `cargo build --release` from the project root.
   If the build fails, show the error and stop.

2. **Check for an already running instance.**
   Use a platform-appropriate process check.
   On Windows, prefer `tasklist /FI "IMAGENAME eq rustshot.exe"`.
   If RustShot is already running, inform the user before launching another instance.

3. **Launch RustShot.**
   Run the release binary from `target/release/` in the background.
   Confirm it started by checking the process list.

4. **Report Windows autorun status when applicable.**
   On Windows, run:
   `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v RustShot`
   - If present, report the registered path.
   - If not present, report that autorun is currently disabled or not registered.
   Do not assume the app will add the entry automatically.

5. **Report** a short summary:
   - Binary location
   - Whether the app is running
   - Whether startup registration is active

Rules:
- Always build in release mode for the binary users will actually run.
- Do NOT modify the Windows registry directly — the app handles autorun itself.
- If the user wants to remove the startup entry, use:
  `reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v RustShot /f`
