//! Tests for `build_region_from_drag` and desktop scale selection.

use super::*;

// --- build_region_from_drag tests ---

#[test]
fn build_region_from_drag_exactly_5x5_returns_none() {
    // width=5 and height=5: condition is width > 5 && height > 5, so 5 is NOT > 5
    assert!(build_region_from_drag((0, 0), (5, 5)).is_none());
}

#[test]
fn build_region_from_drag_6x6_returns_some() {
    // 6 > 5 and 6 > 5, so this should return Some
    let r = build_region_from_drag((0, 0), (6, 6));
    assert!(r.is_some());
    let r = r.unwrap();
    assert_eq!(r.physical_width, 6);
    assert_eq!(r.physical_height, 6);
}

#[test]
fn build_region_from_drag_exactly_width_6_height_5_returns_none() {
    // width=6 > 5, but height=5 is NOT > 5
    assert!(build_region_from_drag((0, 0), (6, 5)).is_none());
}

#[test]
fn build_region_from_drag_exactly_width_5_height_6_returns_none() {
    // width=5 is NOT > 5
    assert!(build_region_from_drag((0, 0), (5, 6)).is_none());
}

#[test]
fn virtual_desktop_scale_factor_falls_back_to_first_monitor() {
    let scale = virtual_desktop_scale_factor([(false, 1.25), (false, 1.5)]);
    assert_eq!(scale, Some(1.25));
}

#[test]
fn virtual_desktop_scale_factor_returns_none_for_empty_input() {
    let scale = virtual_desktop_scale_factor(std::iter::empty::<(bool, f32)>());
    assert_eq!(scale, None);
}
