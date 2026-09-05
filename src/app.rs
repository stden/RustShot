//! Composition root and presentation coordination for the RustShot binary.
//!
//! [`run`] chooses the process mode and connects presentation adapters to the
//! capture, recording, settings, and [`crate::application`] workflows. Shared
//! workflows must not import this module: otherwise a dialog calling a startup
//! use case pulls the composition root back into the presentation dependency graph.
//!
//! - `cli_mode` — CLI argument model, format policy, and one-shot capture flow
//! - `runtime` — logging setup and the selection/recording child-process modes
//! - `tray_app` — tray icon, global hotkeys, and the winit event-loop app

use clap::Parser;
use log::{debug, error, info};
use winit::event_loop::{ControlFlow, EventLoop};

pub(crate) mod cli_mode;
pub(crate) mod runtime;
pub(crate) mod tray_app;

pub(crate) use crate::application::autorun::*;
#[doc(hidden)]
pub use crate::application::autorun::{
    benchmark_sync_autorun_at_with_handlers, benchmark_sync_current_exe_autorun_at_with_handlers,
};
#[doc(hidden)]
pub use crate::application::capture::benchmark_capture_active_window_outputs_with_handlers;
#[cfg(test)]
pub(crate) use crate::application::capture::capture_active_window_outputs_with_handlers;
pub(crate) use cli_mode::*;
pub(crate) use runtime::*;
pub(crate) use tray_app::*;

/// Start RustShot and route the parsed command line to the selected app mode.
///
/// Keeping this entry point in the library lets the executable remain a thin
/// launcher instead of compiling a second copy of every shared module.
pub fn run() {
    init_logging();
    // Anchor the capture timeline at process start: for `--select` and
    // `--select-record` this process *is* one capture, so every milestone offset
    // is measured from the moment the hotkey spawned us.
    crate::timing::start_capture_timeline();
    info!("RustShot v{} starting", env!("CARGO_PKG_VERSION"));

    let args = Args::parse();
    let format_explicit = cli_format_flag_present_in(std::env::args_os());
    debug!("CLI args: {:?}", args);

    // The MCP server is its own top-level mode: it owns stdin/stdout for the
    // JSON-RPC transport, so it must run before any tray/CLI routing.
    if args.mcp {
        info!("Starting MCP server on stdio");
        if let Err(e) = crate::mcp::run() {
            error!("MCP server error: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    match decide_app_mode_with_format_explicit(&args, format_explicit) {
        crate::cli::AppMode::SelectRecord => run_select_record(),
        crate::cli::AppMode::Selection => run_selection(),
        crate::cli::AppMode::Cli => run_cli(args),
        crate::cli::AppMode::Tray => {
            #[cfg(windows)]
            match sync_autorun(crate::settings::load_settings().auto_start) {
                Ok(StartupAction::Continue) => {}
                Ok(StartupAction::Exit) => {
                    info!("Installed RustShot for Windows startup; exiting source process");
                    return;
                }
                Err(error) => error!("Windows startup installation failed: {error}"),
            }

            let event_loop = EventLoop::<RustShotEvent>::with_user_event()
                .build()
                .expect("Failed to create event loop");
            event_loop.set_control_flow(ControlFlow::Wait);
            install_event_handlers(event_loop.create_proxy());

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
