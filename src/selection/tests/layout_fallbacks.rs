//! Tests for layout fallbacks, cursor position, and preferred screen choice.

use super::*;

// --- build_physical_screen_layouts additional tests ---

#[test]
fn build_physical_screen_layouts_single_monitor() {
    let layouts = build_physical_screen_layouts(&[ScreenLayoutInput {
        logical_x: 0,
        logical_y: 0,
        logical_width: 1920,
        logical_height: 1080,
        physical_width: 1920,
        physical_height: 1080,
        is_primary: true,
    }])
    .unwrap();
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[0].physical_y, 0);
}

#[test]
fn build_physical_screen_layouts_empty_returns_error() {
    let result = build_physical_screen_layouts(&[]);
    assert!(result.is_err());
}

#[test]
fn build_physical_screen_layouts_three_monitors_horizontal() {
    let layouts = build_physical_screen_layouts(&[
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        ScreenLayoutInput {
            is_primary: true,
            ..make_screen(1920, 0, 1920, 1080, 1920, 1080)
        },
        make_screen(3840, 0, 1920, 1080, 1920, 1080),
    ])
    .unwrap();
    // Primary is index 1, anchored at physical_x=0
    assert_eq!(layouts[0].physical_x, -1920);
    assert_eq!(layouts[1].physical_x, 0);
    assert_eq!(layouts[2].physical_x, 1920);
}

#[test]
fn build_physical_screen_layouts_stacked_monitors() {
    let layouts = build_physical_screen_layouts(&[
        ScreenLayoutInput {
            is_primary: true,
            ..make_screen(0, 0, 1920, 1080, 1920, 1080)
        },
        make_screen(0, 1080, 1920, 1080, 1920, 1080),
    ])
    .unwrap();
    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[0].physical_y, 0);
    assert_eq!(layouts[1].physical_x, 0);
    assert_eq!(layouts[1].physical_y, 1080);
}

// --- fallback_physical_x tests ---

#[test]
fn fallback_physical_x_from_left_neighbor_with_gap() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(2000, 0, 1920, 1080, 1920, 1080), // 80px gap
    ];
    let positions = vec![Some(0), None];
    let result = fallback_physical_x(&inputs, &positions, 1);
    // gap = 80, scale = 1.0, so physical_x = 0 + 1920 + 80 = 2000
    assert_eq!(result, Some(2000));
}

#[test]
fn fallback_physical_x_from_right_neighbor_with_gap() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(2000, 0, 1920, 1080, 1920, 1080),
    ];
    let positions = vec![None, Some(2000)];
    let result = fallback_physical_x(&inputs, &positions, 0);
    // target is left of reference, gap = 80, scale = 1.0
    assert_eq!(result, Some(0));
}

#[test]
fn fallback_physical_x_prefers_nearest_left_neighbor() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(2500, 0, 1920, 1080, 1920, 1080),
        make_screen(4100, 0, 1920, 1080, 1920, 1080),
    ];
    let positions = vec![Some(0), Some(2500), None];
    let result = fallback_physical_x(&inputs, &positions, 2);
    assert_eq!(result, Some(4100));
}

#[test]
fn fallback_physical_x_high_dpi_gap_scales_target_width() {
    let inputs = vec![
        make_screen(0, 0, 100, 100, 100, 100),
        make_screen(150, 0, 100, 100, 200, 200),
    ];
    let positions = vec![Some(0), None];
    let result = fallback_physical_x(&inputs, &positions, 1);
    assert_eq!(result, Some(200));
}

#[test]
fn fallback_physical_x_no_resolved_neighbors() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(2000, 0, 1920, 1080, 1920, 1080),
    ];
    let positions = vec![None, None];
    let result = fallback_physical_x(&inputs, &positions, 0);
    assert_eq!(result, None);
}

#[test]
fn fallback_physical_x_out_of_bounds_index() {
    let inputs = vec![make_screen(0, 0, 1920, 1080, 1920, 1080)];
    let positions = vec![Some(0)];
    let result = fallback_physical_x(&inputs, &positions, 5);
    assert_eq!(result, None);
}

// --- fallback_physical_y tests ---

#[test]
fn fallback_physical_y_from_above_neighbor_with_gap() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(0, 1200, 1920, 1080, 1920, 1080), // 120px gap
    ];
    let positions = vec![Some(0), None];
    let result = fallback_physical_y(&inputs, &positions, 1);
    assert_eq!(result, Some(1200));
}

#[test]
fn fallback_physical_y_from_below_neighbor_with_gap() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(0, 1200, 1920, 1080, 1920, 1080),
    ];
    let positions = vec![None, Some(1200)];
    let result = fallback_physical_y(&inputs, &positions, 0);
    assert_eq!(result, Some(0));
}

#[test]
fn fallback_physical_y_high_dpi_gap_scales_target_height() {
    let inputs = vec![
        make_screen(0, 0, 100, 100, 100, 100),
        make_screen(0, 150, 100, 100, 200, 200),
    ];
    let positions = vec![Some(0), None];
    let result = fallback_physical_y(&inputs, &positions, 1);
    assert_eq!(result, Some(200));
}

