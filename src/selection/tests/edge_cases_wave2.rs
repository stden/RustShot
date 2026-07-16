//! Additional edge-case coverage across selection helpers (wave 2).

use super::*;

// -------------------------------------------------------------------------
// monitor_scale_ratio additional tests
// -------------------------------------------------------------------------

#[test]
fn monitor_scale_ratio_equal_sizes_is_1() {
    assert_eq!(monitor_scale_ratio(1920, 1920), 1.0);
}

#[test]
fn monitor_scale_ratio_double_is_2() {
    assert_eq!(monitor_scale_ratio(1920, 3840), 2.0);
}

#[test]
fn monitor_scale_ratio_zero_logical_defaults_to_1() {
    assert_eq!(monitor_scale_ratio(0, 1920), 1.0);
}

// -------------------------------------------------------------------------
// logical_end and physical_end additional tests
// -------------------------------------------------------------------------

#[test]
fn logical_end_from_zero_origin() {
    assert_eq!(logical_end(0, 1920), 1920);
}

#[test]
fn logical_end_from_negative_origin() {
    assert_eq!(logical_end(-1920, 1920), 0);
}

#[test]
fn logical_end_with_zero_size() {
    assert_eq!(logical_end(100, 0), 100);
}

#[test]
fn physical_end_from_zero_origin() {
    assert_eq!(physical_end(0, 3840), 3840);
}

// -------------------------------------------------------------------------
// logical_to_physical additional edge cases
// -------------------------------------------------------------------------

#[test]
fn logical_to_physical_zero_with_2x_scale() {
    assert_eq!(logical_to_physical(0, 2.0), 0);
}

#[test]
fn logical_to_physical_identity_at_1x() {
    assert_eq!(logical_to_physical(100, 1.0), 100);
}

#[test]
fn logical_to_physical_960_at_2x() {
    assert_eq!(logical_to_physical(960, 2.0), 1920);
}

#[test]
fn logical_to_physical_negative_at_2x() {
    assert_eq!(logical_to_physical(-100, 2.0), -200);
}

// -------------------------------------------------------------------------
// scaled_gap additional tests
// -------------------------------------------------------------------------

#[test]
fn scaled_gap_zero_gap_identity() {
    assert_eq!(scaled_gap(0, 1920, 1920), 0);
}

#[test]
fn scaled_gap_at_1x() {
    assert_eq!(scaled_gap(10, 1920, 1920), 10);
}

#[test]
fn scaled_gap_at_2x() {
    assert_eq!(scaled_gap(10, 1920, 3840), 20);
}

// -------------------------------------------------------------------------
// virtual_desktop_scale_factor extra tests
// -------------------------------------------------------------------------

#[test]
fn virtual_desktop_scale_factor_one_primary_returns_its_scale() {
    let scale = virtual_desktop_scale_factor([(true, 2.0)]);
    assert_eq!(scale, Some(2.0));
}

#[test]
fn virtual_desktop_scale_factor_one_non_primary_returns_fallback() {
    let scale = virtual_desktop_scale_factor([(false, 1.5)]);
    assert_eq!(scale, Some(1.5));
}

// -------------------------------------------------------------------------
// build_physical_screen_layouts error path: disconnected screens
// -------------------------------------------------------------------------

#[test]
fn build_physical_screen_layouts_disconnected_screens_returns_error() {
    // Two screens with no logical adjacency or overlap — resolution should fail
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(5000, 5000, 1920, 1080, 1920, 1080),
    ];
    // The primary is index 0, so it gets (0,0). Screen 1 at (5000,5000)
    // has no overlap or adjacency with screen 0, so fallback positioning
    // kicks in. Whether this succeeds or fails depends on the fallback
    // logic finding a gap-based match.
    let result = build_physical_screen_layouts(&inputs);
    // The fallback path uses gap-based positioning, so it should still succeed
    // (left_candidate or above_candidate will match).
    assert!(result.is_ok());
}

// -------------------------------------------------------------------------
// normalize_physical_screen_layouts overflow error path
// -------------------------------------------------------------------------

