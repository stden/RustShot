//! RustShot — screenshot and screen recording library.
//!
//! # Data-flow overview
//!
//! ```text
//!   CLI / Tray hotkey
//!         │
//!         ▼
//!   ┌───────────┐   PrintScreen    ┌──────────────┐     ┌──────────────┐
//!   │  app.rs   │ ─────────────▶   │ selection.rs  │ ──▶ │  capture.rs  │
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
//! - **Settings JSON is hand-rolled — no `serde`.** The settings schema is tiny
//!   and fixed, so `settings.rs` parses and formats it by hand rather than pull
//!   `serde` in for five fields. (The MCP server in `mcp.rs` is the one place
//!   `serde_json` is used, because it parses arbitrary JSON-RPC from clients —
//!   the opposite situation, where hand-rolling would be fragile.)
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

pub mod app;
pub mod capture;
pub mod cli;
pub mod mcp;
pub mod record;
pub mod selection;
pub mod settings;
#[cfg(windows)]
mod settings_dialog;
pub mod tray;
pub mod window;

#[cfg(test)]
mod main_tests;
#[cfg(test)]
mod test_support;
