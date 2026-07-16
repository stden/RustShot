//! Tests for adjacency inference and physical screen layout building.

use super::*;

// =========================================================================
// infer_physical_y_from_horizontal_neighbor
// =========================================================================

#[test]
fn infer_physical_y_from_horizontal_neighbor_adjacent_right() {
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
        logical_x: 1920,
        logical_y: 100,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: false,
    };
    // Adjacent right, y_diff = 100, scale ratio = 1.0, so physical y = 0 + 100 = 100
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        Some(100)
    );
}

#[test]
fn infer_physical_y_from_horizontal_neighbor_not_adjacent_returns_none() {
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
        logical_x: 5000,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: false,
    };
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        None
    );
}

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
fn infer_physical_x_from_vertical_neighbor_not_adjacent_returns_none() {
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
        logical_x: 0,
        logical_y: 5000,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: false,
    };
    assert_eq!(
        infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
        None
    );
}

#[test]
fn infer_physical_y_from_horizontal_neighbor_adjacent_left() {
    // Target sits to the LEFT of the reference: target_right == reference.logical_x.
    let reference = ScreenLayoutInput {
        logical_x: 1920,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    };
    let target = ScreenLayoutInput {
        logical_x: 0,
        logical_y: 100,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: false,
    };
    // Adjacent left, y_diff = 100, scale ratio = 1.0, so physical y = 0 + 100 = 100.
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        Some(100)
    );
}

#[test]
fn infer_physical_y_from_horizontal_neighbor_high_dpi_scales_y_diff() {
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
        logical_x: 1920,
        logical_y: 50,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 3840,
        physical_height: 2160,
        is_primary: false,
    };
    // Adjacent right, y_diff = 50, scale ratio = 2160/1080 = 2.0, so physical y = 0 + 100.
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        Some(100)
    );
}

#[test]
fn infer_physical_x_from_vertical_neighbor_adjacent_above() {
    // Target sits ABOVE the reference: target_bottom == reference.logical_y.
    let reference = ScreenLayoutInput {
        logical_x: 0,
        logical_y: 1080,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    };
    let target = ScreenLayoutInput {
        logical_x: 300,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: false,
    };
    // Adjacent above, x_diff = 300, scale ratio = 1.0, so physical x = 0 + 300 = 300.
    assert_eq!(
        infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
        Some(300)
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
fn build_physical_screen_layouts_single_screen() {
    let inputs = vec![ScreenLayoutInput {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    }];
    let layouts = build_physical_screen_layouts(&inputs).unwrap();
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[0].physical_y, 0);
    assert_eq!(layouts[0].physical_width, 1920);
    assert_eq!(layouts[0].physical_height, 1080);
}

#[test]
fn build_physical_screen_layouts_two_horizontal() {
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
            logical_width: 1920,
            logical_height: 1080,
            physical_width: 1920,
            physical_height: 1080,
            is_primary: false,
        },
    ];
    let layouts = build_physical_screen_layouts(&inputs).unwrap();
    assert_eq!(layouts.len(), 2);
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[1].physical_x, 1920);
    assert_eq!(layouts[0].physical_y, 0);
    assert_eq!(layouts[1].physical_y, 0);
}

#[test]
fn build_physical_screen_layouts_two_vertical() {
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
    assert_eq!(layouts[1].physical_y, 1080);
}

#[test]
fn build_physical_screen_layouts_empty_input_is_error() {
    let result = build_physical_screen_layouts(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No screens"));
}

#[test]
fn build_physical_screen_layouts_mixed_dpi() {
    // Primary 1920x1080@1x, secondary 1707x960@1.5x (physical 2560x1440)
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
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[1].physical_x, 1920);
}

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
    let (normalized, min_x, min_y, w, h) = normalize_physical_screen_layouts(&layouts).unwrap();
    assert_eq!(min_x, 0);
    assert_eq!(min_y, 0);
    assert_eq!(w, 1920);
    assert_eq!(h, 1080);
    assert_eq!(normalized[0].physical_x, 0);
    assert_eq!(normalized[0].physical_y, 0);
}

#[test]
fn normalize_physical_screen_layouts_negative_origin_shifted() {
    let layouts = vec![
        PhysicalScreenLayout {
            logical_x: -1920,
            logical_y: 0,
            logical_width: 1920,
            logical_height: 1080,
            physical_x: -1920,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        },
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
    ];
    let (normalized, min_x, _min_y, w, h) = normalize_physical_screen_layouts(&layouts).unwrap();
    assert_eq!(min_x, -1920);
    assert_eq!(w, 3840);
    assert_eq!(h, 1080);
    assert_eq!(normalized[0].physical_x, 0);
    assert_eq!(normalized[1].physical_x, 1920);
}

#[test]
fn normalize_physical_screen_layouts_empty_returns_error() {
    let result = normalize_physical_screen_layouts(&[]);
    assert!(result.is_err());
}
