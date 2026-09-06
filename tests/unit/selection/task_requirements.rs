//! Task 2.1/2.2 requirement tests for ranges and layout building.

use super::*;

// =========================================================================
// Task 2.1 — additional unit tests for ranges_overlap, monitor_scale_ratio,
//            scaled_gap, logical_to_physical
// Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9
// =========================================================================

// --- ranges_overlap: both ranges zero-size at different positions ---

#[test]
fn ranges_overlap_both_zero_size_at_different_positions() {
    // Two zero-size ranges at distinct positions never overlap
    assert!(!ranges_overlap(10, 0, 20, 0));
}

// --- monitor_scale_ratio: non-trivial ratios ---

#[test]
fn monitor_scale_ratio_small_logical_large_physical() {
    // 100 logical → 250 physical = 2.5
    assert_eq!(monitor_scale_ratio(100, 250), 2.5);
}

#[test]
fn monitor_scale_ratio_physical_smaller_than_logical() {
    // Downscale: 1920 logical → 960 physical = 0.5
    assert_eq!(monitor_scale_ratio(1920, 960), 0.5);
}

// --- scaled_gap: round(gap * physical / logical) verification ---

#[test]
fn scaled_gap_negative_gap_fractional_scale() {
    // gap=-10, logical=3, physical=4 → -10 * (4/3) = -13.333... → rounds to -13
    assert_eq!(scaled_gap(-10, 3, 4), -13);
}

#[test]
fn scaled_gap_matches_manual_formula() {
    // Verify scaled_gap(gap, l, p) == round(gap * p / l) for several cases
    let cases: Vec<(i32, u32, u32)> = vec![
        (100, 1920, 2880), // 1.5x → 150
        (50, 960, 1920),   // 2.0x → 100
        (7, 100, 150),     // 1.5x → 10.5 → 11 (rounds up)
    ];
    for (gap, logical, physical) in cases {
        let expected = ((gap as f32) * (physical as f32 / logical as f32)).round() as i32;
        assert_eq!(
            scaled_gap(gap, logical, physical),
            expected,
            "scaled_gap({gap}, {logical}, {physical}) should equal {expected}"
        );
    }
}

// --- logical_to_physical: negative coords + fractional scale ---

#[test]
fn logical_to_physical_negative_fractional_rounds_away_from_zero() {
    // -7 * 1.25 = -8.75 → rounds to -9
    assert_eq!(logical_to_physical(-7, 1.25), -9);
}

#[test]
fn logical_to_physical_zero_with_any_scale_is_zero() {
    // Zero input always produces zero regardless of scale
    for scale in [0.5, 1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
        assert_eq!(
            logical_to_physical(0, scale),
            0,
            "logical_to_physical(0, {scale}) should be 0"
        );
    }
}

// =========================================================================
// Task 2.2 — unit tests for build_physical_screen_layouts and
//            normalize_physical_screen_layouts
// Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7
// =========================================================================

// --- Req 2.1: single primary monitor placed at physical origin (0, 0) ---

#[test]
fn build_layouts_single_primary_at_origin() {
    let inputs = vec![ScreenLayoutInput {
        logical_x: 100,
        logical_y: 200,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 2880,
        physical_height: 1620,
        is_primary: true,
    }];
    let layouts = build_physical_screen_layouts(&inputs).unwrap();
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].physical_x, 0, "primary must be anchored at x=0");
    assert_eq!(layouts[0].physical_y, 0, "primary must be anchored at y=0");
    assert_eq!(layouts[0].physical_width, 2880);
    assert_eq!(layouts[0].physical_height, 1620);
    // Logical fields preserved
    assert_eq!(layouts[0].logical_x, 100);
    assert_eq!(layouts[0].logical_y, 200);
}

// --- Req 2.2: two horizontally adjacent monitors at different DPI ---

#[test]
fn build_layouts_horizontal_mixed_dpi_positions_account_for_pixel_dimensions() {
    // Primary: 1920x1080 logical, 1920x1080 physical (1x)
    // Secondary: 1707x960 logical, 2560x1440 physical (1.5x), right of primary
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
            logical_x: 1920,
            logical_y: 0,
            logical_width: 1707,
            logical_height: 960,
            physical_width: 2560,
            physical_height: 1440,
            is_primary: false,
        },
    ];
    let layouts = build_physical_screen_layouts(&inputs).unwrap();
    // Primary at origin
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[0].physical_y, 0);
    // Secondary physical_x = primary physical_width (they are adjacent)
    assert_eq!(
        layouts[1].physical_x, 1920,
        "second monitor physical_x should equal first monitor's physical width"
    );
    assert_eq!(layouts[1].physical_y, 0);
    // Each monitor retains its own physical dimensions
    assert_eq!(layouts[0].physical_width, 1920);
    assert_eq!(layouts[1].physical_width, 2560);
}

