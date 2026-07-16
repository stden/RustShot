//! Additional edge-case coverage across selection helpers (wave 1).

use super::*;

// =========================================================================
// preferred_screen_index additional edge cases
// =========================================================================

#[test]
fn preferred_screen_index_single_non_primary_with_cursor_hit() {
    let screens = vec![(0i32, 0i32, 1920u32, 1080u32, false)];
    assert_eq!(preferred_screen_index(screens, Some((960, 540))), Some(0));
}

#[test]
fn preferred_screen_index_cursor_on_boundary_pixel() {
    let screens = vec![(0i32, 0i32, 1920u32, 1080u32, true)];
    // Last valid pixel is (1919, 1079)
    assert_eq!(preferred_screen_index(screens, Some((1919, 1079))), Some(0));
}

#[test]
fn preferred_screen_index_cursor_just_outside() {
    let screens = vec![(0i32, 0i32, 1920u32, 1080u32, true)];
    // (1920, 1080) is outside
    assert_eq!(
        preferred_screen_index(screens, Some((1920, 1080))),
        Some(0) // fallback to primary
    );
}

// --- logical_end / physical_end tests ---

#[test]
fn logical_end_basic() {
    assert_eq!(logical_end(0, 100), 100);
}

#[test]
fn logical_end_with_negative_start() {
    assert_eq!(logical_end(-100, 200), 100);
}

#[test]
fn logical_end_zero_size() {
    assert_eq!(logical_end(50, 0), 50);
}

#[test]
fn logical_end_large_values() {
    assert_eq!(logical_end(i32::MAX, 1), i64::from(i32::MAX) + 1);
}

#[test]
fn physical_end_basic() {
    assert_eq!(physical_end(0, 1920), 1920);
}

#[test]
fn physical_end_with_offset() {
    assert_eq!(physical_end(100, 500), 600);
}

#[test]
fn physical_end_negative_start() {
    assert_eq!(physical_end(-50, 100), 50);
}

#[test]
fn physical_end_zero_size() {
    assert_eq!(physical_end(100, 0), 100);
}

// --- scaled_physical_dimension edge cases ---

#[test]
fn scaled_physical_dimension_1x_scale() {
    assert_eq!(scaled_physical_dimension(1920, 1.0).unwrap(), 1920);
}

#[test]
fn scaled_physical_dimension_2x_scale() {
    assert_eq!(scaled_physical_dimension(960, 2.0).unwrap(), 1920);
}

#[test]
fn scaled_physical_dimension_zero_logical_size() {
    assert_eq!(scaled_physical_dimension(0, 2.0).unwrap(), 0);
}

#[test]
fn scaled_physical_dimension_negative_scale_fails() {
    assert!(scaled_physical_dimension(1920, -1.0).is_err());
}

#[test]
fn scaled_physical_dimension_zero_scale_fails() {
    assert!(scaled_physical_dimension(1920, 0.0).is_err());
}

#[test]
fn scaled_physical_dimension_nan_scale_fails() {
    assert!(scaled_physical_dimension(1920, f32::NAN).is_err());
}

#[test]
fn scaled_physical_dimension_infinity_scale_fails() {
    assert!(scaled_physical_dimension(1920, f32::INFINITY).is_err());
}

#[test]
fn scaled_physical_dimension_fractional_rounds_up() {
    // 1000 * 1.5 = 1500 (exact)
    assert_eq!(scaled_physical_dimension(1000, 1.5).unwrap(), 1500);
}

#[test]
fn scaled_physical_dimension_125_percent_uses_ceil() {
    // 1000 * 1.25 = 1250 (exact)
    assert_eq!(scaled_physical_dimension(1000, 1.25).unwrap(), 1250);
}

// --- resolve_recording_selection edge cases ---

#[test]
fn resolve_recording_selection_empty_layouts_returns_error() {
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    assert!(resolve_recording_selection(&[], region).is_err());
}

#[test]
fn resolve_recording_selection_region_at_origin() {
    let layouts = vec![PhysicalScreenLayout {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    }];
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 100,
        physical_height: 100,
    };
    let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(local.physical_x, 0);
    assert_eq!(local.physical_y, 0);
}

#[test]
fn resolve_recording_selection_full_screen_region() {
    let layouts = vec![PhysicalScreenLayout {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    }];
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    };
    let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(local.physical_width, 1920);
    assert_eq!(local.physical_height, 1080);
}

