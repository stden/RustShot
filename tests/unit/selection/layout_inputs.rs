//! Tests for `build_screen_layout_inputs` and additional geometry coverage.

use super::*;

// =========================================================================
// build_screen_layout_inputs tests
// =========================================================================

#[test]
fn build_screen_layout_inputs_single_monitor() {
    let inputs =
        build_screen_layout_inputs([(0, 0, 1920, 1080, 1.0, true)], 0, (1920, 1080)).unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].physical_width, 1920);
    assert_eq!(inputs[0].physical_height, 1080);
    assert_eq!(inputs[0].logical_x, 0);
    assert_eq!(inputs[0].logical_y, 0);
    assert!(inputs[0].is_primary);
}

#[test]
fn build_screen_layout_inputs_preserves_is_primary() {
    let inputs = build_screen_layout_inputs(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 2560, 1440, 1.0, false),
        ],
        0,
        (1920, 1080),
    )
    .unwrap();
    assert!(inputs[0].is_primary);
    assert!(!inputs[1].is_primary);
}

#[test]
fn build_screen_layout_inputs_error_message_mentions_screen_index() {
    let err =
        build_screen_layout_inputs([(0, 0, 1920, 1080, 1.0, true)], 2, (1920, 1080)).unwrap_err();
    assert!(err.contains("screen index") || err.contains("Invalid"));
}

#[test]
fn build_screen_layout_inputs_empty_screens_returns_error() {
    let result: Result<Vec<ScreenLayoutInput>, String> =
        build_screen_layout_inputs([], 0, (1920, 1080));
    assert!(result.is_err());
}

#[test]
fn build_screen_layout_inputs_three_monitors_target_middle() {
    let inputs = build_screen_layout_inputs(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1707, 960, 1.5, false),
            (3627, 0, 1920, 1080, 1.0, false),
        ],
        1,
        (2560, 1440), // actual capture of the 150% monitor
    )
    .unwrap();
    assert_eq!(inputs.len(), 3);
    // Target (index 1) gets actual capture size
    assert_eq!(inputs[1].physical_width, 2560);
    assert_eq!(inputs[1].physical_height, 1440);
    // Others get scaled size
    assert_eq!(inputs[0].physical_width, 1920); // 1920 * 1.0
    assert_eq!(inputs[2].physical_width, 1920); // 1920 * 1.0
}

#[test]
fn build_screen_layout_inputs_non_target_fractional_scale_uses_ceil() {
    let inputs = build_screen_layout_inputs(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1000, 700, 1.333, false),
        ],
        0,
        (1920, 1080),
    )
    .unwrap();
    // 1000 * 1.333 = 1333.0, ceil = 1333
    assert_eq!(inputs[1].physical_width, 1333);
    // 700 * 1.333 = 933.1, ceil = 934
    assert_eq!(inputs[1].physical_height, 934);
}

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
fn build_screen_layout_inputs_preserves_order() {
    let inputs = build_screen_layout_inputs(
        [
            (100, 200, 800, 600, 1.0, false),
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 2560, 1440, 2.0, false),
        ],
        1,
        (1920, 1080),
    )
    .unwrap();
    assert_eq!(inputs[0].logical_x, 100);
    assert_eq!(inputs[1].logical_x, 0);
    assert_eq!(inputs[2].logical_x, 1920);
}

// =========================================================================
// find_screen_index_at_position — additional tests for cursor feature
// =========================================================================

#[test]
fn find_screen_index_at_position_cursor_exactly_on_monitor_origin() {
    let screens = vec![(1920, 0, 2560, 1440)];
    assert_eq!(find_screen_index_at_position(screens, (1920, 0)), Some(0));
}

#[test]
fn find_screen_index_at_position_cursor_one_before_monitor_start() {
    let screens = vec![(1920, 0, 2560, 1440)];
    assert_eq!(find_screen_index_at_position(screens, (1919, 0)), None);
}

#[test]
fn find_screen_index_at_position_cursor_at_bottom_right_edge() {
    let screens = vec![(0, 0, 1920, 1080)];
    // x=1919, y=1079 is last valid pixel
    assert_eq!(
        find_screen_index_at_position(screens, (1919, 1079)),
        Some(0)
    );
}

#[test]
fn find_screen_index_at_position_cursor_just_outside_bottom_right() {
    let screens = vec![(0, 0, 1920, 1080)];
    // x=1920 is out of bounds (exclusive)
    assert_eq!(find_screen_index_at_position(screens, (1920, 1079)), None);
}

#[test]
fn find_screen_index_at_position_four_monitors_grid() {
    let screens = vec![
        (0, 0, 1920, 1080),       // top-left
        (1920, 0, 1920, 1080),    // top-right
        (0, 1080, 1920, 1080),    // bottom-left
        (1920, 1080, 1920, 1080), // bottom-right
    ];
    assert_eq!(
        find_screen_index_at_position(screens, (2500, 1500)),
        Some(3)
    );
}

// =========================================================================
// logical_end / physical_end — additional coverage
// =========================================================================

#[test]
fn logical_end_i32_min_start_zero_size() {
    assert_eq!(logical_end(i32::MIN, 0), i64::from(i32::MIN));
}

#[test]
fn physical_end_large_physical_size() {
    assert_eq!(physical_end(0, u32::MAX), i64::from(u32::MAX));
}

#[test]
fn physical_end_negative_origin_with_size() {
    assert_eq!(physical_end(-500, 1000), 500);
}

// =========================================================================
// ranges_overlap — additional coverage
// =========================================================================

#[test]
fn ranges_overlap_by_one_pixel() {
    // [0, 51) and [50, 100) — overlap by 1
    assert!(ranges_overlap(0, 51, 50, 50));
}

#[test]
fn ranges_overlap_second_zero_size_inside_first() {
    // An empty interval contains no pixels, even inside a non-empty interval.
    assert!(!ranges_overlap(0, 100, 50, 0));
}

// =========================================================================
// monitor_scale_ratio — additional coverage
// =========================================================================

#[test]
fn monitor_scale_ratio_fractional() {
    let ratio = monitor_scale_ratio(1000, 1333);
    assert!((ratio - 1.333).abs() < 0.001);
}

// =========================================================================
// scaled_gap — additional coverage
// =========================================================================
