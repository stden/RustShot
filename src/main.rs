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
//! main.rs          ──  crate root: module tree and startup routing
//!   ├─ app/        ──  application layer (autorun, CLI mode, runtime, tray app)
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

mod capture;
mod cli;
mod mcp;
mod record;
mod selection;
mod settings;
#[cfg(windows)]
mod settings_dialog;
mod tray;
mod window;

mod app;
#[cfg(test)]
mod main_tests;

use app::*;
use clap::Parser;
use log::{debug, error, info};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    init_logging();
    info!("RustShot v{} starting", env!("CARGO_PKG_VERSION"));

    let args = Args::parse();
    let format_explicit = cli_format_flag_present_in(std::env::args_os());
    debug!("CLI args: {:?}", args);

    // The MCP server is its own top-level mode: it owns stdin/stdout for the
    // JSON-RPC transport, so it must run before any tray/CLI routing.
    if args.mcp {
        info!("Starting MCP server on stdio");
        if let Err(e) = mcp::run() {
            error!("MCP server error: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    match decide_app_mode_with_format_explicit(&args, format_explicit) {
        cli::AppMode::SelectRecord => run_select_record(),
        cli::AppMode::Selection => run_selection(),
        cli::AppMode::Cli => run_cli(args),
        cli::AppMode::Tray => {
            #[cfg(windows)]
            sync_autorun(settings::load_settings().auto_start);

            let event_loop = EventLoop::new().expect("Failed to create event loop");
            event_loop.set_control_flow(ControlFlow::Wait);

            let mut app = match RustShotApp::new() {
                Ok(app) => app,
                Err(e) => {
                    error!("Tray startup error: {}", e);
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            info!("Entering event loop");
            event_loop.run_app(&mut app).expect("Event loop error");
        }
    }
}
