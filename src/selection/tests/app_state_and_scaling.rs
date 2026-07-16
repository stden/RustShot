//! Tests for drag state transitions, `logical_to_physical`, and scale factors.

use super::*;

// --- SelectionApp: state transitions ---

#[test]
fn selection_app_start_and_end_drag_simulation() {
    let mut app = SelectionApp::new(vec![0u8; 64], 4, 4, 0, 0);
    // Simulate mouse move
    app.current_pos = Some((1, 1));
    // Simulate mouse press
    app.is_dragging = true;
    app.start_pos = app.current_pos;
    // Simulate mouse move during drag
    app.current_pos = Some((3, 3));
    // Simulate mouse release
    app.is_dragging = false;
    if let (Some(start), Some(end)) = (app.start_pos, app.current_pos) {
        app.result = build_region_from_drag(start, end);
    }
    // Region is 2x2, which is < 5, so None
    assert!(app.result.is_none());
}

#[test]
fn selection_app_large_drag_produces_region() {
    let mut app = SelectionApp::new(vec![0u8; 400 * 400 * 4], 400, 400, 0, 0);
    app.start_pos = Some((10, 10));
    app.current_pos = Some((100, 100));
    app.is_dragging = false;
    if let (Some(start), Some(end)) = (app.start_pos, app.current_pos) {
        app.result = build_region_from_drag(start, end);
    }
    let region = app.result.unwrap();
    assert_eq!(region.physical_x, 10);
    assert_eq!(region.physical_y, 10);
    assert_eq!(region.physical_width, 90);
    assert_eq!(region.physical_height, 90);
}

#[test]
fn selection_app_cancel_resets_nothing_but_sets_cancelled() {
    let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
    app.cancelled = true;
    assert!(app.cancelled);
    assert!(app.result.is_none());
}

// --- logical_to_physical additional tests ---

#[test]
fn logical_to_physical_scale_1_is_identity() {
    assert_eq!(logical_to_physical(100, 1.0), 100);
    assert_eq!(logical_to_physical(-100, 1.0), -100);
    assert_eq!(logical_to_physical(0, 1.0), 0);
}

#[test]
fn logical_to_physical_scale_2_doubles() {
    assert_eq!(logical_to_physical(100, 2.0), 200);
    assert_eq!(logical_to_physical(-50, 2.0), -100);
}

#[test]
fn logical_to_physical_zero_value() {
    assert_eq!(logical_to_physical(0, 1.5), 0);
    assert_eq!(logical_to_physical(0, 2.0), 0);
}

#[test]
fn logical_to_physical_fractional_scale_rounds() {
    // 100 * 1.25 = 125.0 (exact)
    assert_eq!(logical_to_physical(100, 1.25), 125);
    // 3 * 1.5 = 4.5 → rounds to 4 (banker's rounding) or 5
    let result = logical_to_physical(3, 1.5);
    assert!(result == 4 || result == 5, "expected 4 or 5, got {result}");
}

#[test]
fn logical_to_physical_negative_half_rounds_away_from_zero() {
    assert_eq!(logical_to_physical(-3, 1.5), -5);
}

// --- virtual_desktop_scale_factor additional tests ---

#[test]
fn virtual_desktop_scale_factor_single_primary_monitor() {
    let scale = virtual_desktop_scale_factor([(true, 2.0)]);
    assert_eq!(scale, Some(2.0));
}

#[test]
fn virtual_desktop_scale_factor_primary_is_first() {
    let scale = virtual_desktop_scale_factor([(true, 1.25), (false, 1.5)]);
    assert_eq!(scale, Some(1.25));
}

#[test]
fn virtual_desktop_scale_factor_primary_is_last_of_three() {
    let scale = virtual_desktop_scale_factor([(false, 1.0), (false, 1.25), (true, 1.5)]);
    assert_eq!(scale, Some(1.5));
}

#[test]
fn virtual_desktop_scale_factor_multiple_primary_monitors_uses_first_primary() {
    let scale = virtual_desktop_scale_factor([(true, 1.25), (true, 1.5), (false, 2.0)]);
    assert_eq!(scale, Some(1.25));
}

// --- find_screen_index_at_position additional tests ---

#[test]
fn find_screen_index_at_position_large_monitor_count() {
    // 10 side-by-side 100px monitors
    let screens: Vec<(i32, i32, u32, u32)> =
        (0..10).map(|i| (i * 100, 0, 100u32, 100u32)).collect();
    assert_eq!(
        find_screen_index_at_position(screens.clone(), (950, 50)),
        Some(9)
    );
    assert_eq!(find_screen_index_at_position(screens, (1000, 50)), None);
}

#[test]
fn find_screen_index_at_position_negative_y_offset() {
    // Monitor above the primary
    let screens = vec![(0, -1080, 1920, 1080), (0, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (100, -500)), Some(0));
}

