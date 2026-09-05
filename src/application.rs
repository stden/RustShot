//! Application workflows shared by presentation adapters.
//!
//! Workflows own operation ordering and partial-failure policy. Their injected
//! handlers describe the effects they need, so unit and behaviour tests can use
//! in-memory effects without opening a desktop or editing the host registry.
//! They do not depend on the tray, settings dialog, or [`crate::app`] composition
//! root. Feature modules retain their own native adapters and shared domain types.
//!
//! ```mermaid
//! flowchart TD
//!     Tray[Tray controller] --> Capture[Capture output workflow]
//!     Startup[Startup routing] --> Autorun[Auto-start workflow]
//!     Settings[Settings dialog] --> Autorun
//!     Capture --> CapturePort[Capture / clipboard / save handlers]
//!     Autorun --> StartupPort[Install / register / launch handlers]
//! ```

pub mod autorun;
pub mod capture;