// --- Req 2.3: two vertically stacked monitors ---

#[test]
fn build_layouts_vertical_stack_second_y_equals_first_physical_height() {
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
            logical_y: 1080,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
    ];
    let layouts = build_physical_screen_layouts(&inputs).unwrap();
    assert_eq!(layouts[0].physical_y, 0);
    assert_eq!(
        layouts[1].physical_y, layouts[0].physical_height as i32,
        "second monitor physical_y should equal first monitor's physical height"
    );
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[1].physical_x, 0);
}

// --- Req 2.4: empty input returns error ---

#[test]
fn build_layouts_empty_input_returns_error_with_message() {
    let result = build_physical_screen_layouts(&[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("No screens"),
        "error should mention no screens: {err}"
    );
}

// --- Req 2.5: no primary monitor uses first as anchor at (0, 0) ---

#[test]
fn build_layouts_no_primary_uses_first_as_anchor() {
    let inputs = vec![
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false, // not primary
        },
        ScreenLayoutInput {
            logical_x: 1920,
            logical_y: 0,
            logical_width: 2560,
            logical_height: 1440,
            physical_width: 2560,
            physical_height: 1440,
            is_primary: false, // not primary either
        },
    ];
    let layouts = build_physical_screen_layouts(&inputs).unwrap();
    // First monitor should be the anchor at (0, 0)
    assert_eq!(
        layouts[0].physical_x, 0,
        "first monitor should be anchor at x=0 when no primary"
    );
    assert_eq!(
        layouts[0].physical_y, 0,
        "first monitor should be anchor at y=0 when no primary"
    );
    // Second monitor positioned relative to first
    assert_eq!(layouts[1].physical_x, 1920);
}

// --- Req 2.6: negative coordinate normalization shifts min X/Y to zero ---

#[test]
fn normalize_layouts_negative_coords_shifted_to_zero() {
    let layouts = vec![
        make_layout((-1920, -500), (1920, 1080), (-2560, -750), (2560, 1440)),
        make_layout((0, 0), (1920, 1080), (0, 0), (1920, 1080)),
    ];
    let (normalized, min_x, min_y, _w, _h) = normalize_physical_screen_layouts(&layouts).unwrap();
    assert_eq!(min_x, -2560);
    assert_eq!(min_y, -750);
    // After normalization, the leftmost/topmost screen starts at (0, 0)
    assert_eq!(
        normalized[0].physical_x, 0,
        "min X should be shifted to zero"
    );
    assert_eq!(
        normalized[0].physical_y, 0,
        "min Y should be shifted to zero"
    );
    // Second screen shifted by the same offset
    assert_eq!(normalized[1].physical_x, 2560);
    assert_eq!(normalized[1].physical_y, 750);
}

// --- Req 2.7: total width/height spans full virtual desktop ---

#[test]
fn normalize_layouts_total_size_spans_full_virtual_desktop() {
    // Two horizontally adjacent monitors with different physical sizes
    let layouts = vec![
        make_layout((0, 0), (1920, 1080), (0, 0), (1920, 1080)),
        make_layout((1920, 0), (1707, 960), (1920, 0), (2560, 1440)),
    ];
    let (_normalized, _min_x, _min_y, width, height) =
        normalize_physical_screen_layouts(&layouts).unwrap();
    // Total width = rightmost physical_end - leftmost physical_x
    // = (1920 + 2560) - 0 = 4480
    assert_eq!(
        width, 4480,
        "total width should span from leftmost to rightmost physical edge"
    );
    // Total height = max physical_end_y - min physical_y
    // = max(1080, 1440) - 0 = 1440
    assert_eq!(
        height, 1440,
        "total height should span from topmost to bottommost physical edge"
    );
}

#[test]
fn normalize_layouts_vertical_stack_total_height_is_sum() {
    let layouts = vec![
        make_layout((0, 0), (1920, 1080), (0, 0), (1920, 1080)),
        make_layout((0, 1080), (1920, 1080), (0, 1080), (1920, 1080)),
    ];
    let (_normalized, _min_x, _min_y, width, height) =
        normalize_physical_screen_layouts(&layouts).unwrap();
    assert_eq!(width, 1920, "width should be the single column width");
    assert_eq!(
        height, 2160,
        "height should be sum of both monitors' physical heights"
    );
}