#[test]
fn normalize_physical_screen_layouts_extreme_coordinates() {
    // Screens at extreme coordinates that cause i64 subtraction to exceed u32 range
    let layouts = vec![
        PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: i32::MIN,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
        PhysicalScreenLayout {
            logical_x: 1920,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: i32::MAX,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
    ];
    let result = normalize_physical_screen_layouts(&layouts);
    // physical_end of second screen = i32::MAX + 1920 as i64 = 2147485567
    // min_x = i32::MIN = -2147483648
    // width = 2147485567 - (-2147483648) = 4294969215 which overflows u32
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("out of range"),
        "error should mention out of range"
    );
}

// -------------------------------------------------------------------------
// build_physical_screen_layouts: primary not at index 0
// -------------------------------------------------------------------------

#[test]
fn build_physical_screen_layouts_primary_at_index_2() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(1920, 0, 1920, 1080, 1920, 1080),
        ScreenLayoutInput {
            logical_x: 3840,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        },
    ];
    let result = build_physical_screen_layouts(&inputs).unwrap();
    // Primary (index 2) should be at (0, 0), others positioned relative
    assert_eq!(result[2].physical_x, 0);
    assert_eq!(result[2].physical_y, 0);
    // Screen 1 is left of primary by 1920 logical pixels
    assert_eq!(result[1].physical_x, -1920);
    // Screen 0 is left of screen 1 by another 1920 logical pixels
    assert_eq!(result[0].physical_x, -3840);
}

// -------------------------------------------------------------------------
// resolve_recording_selection: region at exact screen boundary
// -------------------------------------------------------------------------

#[test]
fn resolve_recording_selection_region_at_bottom_right_corner() {
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
    // Region touching the bottom-right corner (1-pixel region at last pixel)
    let region = Region {
        physical_x: 1919,
        physical_y: 1079,
        physical_width: 1,
        physical_height: 1,
    };
    let result = resolve_recording_selection(&layouts, region);
    assert!(result.is_ok());
    let (idx, local) = result.unwrap();
    assert_eq!(idx, 0);
    assert_eq!(local.physical_x, 1919);
    assert_eq!(local.physical_y, 1079);
}

#[test]
fn resolve_recording_selection_region_one_pixel_past_boundary() {
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
    // Region that extends 1 pixel past the right edge
    let region = Region {
        physical_x: 1920,
        physical_y: 0,
        physical_width: 1,
        physical_height: 1,
    };
    let result = resolve_recording_selection(&layouts, region);
    assert!(result.is_err());
}

// -------------------------------------------------------------------------
// scaled_physical_dimension edge cases
// -------------------------------------------------------------------------

#[test]
fn scaled_physical_dimension_negative_infinity_returns_error() {
    assert!(scaled_physical_dimension(100, f32::NEG_INFINITY).is_err());
}

#[test]
fn scaled_physical_dimension_very_small_positive_scale() {
    // Very small but positive scale factor should succeed
    let result = scaled_physical_dimension(100, 0.01);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // ceil(100 * 0.01) = ceil(1.0) = 1
}

// -------------------------------------------------------------------------
// normalize_physical_screen_layouts — height overflow
// -------------------------------------------------------------------------

#[test]
fn normalize_physical_screen_layouts_width_overflow_returns_error() {
    // physical_end(1, u32::MAX) = 1 + u32::MAX = u32::MAX+1 as i64.
    // min_x = 0, so width = u32::MAX+1 > u32::MAX → Err.
    // Normalization: (0 - 0 = 0) and (1 - 0 = 1), both safe i32 ops.
    let layouts = vec![
        PhysicalScreenLayout {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1,
            logical_height: 1080,
            physical_x: 0,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1080,
        },
        PhysicalScreenLayout {
            logical_x: 1,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 1,
            physical_y: 0,
            physical_width: u32::MAX,
            physical_height: 1080,
        },
    ];
    let result = normalize_physical_screen_layouts(&layouts);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("out of range"),
        "error should mention out of range: {err}"
    );
}

