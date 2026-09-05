//! RustShot — screenshot and screen recording library.
//!
//! # Layers and dependency direction
//!
//! `app` is the composition root: it selects a process mode, constructs the
//! event loops, and connects effects to workflows. Presentation code lives in
//! `app/{cli_mode,tray_app,runtime}`, `mcp`, `tray`, `settings_dialog`, and
//! the selection overlay. It translates user input and displays results.
//!
//! `application` owns shared startup and capture-output workflows, including
//! operation ordering and partial-failure policy. The `capture`, `record`,
//! `window`, and `settings` feature services provide capture, recording,
//! persistence, and window operations. Shared native capture, cursor, and startup
//! adapters live in `platform`; neither capture backend calls a feature service.
//! Feature-specific enumeration, image/video encoding, and screen metadata stay
//! with their feature owners (`window`, `capture`, `record/encoder`, and
//! `selection/desktop`). Native presentation code stays with its UI module.
//!
//! `pixels` contains shared buffer validation, cropping, padding, and channel
//! conversion. `domain` contains `Format`, `Fps`, typed coordinates,
//! `Region`, and pure monitor-layout rules; it uses only the standard library.
//! Recording-specific dimensions and timeline values stay in `record/domain`.
//!
//! Arrows below mean “depends on”; omitted arrows to shared domain values are
//! permitted. Feature services may use settings persistence, but settings never
//! imports a capture service. The overlay calls the same native monitor adapter
//! as screenshot and recording services.
//!
//! ```mermaid
//! flowchart TD
//!     Root["app: composition root"] --> UI["CLI / MCP / tray / settings dialog / overlay"]
//!     Root --> Workflows["application: startup and output delivery"]
//!     UI --> Workflows
//!     UI --> Services["capture / record / window / settings"]
//!     Workflows --> Native["platform: window / monitor / autorun / cursor"]
//!     Services --> Native
//!     UI -->|overlay desktop capture| Native
//!     Services --> Pixels["pixels: shared frame transforms"]
//!     Native --> Pixels
//!     Pixels --> Domain["domain: values and geometry"]
//!     Native --> Domain
//!     Services --> Domain
//!     UI --> Domain
//! ```
//!
//! This is a modular library, not a set of runtime plug-ins. Native resource
//! ownership remains explicit: window-generation leases, capture workers, and
//! recording sessions retain their existing cleanup behavior. Closure parameters
//! provide replaceable effects where a workflow needs them; pure geometry and
//! pixel transforms need no service objects.
//!
//! Existing paths such as `capture::Format`, `settings::Fps`, and
//! `selection::Region` re-export the shared types, preserving callers while
//! shared consumers import domain values directly. Public benchmark entry
//! points also remain available.
//!
//! # Interactive capture flow
//!
//! One invocation-time window identity accompanies delayed work. The application
//! workflow captures once, then attempts both clipboard and file delivery using
//! the same image. Clipboard failure still allows the file save to run.
//!
//! ```mermaid
//! sequenceDiagram
//!     participant User
//!     participant Tray as Tray controller
//!     participant Workflow as Application output workflow
//!     participant Native as Platform window capture
//!     participant Clipboard
//!     participant File as Image file writer
//!     User->>Tray: Hotkey or tray command
//!     Tray->>Tray: Retain invocation-time window identity
//!     Tray->>Workflow: Supply capture and output handlers
//!     Workflow->>Native: Capture validated window target
//!     Native-->>Workflow: One RGBA image or capture error
//!     opt Capture succeeded
//!         Workflow->>Clipboard: Copy image
//!         Clipboard-->>Workflow: Result
//!         Workflow->>File: Save the same image
//!         File-->>Workflow: Result
//!     end
//!     Workflow-->>Tray: Combined result
//! ```
//!
//! Region selection and recording still run in child processes. The tray owns
//! their lifetime; the recording stop-file path is shared by parent and child.
//!
//! # Code and test separation
//!
//! Production implementations live under `src/`. Unit test bodies and fixtures
//! live under `tests/unit/`, mounted into their owning modules with
//! `#[cfg(test)] #[path = "..."] mod tests;`. This preserves Rust's private
//! child-module access without publishing internals for tests. Small test-only
//! hooks at native and settings boundaries remain gated by `cfg(test)`; they
//! are absent from the production build.
//!
//! ```mermaid
//! flowchart LR
//!     Unit["tests/unit: private unit tests and fixtures"] -->|cfg test / path| LibraryTest["Library test build"]
//!     Source["src: production modules"] --> LibraryTest
//!     Source --> Library["Production library"]
//!     Integration["tests/*.rs: API and architecture checks"] --> Library
//!     Features["tests/features + tests/bdd"] --> Library
//!     Bench["benches: performance workloads"] --> Library
//!     Binary["main.rs: launcher"] --> Library
//! ```
//!
//! `cargo test` runs unit, integration, Gherkin, architecture, and rustdoc tests.
//! `tests/architecture.rs` parses Rust imports to reject reverse dependencies in
//! shared layers and prevents unit test bodies returning to `src/`. The
//! existing behavioral tests pin behavior during this restructuring; they are
//! not claimed as newly failing regression tests.
//!
//! Mermaid source lives beside the implementation. The documentation map
//! (`docs/README.md`) includes the optional command to render these diagrams
//! in generated rustdoc, retaining their source when offline.
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
pub mod application;
pub mod capture;
pub mod cli;
pub mod domain;
pub mod mcp;
pub(crate) mod pixels;
pub(crate) mod platform;
pub mod record;
pub mod selection;
pub mod settings;
#[cfg(windows)]
mod settings_dialog;
pub(crate) mod timing;
pub mod tray;
pub mod window;

#[cfg(test)]
#[path = "../tests/unit/app/mod.rs"]
mod main_tests;
#[cfg(test)]
#[path = "../tests/unit/support/mod.rs"]
mod test_support;
