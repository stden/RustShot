//! RustShot — screenshot and screen recording utility.
//!
//! # Application modes
//!
//! The binary runs in one of three modes depending on command-line flags:
//!
//! | Mode | Trigger | Description |
//! |------|---------|-------------|
//! | **Tray** | no flags | System tray icon with hotkeys (PrintScreen, Ctrl+Shift+R). |
//! | **CLI** | `--cli`, `--list`, `--output`, `--screen`, or `--format` | Single capture, then exit. |
//! | **Selection** | `--select` / `--select-record` (internal) | Child process spawned by tray mode for the overlay UI. |
//!
//! # Architecture overview
//!
//! ```text
//! main.rs          ──  thin executable launcher
//!   ├─ app/        ──  library application layer and startup routing
//!   ├─ cli.rs      ──  pure CLI helpers and app-mode decision policy
//!   ├─ capture.rs  ──  screenshot capture & file saving
//!   ├─ selection.rs ── interactive region selection overlay (winit + softbuffer)
//!   ├─ tray.rs     ──  tray icon & context menu construction
//!   └─ record.rs   ──  screen recording via Windows Media Foundation
//! ```
//!
//! On Windows the binary is built as a GUI subsystem app (no console window).
//! The `--select` child process performs the overlay selection, copies the
//! cropped image to the clipboard, and exits.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    rustshot::app::run();
}
