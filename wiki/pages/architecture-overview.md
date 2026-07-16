[← Back to Index](../index.md)

# Architecture Overview

`RustShot` is designed as a modular desktop utility, separating concerns into distinct layers to manage complexity despite its small footprint.

## Core Modules
- **`main.rs`**: Entry point, CLI parsing, startup routing, and tray event loop.
- **`cli.rs`**: Pure CLI helper logic, output-format policy, and app-mode decision rules.
- **`capture.rs`**: Screen capture logic and static image saving.
- **`selection.rs`**: Interactive region selection overlay, multi-monitor coordination, and DPI-aware coordinate handling.
- **`record.rs`**: Screen recording pipeline (Windows Media Foundation).
- **`settings.rs`**: Persistent state management.
- **`tray.rs`**: System tray icon, context menu, and global hotkey integration.

## Key Principles
* **Narrow Responsibility**: Each function and module has a clear, specific role.
* **Pure Logic vs. Platform Specific**: Separation of cross-platform logic from OS-dependent paths.
* **Explicit Boundary Checks**: Validating coordinates, file paths, and system states at module boundaries.
* **Testable Helpers**: Complex logic is extracted into small, testable helper functions.

## Hidden Complexity
The application must balance multiple states (tray active, selection ongoing, recording in progress) and navigate external factors like display configurations and filesystem permissions.

## 📋 Project Specifications (Codev)
* [Spec 0001: RustShot Utility](../../codev/specs/0001-rustshot-screenshot-utility.md)
* [Spec 0003: Settings and Runtime Configuration](../../codev/specs/0003-settings-and-runtime-configuration.md)

## See Also
* [Source: Screenshot Architecture](../pages/source-architecture.md)
* [DPI and Coordinate Systems](dpi-coordinate-systems.md)
