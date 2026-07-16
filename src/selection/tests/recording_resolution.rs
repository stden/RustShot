//! Tests for recording-selection resolution and capture plumbing.

use super::*;

// =========================================================================
// resolve_recording_selection
// =========================================================================

#[test]
fn resolve_recording_selection_region_within_first_screen() {
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
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 1920,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
    ];
    let region = Region {
        physical_x: 100,
        physical_y: 100,
        physical_width: 500,
        physical_height: 400,
    };
    let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(local.physical_x, 100);
    assert_eq!(local.physical_y, 100);
    assert_eq!(local.physical_width, 500);
    assert_eq!(local.physical_height, 400);
}

#[test]
fn resolve_recording_selection_region_within_second_screen() {
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
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 1920,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
    ];
    let region = Region {
        physical_x: 2000,
        physical_y: 100,
        physical_width: 500,
        physical_height: 400,
    };
    let (idx, local) = resolve_recording_selection(&layouts, region).unwrap();
    assert_eq!(idx, 1);
    assert_eq!(local.physical_x, 80); // 2000 - 1920
    assert_eq!(local.physical_y, 100);
}

#[test]
fn resolve_recording_selection_spanning_two_screens_returns_error() {
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
            logical_width: 1920,
            logical_height: 1080,
            physical_x: 1920,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
    ];
    // Region spans from screen 0 to screen 1
    let region = Region {
        physical_x: 1800,
        physical_y: 100,
        physical_width: 500,
        physical_height: 400,
    };
    let result = resolve_recording_selection(&layouts, region);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("single monitor"));
}

#[test]
fn resolve_recording_selection_exact_screen_bounds() {
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
    assert_eq!(local.physical_x, 0);
    assert_eq!(local.physical_y, 0);
    assert_eq!(local.physical_width, 1920);
    assert_eq!(local.physical_height, 1080);
}

// =========================================================================
// scaled_physical_dimension
// =========================================================================

#[test]
fn scaled_physical_dimension_1x() {
    assert_eq!(scaled_physical_dimension(1920, 1.0).unwrap(), 1920);
}

#[test]
fn scaled_physical_dimension_150pct() {
    // 1707 * 1.5 = 2560.5, ceil = 2561
    assert_eq!(scaled_physical_dimension(1707, 1.5).unwrap(), 2561);
}

#[test]
fn scaled_physical_dimension_2x() {
    assert_eq!(scaled_physical_dimension(1920, 2.0).unwrap(), 3840);
}

#[test]
fn scaled_physical_dimension_zero_logical() {
    assert_eq!(scaled_physical_dimension(0, 1.5).unwrap(), 0);
}

#[test]
fn scaled_physical_dimension_zero_scale_returns_error() {
    assert!(scaled_physical_dimension(1920, 0.0).is_err());
}

#[test]
fn scaled_physical_dimension_negative_scale_returns_error() {
    assert!(scaled_physical_dimension(1920, -1.0).is_err());
}

#[test]
fn scaled_physical_dimension_nan_scale_returns_error() {
    assert!(scaled_physical_dimension(1920, f32::NAN).is_err());
}

#[test]
fn scaled_physical_dimension_infinity_scale_returns_error() {
    assert!(scaled_physical_dimension(1920, f32::INFINITY).is_err());
}

// =========================================================================
// logical_to_physical additional tests
// =========================================================================

#[test]
fn logical_to_physical_1x_is_identity() {
    assert_eq!(logical_to_physical(1920, 1.0), 1920);
}

#[test]
fn logical_to_physical_2x_doubles() {
    assert_eq!(logical_to_physical(960, 2.0), 1920);
}

#[test]
fn logical_to_physical_zero_always_zero() {
    assert_eq!(logical_to_physical(0, 1.5), 0);
}

#[test]
fn logical_to_physical_125pct() {
    // 1280 * 1.25 = 1600.0
    assert_eq!(logical_to_physical(1280, 1.25), 1600);
}

// =========================================================================
// virtual_desktop_scale_factor additional tests
// =========================================================================

#[test]
fn virtual_desktop_scale_factor_single_primary() {
    let scale = virtual_desktop_scale_factor([(true, 2.0)]);
    assert_eq!(scale, Some(2.0));
}

#[test]
fn virtual_desktop_scale_factor_single_non_primary() {
    let scale = virtual_desktop_scale_factor([(false, 1.25)]);
    assert_eq!(scale, Some(1.25));
}

#[test]
fn virtual_desktop_scale_factor_primary_first_wins() {
    let scale = virtual_desktop_scale_factor([(true, 1.0), (true, 2.0), (false, 3.0)]);
    assert_eq!(scale, Some(1.0));
}

// =========================================================================
// build_screen_layout_inputs
// =========================================================================

#[test]
fn build_screen_layout_inputs_target_gets_exact_capture_size() {
    let screens = vec![(0, 0, 1920u32, 1080u32, 1.5f32, true)];
    let inputs = build_screen_layout_inputs(screens, 0, (2880, 1620)).unwrap();
    assert_eq!(inputs[0].physical_width, 2880);
    assert_eq!(inputs[0].physical_height, 1620);
}