#[test]
fn fallback_physical_y_no_resolved_neighbors() {
    let inputs = vec![
        make_screen(0, 0, 1920, 1080, 1920, 1080),
        make_screen(0, 2000, 1920, 1080, 1920, 1080),
    ];
    let positions = vec![None, None];
    let result = fallback_physical_y(&inputs, &positions, 1);
    assert_eq!(result, None);
}

#[test]
fn fallback_physical_y_out_of_bounds_index() {
    let inputs = vec![make_screen(0, 0, 1920, 1080, 1920, 1080)];
    let positions = vec![Some(0)];
    let result = fallback_physical_y(&inputs, &positions, 5);
    assert_eq!(result, None);
}

// =========================================================================
// get_cursor_position tests
// =========================================================================

#[cfg(not(windows))]
#[test]
fn get_cursor_position_returns_error_on_non_windows() {
    let result = get_cursor_position();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not supported"));
}

#[cfg(not(windows))]
#[test]
fn get_cursor_position_error_message_is_not_empty() {
    let err = get_cursor_position().unwrap_err();
    assert!(!err.is_empty());
}

// =========================================================================
// preferred_screen_index tests
// =========================================================================

#[test]
fn preferred_screen_index_empty_returns_none() {
    let screens: Vec<(i32, i32, u32, u32, bool)> = vec![];
    assert_eq!(preferred_screen_index(screens, Some((100, 100))), None);
}

#[test]
fn preferred_screen_index_single_screen_always_returns_zero() {
    let screens = vec![(0, 0, 1920, 1080, true)];
    assert_eq!(preferred_screen_index(screens.clone(), None), Some(0));
    assert_eq!(
        preferred_screen_index(screens.clone(), Some((500, 500))),
        Some(0)
    );
    assert_eq!(preferred_screen_index(screens, Some((9999, 9999))), Some(0));
}

#[test]
fn preferred_screen_index_cursor_on_second_monitor() {
    let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
    // Cursor at (2000, 500) is on the second monitor
    assert_eq!(preferred_screen_index(screens, Some((2000, 500))), Some(1));
}

#[test]
fn preferred_screen_index_cursor_on_first_monitor() {
    let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
    assert_eq!(preferred_screen_index(screens, Some((960, 540))), Some(0));
}

#[test]
fn preferred_screen_index_cursor_on_boundary_between_monitors() {
    let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
    // x=1920 is exactly the start of monitor 2 (exclusive end of monitor 1)
    assert_eq!(preferred_screen_index(screens, Some((1920, 500))), Some(1));
}

#[test]
fn preferred_screen_index_cursor_at_origin() {
    let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
    assert_eq!(preferred_screen_index(screens, Some((0, 0))), Some(0));
}

#[test]
fn preferred_screen_index_cursor_outside_all_screens_falls_back_to_primary() {
    let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
    assert_eq!(
        preferred_screen_index(screens, Some((10000, 10000))),
        Some(1)
    );
}

#[test]
fn preferred_screen_index_cursor_outside_no_primary_falls_back_to_first() {
    let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, false)];
    assert_eq!(
        preferred_screen_index(screens, Some((10000, 10000))),
        Some(0)
    );
}

#[test]
fn preferred_screen_index_no_cursor_uses_primary() {
    let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
    assert_eq!(preferred_screen_index(screens, None), Some(1));
}

#[test]
fn preferred_screen_index_no_cursor_no_primary_uses_first() {
    let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, false)];
    assert_eq!(preferred_screen_index(screens, None), Some(0));
}

#[test]
fn preferred_screen_index_three_monitors_cursor_on_middle() {
    let screens = vec![
        (0, 0, 1920, 1080, true),
        (1920, 0, 2560, 1440, false),
        (4480, 0, 1920, 1080, false),
    ];
    assert_eq!(preferred_screen_index(screens, Some((3000, 700))), Some(1));
}

#[test]
fn preferred_screen_index_three_monitors_cursor_on_third() {
    let screens = vec![
        (0, 0, 1920, 1080, true),
        (1920, 0, 2560, 1440, false),
        (4480, 0, 1920, 1080, false),
    ];
    assert_eq!(preferred_screen_index(screens, Some((5000, 500))), Some(2));
}

#[test]
fn preferred_screen_index_negative_coords_monitor() {
    let screens = vec![(-1920, 0, 1920, 1080, false), (0, 0, 1920, 1080, true)];
    assert_eq!(preferred_screen_index(screens, Some((-500, 500))), Some(0));
}

#[test]
fn preferred_screen_index_vertically_stacked_monitors() {
    let screens = vec![(0, 0, 1920, 1080, true), (0, 1080, 1920, 1080, false)];
    // Cursor on lower monitor
    assert_eq!(preferred_screen_index(screens, Some((960, 1500))), Some(1));
}

#[test]
fn preferred_screen_index_cursor_at_last_pixel_of_first_monitor() {
    let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
    // x=1919, y=1079 is the last valid pixel of monitor 0
    assert_eq!(preferred_screen_index(screens, Some((1919, 1079))), Some(0));
}

#[test]
fn preferred_screen_index_priority_cursor_over_primary() {
    // Even though monitor 0 is primary, cursor on monitor 1 → returns 1
    let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
    assert_eq!(preferred_screen_index(screens, Some((2500, 200))), Some(1));
}
