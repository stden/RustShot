//! Interactive region selection overlay.
//!
//! When the user presses PrintScreen (or chooses "Capture" from the tray menu),
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
mod domain;
mod geometry;
mod layout;
mod overlay;

// The bench shims are consumed only through the library tree (benches import
// `rustshot::selection::benchmark_*`); in the binary tree this re-export has
// no users, which is fine.
#[allow(unused_imports)]
pub use bench::*;
pub use desktop::*;
pub use domain::*;
pub use geometry::*;
// layout and overlay contain no fully-`pub` items (their contents are
// `pub(super)`), so a visible re-export would re-export nothing and trip the
// unused_imports lint; a private glob still lets descendant modules (tests)
// reach them through `crate::selection::*`.
#[allow(unused_imports)]
use layout::*;
#[allow(unused_imports)]
use overlay::*;

#[cfg(test)]
mod tests;
