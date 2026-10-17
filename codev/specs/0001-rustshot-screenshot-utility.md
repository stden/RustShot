# Specification: RustShot — Screenshot & Screen Recording Utility

## Metadata
- **ID**: spec-2026-03-13-rustshot-screenshot-utility
- **Status**: approved
- **Created**: 2026-03-13

## Clarifying Questions Asked

1. **Q**: What is the primary use case for RustShot?
   **A**: Quick screenshot capture on Windows with tray icon, hotkey (PrintScreen), and region selection via mouse. Also supports screen recording (MP4/H.264).

2. **Q**: What platforms are targeted?
   **A**: Windows is the primary platform. Linux and macOS are secondary (CI builds, no tray/hotkey there yet).

3. **Q**: How should screenshots be delivered to the user?
   **A**: Region selection copies to clipboard. CLI mode saves to file (PNG/JPG/BMP). Tray mode is the default.

4. **Q**: Is multi-monitor support required?
   **A**: Yes. Users can select a specific screen via `--screen N`, and the selection overlay spans the virtual desktop.

5. **Q**: What about screen recording?
   **A**: Region-based recording via Ctrl+Shift+R. Uses Windows Media Foundation (H.264 MP4). Start/stop toggle from tray or hotkey.

## Problem Statement

Users on Windows need a lightweight, fast screenshot and screen recording tool that:
- Lives in the system tray and responds instantly to hotkeys
- Allows precise region selection with visual feedback (crosshair + selection border)
- Copies screenshots to clipboard for immediate paste
- Supports saving in multiple formats (PNG, JPG, BMP)
- Records screen regions as MP4 video
- Works seamlessly across multiple monitors

Existing tools are either heavy (Snagit, ShareX), lack tray integration, or don't provide region selection with clipboard support out of the box.

## Current State

### Architecture

RustShot is a single-binary Rust application with six top-level modules:

| Module | File | Responsibility |
|--------|------|---------------|
| **main** | `src/main.rs` | CLI parsing (clap), app mode routing, tray event loop, hotkey handling |
| **capture** | `src/capture.rs` | Screenshot capture, format conversion, file saving, path resolution |
| **selection** | `src/selection.rs` | Fullscreen overlay window, mouse-driven region selection, virtual desktop compositing |
| **tray** | `src/tray.rs` | System tray icon creation, context menu, icon pixel generation |
| **record** | `src/record.rs` | Screen recording via Windows Media Foundation, RGBA→BGRA conversion, MP4 encoding |
| **settings** | `src/settings.rs` | Lightweight persisted settings and settings-file editing flow |

### Application Modes

```
rustshot                           → Tray mode (default)
rustshot --cli                     → CLI single-capture mode
rustshot --window                  → Active-window capture (Windows only)
rustshot --list                    → List available screens
rustshot --select                  → Internal: selection subprocess
rustshot --select-record           → Internal: recording selection subprocess
rustshot -o file.png -f jpg -s 1   → CLI with options
```

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `screenshots` | 0.8 | Cross-platform screen capture |
| `image` | 0.24 | Image processing, format conversion |
| `clap` | 4 | CLI argument parsing with derive |
| `tray-icon` | 0.21 | System tray icon and menu |
| `global-hotkey` | 0.7 | Global hotkey registration (PrintScreen, Ctrl+Shift+R) |
| `winit` | 0.30 | Window creation for selection overlay |
| `softbuffer` | 0.4 | Software rendering for selection overlay |
| `arboard` | 3 | Clipboard access (copy image) |
| `chrono` | 0.4 | Timestamp generation for filenames |
| `dirs` | 6 | OS-specific directories (Pictures and config paths) |
| `windows` | 0.62 | Windows Media Foundation and Win32 APIs (Windows-only) |

### Data Flow

#### Screenshot Flow (Tray Mode)
```
PrintScreen hotkey
  → spawn child process: rustshot --select
    → capture virtual desktop (all screens composited)
    → show fullscreen overlay with screenshot as background
    → user drags mouse to select region
    → crop selected region from composite image
    → copy cropped RGBA image to clipboard
  → child exits
```

#### Screenshot Flow (CLI Mode)
```
rustshot --cli -o file.png -f png -s 0
  → resolve screen index (validate bounds)
  → capture specified screen
  → resolve output path (explicit or timestamped default)
  → ensure parent directory exists
  → save image in requested format
  → print path to stdout
```

#### Recording Flow
```
Ctrl+Shift+R hotkey
  → spawn child process: rustshot --select-record
    → capture virtual desktop for overlay
    → user selects region
    → determine which screen the region falls on
    → convert to screen-local coordinates
  → start RecordingSession::start(screen_x, screen_y, region, 30fps)
    → background thread: capture screen → crop → RGBA→BGRA → write H.264 frames
  → poll for stop-flag file
Ctrl+Shift+R again
  → write stop-flag file
  → child process stops recording, finalizes MP4
  → child exits
```

### Selection Overlay Details

