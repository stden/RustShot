//! Tests for recording-selection resolution and capture plumbing.

use super::*;

// =========================================================================
// scaled_physical_dimension
// =========================================================================

#[test]
fn scaled_physical_dimension_150pct() {
    // 1707 * 1.5 = 2560.5, ceil = 2561
    assert_eq!(scaled_physical_dimension(1707, 1.5).unwrap(), 2561);
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
// build_screen_layout_inputs
// =========================================================================

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