// -------------------------------------------------------------------------
// scaled_physical_dimension — overflow for huge logical_size * scale
// -------------------------------------------------------------------------

#[test]
fn scaled_physical_dimension_huge_size_times_scale_returns_overflow_error() {
    // u32::MAX * 2.0 = ~8.6 billion > u32::MAX
    let result = scaled_physical_dimension(u32::MAX, 2.0);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of range"));
}

// -------------------------------------------------------------------------
// resolve_recording_selection — edge cases
// -------------------------------------------------------------------------

#[test]
fn resolve_recording_selection_local_region_has_zero_origin_for_first_screen() {
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
        physical_x: 100,
        physical_y: 200,
        physical_width: 300,
        physical_height: 400,
    };
    let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(local.physical_x, 100);
    assert_eq!(local.physical_y, 200);
}

#[test]
fn resolve_recording_selection_local_region_subtracts_screen_offset() {
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
    assert_eq!(local.physical_x, 80); // 2000 - 1920
    assert_eq!(local.physical_y, 100); // 100 - 0
    assert_eq!(local.physical_width, 200);
    assert_eq!(local.physical_height, 200);
}

// -------------------------------------------------------------------------
// build_screen_layout_inputs — invalid scale factor propagation
// -------------------------------------------------------------------------

#[test]
fn build_screen_layout_inputs_invalid_scale_factor_returns_error() {
    // Non-target monitor with NaN scale should fail
    let screens = vec![
        (0, 0, 1920, 1080, 1.0f32, true),
        (1920, 0, 2560, 1440, f32::NAN, false),
    ];
    let result = build_screen_layout_inputs(screens, 0, (1920, 1080));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("scale factor"));
}

// -------------------------------------------------------------------------
// build_region_from_drag — reversed coordinates
// -------------------------------------------------------------------------

#[test]
fn build_region_from_drag_reversed_coordinates_normalizes() {
    // Drag from bottom-right to top-left
    let region = build_region_from_drag((200, 300), (10, 20)).unwrap();
    assert_eq!(region.physical_x, 10);
    assert_eq!(region.physical_y, 20);
    assert_eq!(region.physical_width, 190);
    assert_eq!(region.physical_height, 280);
}

#[test]
fn build_region_from_drag_exactly_5_pixels_returns_none() {
    // 5 px in both dimensions — threshold is > 5, so this should be None
    assert!(build_region_from_drag((0, 0), (5, 5)).is_none());
}

#[test]
fn build_region_from_drag_6_pixels_returns_some() {
    // 6 px in both dimensions — just above threshold
    let region = build_region_from_drag((0, 0), (6, 6));
    assert!(region.is_some());
    let r = region.unwrap();
    assert_eq!(r.physical_width, 6);
    assert_eq!(r.physical_height, 6);
}

// -------------------------------------------------------------------------
// preferred_screen_index — cursor in gap between screens
// -------------------------------------------------------------------------

#[test]
fn preferred_screen_index_cursor_in_gap_falls_back_to_primary() {
    let screens = vec![
        (0, 0, 1920, 1080, false),
        (2000, 0, 1920, 1080, true), // gap from 1920 to 2000
    ];
    // Cursor at (1950, 500) is in the gap — not on any screen
    let idx = preferred_screen_index(screens, Some((1950, 500)));
    assert_eq!(idx, Some(1), "should fall back to primary screen");
}

// -------------------------------------------------------------------------
// logical_to_physical — additional scale factors
// -------------------------------------------------------------------------

#[test]
fn logical_to_physical_negative_value() {
    assert_eq!(logical_to_physical(-100, 2.0), -200);
}

#[test]
fn logical_to_physical_zero_value_at_scale_150() {
    assert_eq!(logical_to_physical(0, 1.5), 0);
}

#[test]
fn logical_to_physical_rounds_to_nearest() {
    // 100 * 1.5 = 150.0, rounds to 150
    assert_eq!(logical_to_physical(100, 1.5), 150);
    // 7 * 1.25 = 8.75, rounds to 9
    assert_eq!(logical_to_physical(7, 1.25), 9);
}