The selection overlay is a fullscreen borderless window that:
1. Captures all screens into a single RGBA composite image
2. Displays the composite as the window background via `softbuffer`
3. Shows a **red crosshair** at cursor position (before drag starts)
4. Shows a **3px cyan border** around the selection rectangle (during drag)
5. Minimum selection size: **6×6 pixels** (anything ≤5×5 is rejected)
6. **Left click + drag** = select region
7. **Right click** or **Escape** = cancel
8. On successful selection: crops the composite and copies to clipboard

### File Organization

```
Pictures/RustShot/                    ← screenshots (auto-created)
Pictures/RustShot/recordings/         ← recordings (auto-created)
<config-dir>/RustShot/settings.json   ← lightweight persisted settings
```

Default filename patterns:
- Screenshots: `screenshot_YYYYMMDD_HHMMSS.{png,jpg,bmp}`
- Recordings: `recording_YYYYMMDD_HHMMSS.mp4`

### Error Handling

All functions return `Result<T, String>`. Error messages are human-readable:
- `"No screens detected"` — no screens found by `screenshots` crate
- `"Screen index N is out of range (0-M)"` — invalid screen index
- `"Failed to create directory: ..."` — filesystem error
- `"Selection cancelled"` — user pressed Escape or right-clicked
- `"Failed to capture screen: ..."` — screen capture API error

### Testing Strategy

The project currently has **433 automated tests** in the working tree.

Coverage is concentrated in:
- `main.rs` for CLI parsing, mode routing, tray helpers, and clipboard flow
- `capture.rs` for formats, path resolution, screen selection, and save pipeline
- `selection.rs` for region building, multi-monitor math, and selection state
- `tray.rs` for icon generation and menu constants
- `record.rs` for frame conversion, timing helpers, and recording paths
- `settings.rs` for settings defaults, JSON formatting, and parsing behavior

Key testing patterns:
- **Dependency injection** via closures (e.g., `capture_screenshot_with`, `run_cli_with_capture`, `run_selection_with_clipboard`)
- **No GUI tests** — all GUI-related code is tested through small pure functions extracted from the GUI layer
- **Boundary testing** — extensive tests for edge cases (screen index bounds, minimum selection size, file path handling)

### CI Pipeline

GitHub Actions with 3 jobs:
1. **Format** (`cargo fmt --check`) — Ubuntu
2. **Lint & Test Linux** (`cargo clippy` + `cargo test`) — Ubuntu with X11 dependencies
3. **Test Windows** (`cargo test`) — Windows

Release pipeline: tag `v*` triggers Windows build + GitHub Release with `rustshot.exe`.

### Windows-Specific Behavior

- **Autorun**: Tray-mode startup currently writes a user-level Run entry in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- **Console hidden**: `#![windows_subsystem = "windows"]` hides the console window
- **Recording**: Uses Windows Media Foundation (H.264 encoder) via the `windows` crate
- **Tray icon**: Custom 32×32 RGBA icon (blue camera body with white lens)

## Desired State

RustShot should continue to be a lightweight, fast, and reliable screenshot/recording tool. Future improvements should focus on:

1. **Stability**: Zero crashes in production use
2. **Performance**: Screenshot capture < 100ms, selection overlay at 60fps
3. **Test coverage**: Maintain >95% coverage on testable logic
4. **Cross-platform**: Linux and macOS should work for CLI mode at minimum
5. **Clean architecture**: Keep modules small and focused, logic testable without GUI

## Stakeholders
- **Primary Users**: Windows desktop users who need quick screenshot/recording capture
- **Secondary Users**: Developers who use CLI mode for automated screenshot workflows
- **Technical Team**: Rust developers contributing to the project
- **Business Owners**: Open-source community (MIT/Apache-2.0 license)

## Success Criteria
- [x] Screenshot capture works via PrintScreen hotkey
- [x] Region selection with visual feedback (crosshair + border)
- [x] Screenshots copied to clipboard
- [x] CLI mode with format/screen/output options
- [x] Multi-monitor support (virtual desktop compositing)
- [x] Screen recording via Ctrl+Shift+R
- [x] System tray with context menu
- [x] Current automated test suite passes locally (433 tests in the current tree)
- [x] CI pipeline on Windows and Linux
- [ ] All tests pass with >90% coverage on testable logic
- [ ] Performance benchmarks: capture < 100ms, overlay render < 16ms

## Constraints

### Technical Constraints
- Windows is the primary platform; tray and recording features are Windows-only
- Screen capture depends on `screenshots` crate (X11 on Linux, CoreGraphics on macOS)
- Recording uses Windows Media Foundation (no cross-platform recording yet)
- Selection overlay uses `softbuffer` for CPU rendering (no GPU acceleration)
- Single-binary architecture with a lightweight settings file in the OS config directory

### Business Constraints
- Open-source project: contributions welcome
- MIT/Apache-2.0 dual license
- No telemetry, no network access, no user data collection

## Assumptions
- Users have at least one screen connected
- On Windows, the `screenshots` crate can access the screen via GDI/DXGI
- Clipboard is available for writing (no locked clipboard scenarios)
- Filesystem is writable for saving screenshots/recordings

## Solution Approaches

