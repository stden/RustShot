//! Tests for drag state transitions, `logical_to_physical`, and scale factors.

use super::*;

// --- SelectionApp: state transitions (driven through the real handlers) ---

#[test]
fn selection_app_cancel_resets_nothing_but_sets_cancelled() {
    let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
    app.on_cancel();
    assert!(app.cancelled);
    assert!(app.result.is_none());
    assert!(!app.is_dragging);
    assert_eq!(app.start_pos, None);
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
fn logical_to_physical_negative_half_rounds_away_from_zero() {
    assert_eq!(logical_to_physical(-3, 1.5), -5);
}

// --- virtual_desktop_scale_factor additional tests ---

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
fn capture_selection_desktop_propagates_screen_error() {
    let result = capture_selection_desktop_with(|| Err("single-screen capture failed".to_string()));

    assert_eq!(result.unwrap_err(), "single-screen capture failed");
}

// --- build_region_from_drag boundary precision ---

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
