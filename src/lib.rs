//! RustShot — screenshot and screen recording library.
//!
//! # Data-flow overview
//!
//! ```text
//!   CLI / Tray hotkey
//!         │
//!         ▼
//!   ┌───────────┐   PrintScreen    ┌──────────────┐     ┌──────────────┐
//!   │  main.rs  │ ─────────────▶   │ selection.rs  │ ──▶ │  capture.rs  │
//!   │ (routing) │   Ctrl+Shift+R   │  (overlay UI) │     │ (save image) │
//!   └───────────┘ ─────────────▶   └──────────────┘     └──────────────┘
//!         │                              │
//!         │                              ▼
//!         │                        ┌──────────────┐
//!         │                        │  record.rs   │
//!         │                        │ (H.264 MP4)  │
//!         │                        └──────────────┘
//!         │
//!         ├──▶  cli.rs          — pure CLI helpers + app-mode policy
//!         ├──▶  tray.rs         — system tray icon + context menu
//!         └──▶  settings.rs     — JSON config persistence
//!              settings_dialog.rs — native Win32 settings UI
//! ```
//!
//! # Key design decisions
//!
//! - **No `serde` dependency.** Settings JSON is tiny and stable, so a
//!   hand-rolled parser/formatter (`settings.rs`) avoids pulling in the
//!   serde + serde_json dependency tree (~30 crates).
//!
//! - **`Result<_, String>` everywhere.** The codebase is a small end-user
//!   tool, not a library consumed by other Rust code. `String` errors are
//!   human-readable, trivially composable with `format!`, and avoid the
//!   boilerplate of a custom error enum.
//!
//! - **`softbuffer` for the selection overlay.** The selection window needs
//!   to blit a full-screen screenshot into a framebuffer and draw a
//!   crosshair/rectangle on top. `softbuffer` provides a minimal
//!   CPU-rendered surface with no GPU driver dependency, keeping the binary
//!   small and portable.
//!
//! - **Child-process architecture.** Tray mode spawns `--select` /
//!   `--select-record` child processes so that the overlay window's event
//!   loop is fully isolated from the tray's event loop, preventing deadlocks
//!   and simplifying shutdown.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod capture;
pub mod cli;
pub mod record;
pub mod selection;
pub mod settings;
pub mod tray;
