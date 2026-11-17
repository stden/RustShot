//! RustShot — screenshot and screen recording library.
//!
//! # Data-flow overview
//!
//! ```text
//!   main.rs routes CLI flags, tray menu actions, and global hotkeys:
//!
//!   PrintScreen      -> --select child -> selection.rs -> region clipboard
//!   Alt+PrintScreen  -> capture.rs -> clipboard, then background file save
//!   Ctrl+Shift+R     -> --select-record child -> selection.rs -> record.rs -> MP4
//!   CLI capture      -> capture.rs -> image file and printed output path
//!
//!   cli.rs           Pure routing, format, and output policy
//!   tray.rs          System tray icon and context menu
//!   settings.rs      JSON configuration persistence
//!   portal.rs        Linux Wayland full-desktop CLI capture backend
//!
//!   Binary-private modules own platform UI, child processes, and clipboard
//!   lifetime. On Linux, the tray receives a region image through the child's
//!   stdout pipe and keeps its clipboard owner alive. Windows and macOS
//!   selection children copy directly to the system clipboard.
//! ```
//!
//! # Key design decisions
//!
//! - **No `serde` dependency.** Settings JSON is tiny and stable, so a
//!   dedicated parser/formatter (`settings.rs`) handles the fixed schema.
//!   Escaping, malformed input, and round trips have regression coverage.
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

#[cfg(windows)]
mod gdi;

pub mod capture;
pub mod cli;
#[cfg(target_os = "linux")]
pub mod portal;
pub mod record;
pub mod selection;
pub mod settings;
pub mod tray;