#[test]
fn build_screen_layout_inputs_non_target_uses_scale() {
    let screens = vec![
        (0, 0, 1920u32, 1080u32, 1.0f32, true),
        (1920, 0, 1707u32, 960u32, 1.5f32, false),
    ];
    let inputs = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
    // target (index 0) gets exact capture size
    assert_eq!(inputs[0].physical_width, 1920);
    // non-target (index 1): 1707 * 1.5 = 2560.5 → ceil = 2561
    assert_eq!(inputs[1].physical_width, 2561);
}

#[test]
fn build_screen_layout_inputs_out_of_range_index_returns_error() {
    let screens = vec![(0, 0, 1920u32, 1080u32, 1.0f32, true)];
    let result = build_screen_layout_inputs(screens, 5, (1920, 1080));
    assert!(result.is_err());
}

#[test]
fn build_screen_layout_inputs_keeps_negative_logical_coords() {
    let screens = vec![(-1920, 100, 1920u32, 1080u32, 1.0f32, false)];
    let inputs = build_screen_layout_inputs(screens, 0, (1920, 1080)).unwrap();
    assert_eq!(inputs[0].logical_x, -1920);
    assert_eq!(inputs[0].logical_y, 100);
    assert_eq!(inputs[0].logical_width, 1920);
    assert_eq!(inputs[0].logical_height, 1080);
}

// =========================================================================
// fallback_physical_x / fallback_physical_y
// =========================================================================

#[test]
fn fallback_physical_x_finds_nearest_left_neighbor() {
    let inputs = vec![
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        },
        ScreenLayoutInput {
            // Gap: logically at 2000 (80px gap from 1920)
            logical_x: 2000,
            logical_y: 500,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
    ];
    let positions = vec![Some(0), None];
    // Target index 1: should find from left neighbor (index 0)
    let result = fallback_physical_x(&inputs, &positions, 1);
    assert!(result.is_some());
    // 0 + 1920 + scaled_gap(80, 1920, 1920) = 0 + 1920 + 80 = 2000
    assert_eq!(result.unwrap(), 2000);
}

#[test]
fn fallback_physical_x_no_positioned_neighbors_returns_none() {
    let inputs = vec![ScreenLayoutInput {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    }];
    let positions = vec![None];
    assert_eq!(fallback_physical_x(&inputs, &positions, 0), None);
}

#[test]
fn fallback_physical_y_finds_nearest_above_neighbor() {
    let inputs = vec![
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        },
        ScreenLayoutInput {
            logical_x: 500,
            logical_y: 1200,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
    ];
    let positions = vec![Some(0), None];
    let result = fallback_physical_y(&inputs, &positions, 1);
    assert!(result.is_some());
    // 0 + 1080 + scaled_gap(120, 1080, 1080) = 0 + 1080 + 120 = 1200
    assert_eq!(result.unwrap(), 1200);
}

#[test]
fn fallback_physical_y_out_of_range_returns_none() {
    let inputs = vec![ScreenLayoutInput {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    }];
    let positions = vec![Some(0)];
    assert_eq!(fallback_physical_y(&inputs, &positions, 5), None);
}

#[test]
fn fallback_physical_x_prefers_left_over_right_neighbor() {
    // Both a left and a right neighbor are equally close; the left wins.
    let inputs = vec![
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        },
        ScreenLayoutInput {
            logical_x: 2000,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
        ScreenLayoutInput {
            logical_x: 4000,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
    ];
    let positions = vec![Some(0), None, Some(5000)];
    // Left candidate: 0 + 1920 + scaled_gap(80) = 2000.
    // Right candidate: 5000 - 1920 - 80 = 3000. Left is preferred.
    assert_eq!(fallback_physical_x(&inputs, &positions, 1), Some(2000));
}

#[test]
fn fallback_physical_y_prefers_above_over_below_neighbor() {
    // Both an above and a below neighbor are equally close; the above wins.
    let inputs = vec![
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        },
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 1200,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 2400,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
    ];
    let positions = vec![Some(0), None, Some(5000)];
    // Above candidate: 0 + 1080 + scaled_gap(120) = 1200.
    // Below candidate: 5000 - 1080 - 120 = 3800. Above is preferred.
    assert_eq!(fallback_physical_y(&inputs, &positions, 1), Some(1200));
}

// =========================================================================
// capture_selection_desktop_with
// =========================================================================

#[test]
fn capture_selection_desktop_with_uses_screen_capture() {
    let capture = VirtualDesktopCapture {
        rgba_data: vec![0u8; 16],
        width: 2,
        height: 2,
        origin_x: 0,
        origin_y: 0,
        screens: vec![],
        layouts: vec![],
    };
    let result = capture_selection_desktop_with(|| Ok(capture));
    assert!(result.is_ok());
    let vdc = result.unwrap();
    assert_eq!(vdc.width, 2);
    assert_eq!(vdc.height, 2);
}

#[test]
fn capture_selection_desktop_with_propagates_error() {
    let result = capture_selection_desktop_with(|| Err("capture failed".to_string()));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("capture failed"));
}
