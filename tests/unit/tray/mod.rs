//! Tests for the tray module, split by theme:
//!
//! - [`icon_pixels`] — per-pixel geometry of the default icon (body
//!   boundaries, lens circle, corners, edges).
//! - [`icon_invariants`] — whole-buffer invariants of the default icon
//!   (alpha values, pixel counts, symmetry).
//! - [`menu_and_tray`] — menu id constants, labels, and tray creation.

use super::*;

mod catalog;
mod icon_invariants;
mod icon_pixels;
mod menu_and_tray;

#[cfg(not(target_os = "macos"))]
mod behavior;

// Helper: get pixel index for (x, y) in the 32x32 RGBA buffer
fn pixel_idx(x: u32, y: u32) -> usize {
    ((y * DEFAULT_ICON_SIZE + x) * 4) as usize
}
