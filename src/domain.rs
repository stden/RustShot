//! Shared values and pure monitor geometry.
//!
//! This layer uses only the standard library. It does not enumerate monitors,
//! capture pixels, read settings, or own an event loop. Conversion to image
//! encoder formats belongs to the capture adapter.
//!
//! ```mermaid
//! flowchart BT
//!     Geometry["geometry: drag and screen policy"] --> Coordinates["coordinates: logical / physical / Region"]
//!     Layout["layout: monitor placement"] --> Geometry
//!     Format["Format"]
//!     Rate["Fps"]
//! ```

mod coordinates;
mod format;
mod fps;
pub(crate) mod geometry;
pub(crate) mod layout;

pub use coordinates::*;
pub use format::Format;
pub use fps::Fps;
pub use geometry::*;