#[test]
fn find_screen_index_at_position_negative_top_boundary_is_inclusive() {
    let screens = vec![(0, -1080, 1920, 1080), (0, 0, 1920, 1080)];
    assert_eq!(
        find_screen_index_at_position(screens, (100, -1080)),
        Some(0)
    );
}

#[test]
fn find_screen_index_at_position_negative_monitor_right_boundary_switches_to_next_screen() {
    let screens = vec![(-1920, 0, 1920, 1080), (0, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (0, 300)), Some(1));
}

#[test]
fn preferred_screen_index_uses_monitor_under_cursor() {
    let screens = vec![(0, 0, 1920, 1080, true), (1920, 0, 2560, 1440, false)];
    assert_eq!(preferred_screen_index(screens, Some((2500, 200))), Some(1));
}

#[test]
fn preferred_screen_index_falls_back_to_primary_when_cursor_unavailable() {
    let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, true)];
    assert_eq!(preferred_screen_index(screens, None), Some(1));
}

#[test]
fn preferred_screen_index_falls_back_to_first_when_no_primary_exists() {
    let screens = vec![(0, 0, 1920, 1080, false), (1920, 0, 2560, 1440, false)];
    assert_eq!(preferred_screen_index(screens, Some((9999, 9999))), Some(0));
}

#[test]
fn build_screen_layout_inputs_uses_capture_size_for_target_monitor() {
    let inputs = build_screen_layout_inputs(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1707, 960, 1.5, false),
        ],
        1,
        (2560, 1440),
    )
    .expect("layout inputs should build");

    assert_eq!(inputs[1].physical_width, 2560); // target gets exact capture size
    assert_eq!(inputs[1].physical_height, 1440);
}

#[test]
fn build_screen_layout_inputs_uses_scale_factor_for_other_monitors() {
    let inputs = build_screen_layout_inputs(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1707, 960, 1.5, false),
        ],
        0,
        (1920, 1080),
    )
    .expect("layout inputs should build");

    assert_eq!(inputs[1].physical_width, 2561);
    assert_eq!(inputs[1].physical_height, 1440);
}

#[test]
fn build_screen_layout_inputs_rejects_unknown_target_monitor() {
    let error =
        build_screen_layout_inputs([(0, 0, 1920, 1080, 1.0, true)], 1, (1920, 1080)).unwrap_err();

    assert!(error.contains("screen index"));
}

#[test]
fn capture_selection_desktop_propagates_screen_error() {
    let result = capture_selection_desktop_with(|| Err("single-screen capture failed".to_string()));

    assert_eq!(result.unwrap_err(), "single-screen capture failed");
}

// --- build_region_from_drag boundary precision ---

#[test]
fn build_region_from_drag_width_1_height_1000_returns_none() {
    // Only 1 pixel wide — not > 5
    assert!(build_region_from_drag((0, 0), (1, 1000)).is_none());
}

#[test]
fn build_region_from_drag_exact_boundary_5x6_returns_none() {
    // width=5 is NOT > 5, even though height=6 IS > 5
    assert!(build_region_from_drag((10, 10), (15, 16)).is_none());
}

#[test]
fn build_region_from_drag_exact_boundary_6x5_returns_none() {
    // height=5 is NOT > 5, even though width=6 IS > 5
    assert!(build_region_from_drag((10, 10), (16, 15)).is_none());
}

#[test]
fn build_region_from_drag_very_large_coordinates() {
    let r = build_region_from_drag(
        (i32::MIN / 2, i32::MIN / 2),
        (i32::MIN / 2 + 100, i32::MIN / 2 + 100),
    );
    assert!(r.is_some());
    let r = r.unwrap();
    assert_eq!(r.physical_width, 100);
    assert_eq!(r.physical_height, 100);
}

// --- logical_to_physical edge cases ---

#[test]
fn logical_to_physical_zero_is_zero() {
    assert_eq!(logical_to_physical(0, 1.0), 0);
    assert_eq!(logical_to_physical(0, 2.0), 0);
    assert_eq!(logical_to_physical(0, 0.5), 0);
}

#[test]
fn logical_to_physical_scale_2x() {
    assert_eq!(logical_to_physical(100, 2.0), 200);
    assert_eq!(logical_to_physical(-100, 2.0), -200);
}

#[test]
fn logical_to_physical_scale_half() {
    assert_eq!(logical_to_physical(100, 0.5), 50);
}

#[test]
fn logical_to_physical_rounds_correctly() {
    // 3 * 1.5 = 4.5, f32::round() rounds half-away-from-zero → 5
    assert_eq!(logical_to_physical(3, 1.5), 5);
    // 7 * 0.3 = 2.1, rounds to 2
    assert_eq!(logical_to_physical(7, 0.3), 2);
}

#[test]
fn logical_to_physical_common_dpi_scales() {
    // 125% scaling
    assert_eq!(logical_to_physical(1280, 1.25), 1600);
    // 150% scaling
    assert_eq!(logical_to_physical(1280, 1.5), 1920);
    // 175% scaling
    assert_eq!(logical_to_physical(1280, 1.75), 2240);
}
