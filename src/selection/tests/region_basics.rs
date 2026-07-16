//! Tests for `Region` and `build_region_from_drag` basics.

use super::*;

// --- Region struct tests ---

#[test]
fn region_fields_are_accessible() {
    let r = Region {
        physical_x: 10,
        physical_y: 20,
        physical_width: 300,
        physical_height: 200,
    };
    assert_eq!(r.physical_x, 10);
    assert_eq!(r.physical_y, 20);
    assert_eq!(r.physical_width, 300);
    assert_eq!(r.physical_height, 200);
}

#[test]
fn region_implements_copy() {
    let r = Region {
        physical_x: 1,
        physical_y: 2,
        physical_width: 3,
        physical_height: 4,
    };
    let s = r; // Uses Copy; r is still usable after this
    assert_eq!(r.physical_x, s.physical_x);
    assert_eq!(r.physical_y, s.physical_y);
    assert_eq!(r.physical_width, s.physical_width);
    assert_eq!(r.physical_height, s.physical_height);
}

#[test]
fn region_implements_clone() {
    let r = Region {
        physical_x: 5,
        physical_y: 6,
        physical_width: 7,
        physical_height: 8,
    };
    #[allow(clippy::clone_on_copy)]
    let s = r.clone();
    assert_eq!(s.physical_x, 5);
    assert_eq!(s.physical_y, 6);
    assert_eq!(s.physical_width, 7);
    assert_eq!(s.physical_height, 8);
}

#[test]
fn region_debug_repr_is_non_empty() {
    let r = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(!format!("{:?}", r).is_empty());
}

#[test]
fn region_supports_negative_coords() {
    // Regions can start at negative screen coordinates (multi-monitor)
    let r = Region {
        physical_x: -1920,
        physical_y: -100,
        physical_width: 1920,
        physical_height: 1080,
    };
    assert_eq!(r.physical_x, -1920);
    assert_eq!(r.physical_y, -100);
}

// --- build_region_from_drag tests ---

#[test]
fn build_region_from_drag_supports_normal_drag() {
    let region = build_region_from_drag((5, 7), (15, 20)).expect("region should exist");
    assert_eq!(region.physical_x, 5);
    assert_eq!(region.physical_y, 7);
    assert_eq!(region.physical_width, 10);
    assert_eq!(region.physical_height, 13);
}

#[test]
fn build_region_from_drag_supports_reversed_drag() {
    let region = build_region_from_drag((15, 20), (5, 7)).expect("region should exist");
    assert_eq!(region.physical_x, 5);
    assert_eq!(region.physical_y, 7);
    assert_eq!(region.physical_width, 10);
    assert_eq!(region.physical_height, 13);
}

#[test]
fn build_region_from_drag_filters_too_small_regions() {
    assert!(build_region_from_drag((0, 0), (2, 3)).is_none());
}

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
fn build_region_from_drag_zero_size_returns_none() {
    assert!(build_region_from_drag((0, 0), (0, 0)).is_none());
}

#[test]
fn build_region_from_drag_wide_but_short_returns_none() {
    // width=100 > 5, but height=3 is NOT > 5
    assert!(build_region_from_drag((0, 0), (100, 3)).is_none());
}

#[test]
fn build_region_from_drag_tall_but_narrow_returns_none() {
    // height=100 > 5, but width=3 is NOT > 5
    assert!(build_region_from_drag((0, 0), (3, 100)).is_none());
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
fn build_region_from_drag_result_top_left_is_min_coords() {
    // drag from bottom-right to top-left
    let r = build_region_from_drag((100, 200), (10, 20)).unwrap();
    assert_eq!(r.physical_x, 10);
    assert_eq!(r.physical_y, 20);
    assert_eq!(r.physical_width, 90);
    assert_eq!(r.physical_height, 180);
}

#[test]
fn build_region_from_drag_large_region() {
    let r = build_region_from_drag((0, 0), (1920, 1080)).unwrap();
    assert_eq!(r.physical_x, 0);
    assert_eq!(r.physical_y, 0);
    assert_eq!(r.physical_width, 1920);
    assert_eq!(r.physical_height, 1080);
}

#[test]
fn virtual_desktop_scale_factor_prefers_primary_monitor() {
    let scale = virtual_desktop_scale_factor([(false, 1.0), (true, 1.5), (false, 2.0)]);
    assert_eq!(scale, Some(1.5));
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

#[test]
fn logical_to_physical_rounds_using_scale_factor() {
    assert_eq!(logical_to_physical(-1280, 1.5), -1920);
    assert_eq!(logical_to_physical(1280, 1.5), 1920);
}
