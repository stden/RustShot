//! Tests for adjacency inference and physical screen layout building.

use super::*;

// =========================================================================
// infer_physical_y_from_horizontal_neighbor
// =========================================================================

// =========================================================================
// infer_physical_x_from_vertical_neighbor
// =========================================================================

#[test]
fn infer_physical_x_from_vertical_neighbor_adjacent_below() {
    let reference = ScreenLayoutInput {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    };
    let target = ScreenLayoutInput {
        logical_x: 200,
        logical_y: 1080,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: false,
    };
    // Adjacent below, x_diff = 200, scale ratio = 1.0, so physical x = 0 + 200 = 200
    assert_eq!(
        infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
        Some(200)
    );
}

#[test]
fn infer_physical_x_from_vertical_neighbor_high_dpi_scales_x_diff() {
    let reference = ScreenLayoutInput {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    };
    let target = ScreenLayoutInput {
        logical_x: 80,
        logical_y: 1080,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 3840,
        physical_height: 2160,
        is_primary: false,
    };
    // Adjacent below, x_diff = 80, scale ratio = 3840/1920 = 2.0, so physical x = 0 + 160.
    assert_eq!(
        infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
        Some(160)
    );
}

// =========================================================================
// build_physical_screen_layouts
// =========================================================================

#[test]
fn build_physical_screen_layouts_primary_not_first() {
    // Non-primary first, primary second. Anchor should be at index 1.
    let inputs = vec![
        ScreenLayoutInput {
            logical_x: -1920,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
        ScreenLayoutInput {
            logical_x: 0,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: true,
        },
    ];
    let layouts = build_physical_screen_layouts(&inputs).unwrap();
    // Anchor (primary) is at physical (0, 0)
    assert_eq!(layouts[1].physical_x, 0);
    assert_eq!(layouts[1].physical_y, 0);
    // Left monitor at physical_x = -1920
    assert_eq!(layouts[0].physical_x, -1920);
}

// =========================================================================
// normalize_physical_screen_layouts
// =========================================================================

#[test]
fn normalize_physical_screen_layouts_single_screen() {
    let layouts = vec![make_layout((0, 0), (1920, 1080), (0, 0), (1920, 1080))];
    let (normalized, min_x, min_y, w, h) = normalize_physical_screen_layouts(&layouts).unwrap();
    assert_eq!(min_x, 0);
    assert_eq!(min_y, 0);
    assert_eq!(w, 1920);
    assert_eq!(h, 1080);
    assert_eq!(normalized[0].physical_x, 0);
    assert_eq!(normalized[0].physical_y, 0);
}

#[test]
fn normalize_physical_screen_layouts_empty_returns_error() {
    let result = normalize_physical_screen_layouts(&[]);
    assert!(result.is_err());
}
