//! Interactive region selection overlay.
//!
//! When the user presses `PrintScreen` (or chooses "Capture" from the tray menu),
//! a child process is spawned with `--select`.  This module:
//!
//! 1. Detects which monitor the mouse cursor is on ([`get_cursor_position`]).
//! 2. Captures only that monitor's screen image (`capture_screen_at_cursor`).
//! 3. Opens a borderless, always-on-top window covering that single screen and
//!    draws the screenshot as its background (`SelectionApp`).
//! 4. Lets the user drag a rectangle (crosshair → cyan border) and returns the
//!    cropped [`image::RgbaImage`] or a [`Region`] for recording.
//!
//! Two public entry points:
//! - [`select_region`] — returns the cropped RGBA image (used for screenshots).
//! - [`select_region_for_recording`] — returns the screen + local-coordinate
//!   [`Region`] so the recorder can capture just that area.

mod bench;
mod desktop;
mod overlay;

// Keep existing public selection paths while shared geometry has its own owner.
pub use crate::domain::{
    build_region_from_drag, build_region_from_physical_drag, find_screen_index_at_position,
    logical_to_physical, preferred_screen_index, virtual_desktop_scale_factor, LogicalPoint,
    LogicalSize, PhysicalPoint, PhysicalSize, Region, ScaleFactor,
};
pub use crate::platform::get_cursor_position;
pub use bench::*;
pub use desktop::*;
// Internal geometry and overlay helpers stay private to consumers of this facade.
#[allow(
    unused_imports,
    reason = "Existing child modules and private unit suites reach shared geometry through this facade."
)]
use crate::domain::{geometry::*, layout::*};
#[allow(
    unused_imports,
    reason = "Private unit suites share the overlay facade; production uses only its entry points."
)]
use overlay::*;

#[cfg(test)]
#[path = "../tests/unit/selection/mod.rs"]
mod tests;
