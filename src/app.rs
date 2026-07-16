//! Application layer for the RustShot binary.
//!
//! `main.rs` stays a thin crate root; the actual application logic lives in
//! the parts below. Each part is re-exported wholesale so `main.rs` and the
//! unit tests in `crate::main_tests` can keep referring to items unqualified,
//! exactly as they did when everything lived in `main.rs`.
//!
//! - [`autorun`] — Windows auto-start registry integration
//! - [`cli_mode`] — CLI argument model, format policy, and one-shot capture flow
//! - [`runtime`] — logging setup and the selection/recording child-process modes
//! - [`tray_app`] — tray icon, global hotkeys, and the winit event-loop app

pub(crate) mod autorun;
pub(crate) mod cli_mode;
pub(crate) mod runtime;
pub(crate) mod tray_app;

pub(crate) use autorun::*;
pub(crate) use cli_mode::*;
pub(crate) use runtime::*;
pub(crate) use tray_app::*;