### Approach 1: Current Architecture (Maintain & Improve)
**Description**: Keep the current 4-module architecture, improve test coverage, fix bugs incrementally.

**Pros**:
- Simple, well-understood codebase
- Fast compilation
- Easy to contribute to

**Cons**:
- No plugin system for extensibility
- Error handling with `String` is basic (no typed errors)

**Estimated Complexity**: Low
**Risk Level**: Low

### Approach 2: Library + Binary Split
**Description**: Extract core logic into a `rustshot-lib` crate, keep the binary thin.

**Pros**:
- Enables reuse (other tools can use the capture/selection library)
- Cleaner API boundaries
- Better testability

**Cons**:
- More complex project structure
- Workspace management overhead

**Estimated Complexity**: Medium
**Risk Level**: Low

## Open Questions

### Important (Affects Design)
- [x] Should recording support audio? — Not currently planned
- [x] Should there be a configuration file? — A lightweight JSON settings file now exists; richer settings UI can wait

### Nice-to-Know (Optimization)
- [ ] Could GPU-accelerated rendering improve selection overlay performance?
- [ ] Would `wgpu` be a better choice than `softbuffer` for the overlay?

## Performance Requirements
- **Screenshot capture**: < 100ms from hotkey press to clipboard
- **Selection overlay**: 60fps render during drag
- **Recording**: 30fps with < 5% CPU overhead
- **Memory**: < 100MB for single-screen capture
- **Startup**: < 500ms to tray icon visible

## Security Considerations
- No network access: all data stays local
- No configuration files with credentials
- Autorun registration uses user-level registry only (`HKCU`)
- No elevation required (runs as standard user)
- Clipboard access is write-only (doesn't read existing clipboard)

## Test Scenarios

### Functional Tests
1. **Happy path**: PrintScreen → select region → clipboard contains screenshot
2. **CLI capture**: `--cli -o file.png` → file exists with valid PNG data
3. **Multi-monitor**: `--screen 1` → captures the correct screen
4. **Format conversion**: `--format jpg` → file has JPEG data
5. **Cancel selection**: Right-click during selection → graceful exit, no clipboard change
6. **Recording start/stop**: Ctrl+Shift+R → select → record → Ctrl+Shift+R → valid MP4

### Edge Cases
1. Selection too small (≤5×5) → rejected, user retries
2. Screen index out of bounds → clear error message
3. Output directory doesn't exist → auto-created
4. No screens detected → error, no crash
5. Selection cancelled via Escape → no clipboard modification

### Non-Functional Tests
1. Capture latency < 100ms (measured in tests via timing)
2. Memory usage < 100MB for 1920×1080 capture
3. Current automated tests continue to pass under supported local and CI environments

## Dependencies
- **External Services**: None (fully offline)
- **Internal Systems**: Windows Media Foundation (for recording)
- **Libraries/Frameworks**: See Key Dependencies table above

## References
- [screenshots crate](https://crates.io/crates/screenshots) — cross-platform screen capture
- [clap derive](https://docs.rs/clap/latest/clap/_derive/) — CLI argument parsing
- [winit](https://docs.rs/winit/) — cross-platform windowing
- [softbuffer](https://docs.rs/softbuffer/) — CPU-based window rendering
- [Windows Media Foundation](https://learn.microsoft.com/en-us/windows/win32/medfound/) — video encoding

## Risks and Mitigation
| Risk | Probability | Impact | Mitigation Strategy |
|------|------------|--------|-------------------|
| `screenshots` crate breaks in update | Low | High | Pin version, test before updating |
| Windows API changes break recording | Low | Medium | Pin `windows` crate version, test on Windows CI |
| Selection overlay slow on high-DPI | Medium | Medium | Profile and optimize render loop |
| Clipboard locked by another app | Low | Low | Retry once, then show error |
| Multiple PrintScreen presses spawn many processes | Low | Medium | Already handled: check if selection process is running |

## Approval
- [x] Technical Lead Review
- [x] Stakeholder Sign-off

## Notes

### Module Interaction Map
```
main.rs
├── capture.rs      (CLI mode: capture & save)
├── selection.rs    (Tray mode: region selection → clipboard)
├── tray.rs         (Tray icon & menu creation)
├── record.rs       (Recording session management)
└── settings.rs     (Persisted settings and settings-file workflow)

selection.rs
└── screenshots     (virtual desktop capture)
└── softbuffer      (overlay rendering)
└── winit           (window management)

capture.rs
└── screenshots     (single-screen capture)
└── image           (format conversion & save)
└── dirs            (default save directory)

record.rs
└── windows         (Media Foundation H.264 encoding)
└── screenshots     (per-frame screen capture)
└── image           (frame cropping & padding)
```

### Key Design Decisions
1. **Child process for selection**: Avoids blocking the tray event loop; each selection runs in its own process
2. **Virtual desktop compositing**: All screens captured and stitched into one image for seamless multi-monitor selection
3. **Closure-based DI for testing**: Core functions accept closures for capture/save/clipboard, enabling full unit testing without GUI
4. **Stop-flag file for recording**: Simple IPC between parent (tray) and child (recorder) processes via filesystem

---

## Amendments

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
