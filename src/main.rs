//! Executable launcher for RustShot.
//!
//! Mode routing and dependency construction belong to `rustshot::app`.
//! Architecture diagrams and ownership rules live in the library rustdoc
//! (`src/lib.rs`), so the executable cannot acquire a second module tree.
//! On Windows this uses the GUI subsystem and does not open a console.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    rustshot::app::run();
}