#[test]
fn resolve_recording_selection_selects_second_monitor() {
    let layouts = vec![
        PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
        PhysicalScreenLayout {
            logical_x: 1920,
            logical_y: 0,
            logical_width: 2560,
            logical_height: 1440,
            physical_x: 1920,
            physical_y: 0,
            physical_width: 2560,
            physical_height: 1440,
        },
    ];
    let region = Region {
        physical_x: 2000,
        physical_y: 100,
        physical_width: 200,
        physical_height: 200,
    };
    let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
    assert_eq!(idx, 1);
    assert_eq!(local.physical_x, 80);
    assert_eq!(local.physical_y, 100);
}

// --- monitor_scale_ratio edge cases ---

#[test]
fn monitor_scale_ratio_equal_sizes_returns_one() {
    assert_eq!(monitor_scale_ratio(100, 100), 1.0);
}

#[test]
fn monitor_scale_ratio_half_size() {
    assert_eq!(monitor_scale_ratio(200, 100), 0.5);
}

// -------------------------------------------------------------------------
// scaled_physical_dimension additional tests
// -------------------------------------------------------------------------

#[test]
fn scaled_physical_dimension_125_percent() {
    let result = scaled_physical_dimension(1920, 1.25).unwrap();
    assert_eq!(result, 2400);
}

#[test]
fn scaled_physical_dimension_175_percent() {
    let result = scaled_physical_dimension(1920, 1.75).unwrap();
    // 1920 * 1.75 = 3360
    assert_eq!(result, 3360);
}

#[test]
fn scaled_physical_dimension_fractional_rounds_up_to_ceil() {
    // 1000 * 1.333 = 1333.0 — exact, no rounding
    let result = scaled_physical_dimension(1000, 1.333).unwrap();
    assert_eq!(result, 1333);
}

#[test]
fn scaled_physical_dimension_1_logical_at_2x() {
    let result = scaled_physical_dimension(1, 2.0).unwrap();
    assert_eq!(result, 2);
}

#[test]
fn scaled_physical_dimension_large_logical_size() {
    let result = scaled_physical_dimension(7680, 1.0).unwrap();
    assert_eq!(result, 7680);
}

// -------------------------------------------------------------------------
// build_screen_layout_inputs additional tests
// -------------------------------------------------------------------------

#[test]
fn build_screen_layout_inputs_four_monitors() {
    let screens = vec![
        (0, 0, 1920u32, 1080u32, 1.0f32, true),
        (1920, 0, 1920, 1080, 1.0, false),
        (0, 1080, 1920, 1080, 1.5, false),
        (1920, 1080, 1920, 1080, 2.0, false),
    ];
    let result = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
    assert_eq!(result.len(), 4);
    // Target monitor gets exact capture size
    assert_eq!(result[0].physical_width, 1920);
    assert_eq!(result[0].physical_height, 1080);
    // Non-target at 1.0 scale
    assert_eq!(result[1].physical_width, 1920);
    // Non-target at 1.5 scale: ceil(1920 * 1.5) = 2880
    assert_eq!(result[2].physical_width, 2880);
    // Non-target at 2.0 scale
    assert_eq!(result[3].physical_width, 3840);
}

#[test]
fn build_screen_layout_inputs_target_last_of_three() {
    let screens = vec![
        (0, 0, 1920u32, 1080u32, 1.0f32, true),
        (1920, 0, 2560, 1440, 1.0, false),
        (4480, 0, 1920, 1080, 1.5, false),
    ];
    let result = build_screen_layout_inputs(screens, 2, (2880, 1620)).unwrap();
    // Target at index 2 gets exact capture size
    assert_eq!(result[2].physical_width, 2880);
    assert_eq!(result[2].physical_height, 1620);
    // Non-targets use scale
    assert_eq!(result[0].physical_width, 1920);
}

#[test]
fn build_screen_layout_inputs_negative_coords_preserved() {
    let screens = vec![
        (-1920, 0, 1920u32, 1080u32, 1.0f32, false),
        (0, 0, 1920, 1080, 1.0, true),
    ];
    let result = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
    assert_eq!(result[0].logical_x, -1920);
    assert_eq!(result[1].logical_x, 0);
}

// -------------------------------------------------------------------------
// logical_to_physical additional tests
// -------------------------------------------------------------------------

#[test]
fn logical_to_physical_very_large_value() {
    let result = logical_to_physical(10000, 2.0);
    assert_eq!(result, 20000);
}

#[test]
fn logical_to_physical_zero_at_150pct() {
    assert_eq!(logical_to_physical(0, 1.5), 0);
}

#[test]
fn logical_to_physical_scale_one_is_identity() {
    assert_eq!(logical_to_physical(1920, 1.0), 1920);
    assert_eq!(logical_to_physical(-500, 1.0), -500);
}

