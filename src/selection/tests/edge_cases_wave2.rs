//! Additional edge-case coverage across selection helpers (wave 2).

use super::*;

// -------------------------------------------------------------------------
// build_physical_screen_layouts: disconnected screens
// -------------------------------------------------------------------------

#[test]
fn build_physical_screen_layouts_disconnected_screens_resolved_via_gap_fallback() {
    // Two screens with no logical adjacency or overlap. The primary is
    // index 0, so it anchors at (0, 0). Screen 1 at logical (5000, 5000)
    // cannot be placed by adjacency inference, so the gap-based fallback
    // positions it: 0 + 1920 + gap(3080) = 5000 on x, 0 + 1080 + gap(3920)
    // = 5000 on y (1:1 scale keeps the logical gaps unchanged).
    let inputs = vec![
        ScreenLayoutInput {
            is_primary: true,
            ..make_screen(0, 0, 1920, 1080, 1920, 1080)
        },
        make_screen(5000, 5000, 1920, 1080, 1920, 1080),
    ];
    let layouts = build_physical_screen_layouts(&inputs).expect("gap fallback should resolve");
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[0].physical_y, 0);
    assert_eq!(layouts[1].physical_x, 5000);
    assert_eq!(layouts[1].physical_y, 5000);
}

// -------------------------------------------------------------------------
// normalize_physical_screen_layouts overflow error path
// -------------------------------------------------------------------------

#[test]
fn normalize_physical_screen_layouts_extreme_coordinates() {
    // Screens at extreme coordinates that cause i64 subtraction to exceed u32 range
    let layouts = vec![
        make_layout(0, 0, 1920, 1080, i32::MIN, 0, 1920, 1080),
        make_layout(1920, 0, 1920, 1080, i32::MAX, 0, 1920, 1080),
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
// resolve_recording_selection: region at exact screen boundary
// -------------------------------------------------------------------------

#[test]
fn resolve_recording_selection_region_at_bottom_right_corner() {
    let layouts = vec![make_layout(0, 0, 1920, 1080, 0, 0, 1920, 1080)];
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
    let layouts = vec![make_layout(0, 0, 1920, 1080, 0, 0, 1920, 1080)];
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
// normalize_physical_screen_layouts — width overflow
// -------------------------------------------------------------------------

#[test]
fn normalize_physical_screen_layouts_width_overflow_returns_error() {
    // physical_end(1, u32::MAX) = 1 + u32::MAX = u32::MAX+1 as i64.
    // min_x = 0, so width = u32::MAX+1 > u32::MAX → Err.
    // Normalization: (0 - 0 = 0) and (1 - 0 = 1), both safe i32 ops.
    let layouts = vec![
        make_layout(0, 0, 1, 1080, 0, 0, 1, 1080),
        make_layout(1, 0, 1920, 1080, 1, 0, u32::MAX, 1080),
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