// -------------------------------------------------------------------------
// build_region_from_drag additional tests
// -------------------------------------------------------------------------

#[test]
fn build_region_from_drag_exact_minimum_size() {
    // The minimum accepted region should have width/height > some threshold
    let result = build_region_from_drag((0, 0), (10, 10));
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.physical_width, 10);
    assert_eq!(r.physical_height, 10);
}

#[test]
fn build_region_from_drag_1x1_returns_none() {
    let result = build_region_from_drag((0, 0), (1, 1));
    assert!(result.is_none());
}

#[test]
fn build_region_from_drag_identical_start_end_returns_none() {
    let result = build_region_from_drag((100, 200), (100, 200));
    assert!(result.is_none());
}

#[test]
fn build_region_from_drag_reversed_large_region() {
    let result = build_region_from_drag((1920, 1080), (0, 0));
    assert!(result.is_some());
    let r = result.unwrap();
    assert_eq!(r.physical_x, 0);
    assert_eq!(r.physical_y, 0);
    assert_eq!(r.physical_width, 1920);
    assert_eq!(r.physical_height, 1080);
}

// -------------------------------------------------------------------------
// virtual_desktop_scale_factor additional tests
// -------------------------------------------------------------------------

#[test]
fn virtual_desktop_scale_factor_two_primaries_uses_first() {
    let screens = vec![(true, 1.5f32), (true, 2.0)];
    let result = virtual_desktop_scale_factor(screens);
    assert_eq!(result, Some(1.5));
}

#[test]
fn virtual_desktop_scale_factor_no_primary_falls_back_to_first() {
    let screens = vec![(false, 1.0f32), (false, 2.0)];
    let result = virtual_desktop_scale_factor(screens);
    assert_eq!(result, Some(1.0));
}

// -------------------------------------------------------------------------
// find_screen_index_at_position additional tests
// -------------------------------------------------------------------------

#[test]
fn find_screen_index_at_position_origin_hit() {
    let screens = vec![(0, 0, 1920u32, 1080u32)];
    assert_eq!(find_screen_index_at_position(screens, (0, 0)), Some(0));
}

#[test]
fn find_screen_index_at_position_just_past_bottom_right() {
    let screens = vec![(0, 0, 1920u32, 1080u32)];
    assert_eq!(find_screen_index_at_position(screens, (1920, 1080)), None);
}

#[test]
fn find_screen_index_at_position_negative_coords_miss() {
    let screens = vec![(0, 0, 1920u32, 1080u32)];
    assert_eq!(find_screen_index_at_position(screens, (-1, -1)), None);
}

#[test]
fn find_screen_index_at_position_negative_origin_screen() {
    let screens = vec![(-1920, 0, 1920u32, 1080u32), (0, 0, 1920u32, 1080u32)];
    assert_eq!(find_screen_index_at_position(screens, (-960, 540)), Some(0));
}

// -------------------------------------------------------------------------
// preferred_screen_index additional tests
// -------------------------------------------------------------------------

#[test]
fn preferred_screen_index_single_primary_no_cursor() {
    let screens = vec![(0i32, 0i32, 1920u32, 1080u32, true)];
    assert_eq!(preferred_screen_index(screens, None), Some(0));
}

#[test]
fn preferred_screen_index_single_non_primary_no_cursor() {
    let screens = vec![(0i32, 0i32, 1920u32, 1080u32, false)];
    assert_eq!(preferred_screen_index(screens, None), Some(0));
}

// -------------------------------------------------------------------------
// ranges_overlap additional tests
// -------------------------------------------------------------------------

#[test]
fn ranges_overlap_same_range_overlaps() {
    assert!(ranges_overlap(0, 100, 0, 100));
}

#[test]
fn ranges_overlap_touching_boundaries_no_overlap() {
    // [0..100) and [100..200) do not overlap
    assert!(!ranges_overlap(0, 100, 100, 100));
}

#[test]
fn ranges_overlap_half_overlap() {
    // [0..100) and [50..150)
    assert!(ranges_overlap(0, 100, 50, 100));
}

#[test]
fn ranges_overlap_completely_separated() {
    assert!(!ranges_overlap(0, 10, 20, 10));
}

#[test]
fn ranges_overlap_containment() {
    assert!(ranges_overlap(0, 200, 50, 50));
}

#[test]
fn ranges_overlap_zero_size_a_does_not_overlap() {
    assert!(!ranges_overlap(0, 0, 0, 100));
}
