//! Tests for `SelectionApp` state and multi-monitor screen lookup.

use super::*;

use winit::dpi::{PhysicalPosition, PhysicalSize};

// --- SelectionApp tests ---

#[test]
fn selection_app_new_sets_defaults() {
    let app = SelectionApp::new(vec![0u8; 16], 4, 5, 6, 7);

    assert!(app.window.is_none());
    assert!(app.surface.is_none());
    assert_eq!(app.screenshot, vec![0u8; 16]);
    assert_eq!(app.screen_width, 4);
    assert_eq!(app.screen_height, 5);
    assert_eq!(app.screen_x, 6);
    assert_eq!(app.screen_y, 7);
    assert_eq!(app.start_pos, None);
    assert_eq!(app.current_pos, None);
    assert!(!app.is_dragging);
    assert!(app.result.is_none());
    assert!(!app.cancelled);
}

#[test]
fn selection_app_is_dragging_is_false_by_default() {
    let app = SelectionApp::new(vec![], 800, 600, 0, 0);
    assert!(!app.is_dragging);
}

#[test]
fn selection_app_result_is_none_by_default() {
    let app = SelectionApp::new(vec![], 800, 600, 0, 0);
    assert!(app.result.is_none());
}

#[test]
fn selection_app_cancelled_is_false_by_default() {
    let app = SelectionApp::new(vec![], 800, 600, 0, 0);
    assert!(!app.cancelled);
}

#[test]
fn selection_app_fields_can_be_mutated() {
    // All observable state changes are reachable through the real event
    // handlers: press starts a drag, release stores a result, cancel flags.
    let mut app = SelectionApp::new(vec![0u8; 400 * 400 * 4], 400, 400, 0, 0);
    app.on_cursor_moved(PhysicalPosition::new(10.0, 10.0));
    app.on_press();
    assert!(app.is_dragging);
    app.on_cursor_moved(PhysicalPosition::new(110.0, 110.0));
    assert!(app.on_release(), "100x100 drag should produce a result");
    assert!(app.result.is_some());
    app.on_cancel();
    assert!(app.cancelled);
}

#[test]
fn selection_app_draw_without_surface_is_noop() {
    let mut app = SelectionApp::new(vec![0u8; 16], 4, 4, 0, 0);
    app.draw();
}

#[test]
fn selection_app_new_with_empty_screenshot() {
    let app = SelectionApp::new(vec![], 0, 0, 0, 0);
    assert_eq!(app.screenshot.len(), 0);
    assert_eq!(app.screen_width, 0);
    assert_eq!(app.screen_height, 0);
}

#[test]
fn selection_app_new_stores_screen_offset() {
    let app = SelectionApp::new(vec![], 1920, 1080, -1920, 0);
    assert_eq!(app.screen_x, -1920);
    assert_eq!(app.screen_y, 0);
}

#[test]
fn build_region_from_drag_with_negative_start_coords() {
    // Multi-monitor setup: drag starts at negative x/y
    let r = build_region_from_drag((-100, -50), (-85, -35)).unwrap();
    assert_eq!(r.physical_x, -100);
    assert_eq!(r.physical_y, -50);
    assert_eq!(r.physical_width, 15);
    assert_eq!(r.physical_height, 15);
}

#[test]
fn selection_app_draw_with_current_pos_no_surface_is_noop() {
    let mut app = SelectionApp::new(vec![0u8; 64], 4, 4, 0, 0);
    app.current_pos = Some((2, 2));
    app.draw(); // Should be a no-op since surface is None
}

#[test]
fn selection_app_draw_with_drag_state_no_surface_is_noop() {
    let mut app = SelectionApp::new(vec![0u8; 64], 4, 4, 0, 0);
    app.start_pos = Some((0, 0));
    app.current_pos = Some((3, 3));
    app.is_dragging = true;
    app.draw(); // Should be a no-op since surface is None
}

#[test]
fn region_zero_dimensions_is_valid_struct() {
    // Region can be constructed with zero dims (build_region_from_drag filters these,
    // but the type itself is not restricted)
    let r = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 0,
        physical_height: 0,
    };
    assert_eq!(r.physical_width, 0);
    assert_eq!(r.physical_height, 0);
}

#[test]
fn selection_app_screenshot_data_is_preserved() {
    let data: Vec<u8> = (0..=255u8).collect();
    let app = SelectionApp::new(data.clone(), 8, 8, 0, 0);
    assert_eq!(app.screenshot, data);
}

#[test]
fn selection_app_start_pos_can_be_set() {
    // start_pos is set by the real press handler from the cursor position
    let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
    app.on_cursor_moved(PhysicalPosition::new(10.0, 20.0));
    app.on_press();
    assert_eq!(app.start_pos, Some((10, 20)));
    assert!(app.is_dragging);
}

#[test]
fn selection_app_current_pos_can_be_set() {
    // current_pos is set by the real cursor-move handler
    let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
    app.on_cursor_moved(PhysicalPosition::new(50.0, 60.0));
    assert_eq!(app.current_pos, Some((50, 60)));
}

#[test]
fn region_max_u32_dimensions_are_valid() {
    let r = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: u32::MAX,
        physical_height: u32::MAX,
    };
    assert_eq!(r.physical_width, u32::MAX);
    assert_eq!(r.physical_height, u32::MAX);
}

// --- SelectionApp: extracted event-handler transitions ---

#[test]
fn selection_app_press_without_cursor_position_does_not_start_drag() {
    let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
    app.on_press();
    assert!(!app.is_dragging);
    assert_eq!(app.start_pos, None);
}

#[test]
fn selection_app_release_without_drag_is_noop() {
    let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
    app.on_cursor_moved(PhysicalPosition::new(50.0, 50.0));
    let selected = app.on_release();
    assert!(!selected);
    assert!(app.result.is_none());
    assert_eq!(app.current_pos, Some((50, 50)), "cursor state is untouched");
}

#[test]
fn selection_app_resize_aborts_active_drag() {
    let mut app = SelectionApp::new(vec![0u8; 100 * 100 * 4], 100, 100, 0, 0);
    app.on_cursor_moved(PhysicalPosition::new(10.0, 10.0));
    app.on_press();
    assert!(app.is_dragging);

    // Drag coordinates are stale after a resize, so the drag is aborted
    app.on_resize(PhysicalSize::new(50, 50));
    assert!(!app.is_dragging);
    assert_eq!(app.start_pos, None);
    assert_eq!(app.current_pos, None);

    // A subsequent release produces nothing
    assert!(!app.on_release());
    assert!(app.result.is_none());
}

#[test]
fn selection_app_resize_without_drag_keeps_cursor_state() {
    let mut app = SelectionApp::new(vec![0u8; 100 * 100 * 4], 100, 100, 0, 0);
    app.on_cursor_moved(PhysicalPosition::new(30.0, 40.0));
    app.on_resize(PhysicalSize::new(50, 50));
    assert_eq!(app.current_pos, Some((30, 40)));
}

#[test]
fn selection_app_cursor_move_clamps_to_overlay_bounds() {
    // Without a window the overlay size falls back to the screen dimensions
    let mut app = SelectionApp::new(vec![], 100, 100, 0, 0);
    app.on_cursor_moved(PhysicalPosition::new(-5.0, 250.0));
    assert_eq!(app.current_pos, Some((0, 100)));
}

#[test]
fn selection_app_failed_release_allows_second_attempt() {
    let mut app = SelectionApp::new(vec![0u8; 400 * 400 * 4], 400, 400, 0, 0);
    // First attempt: too small (3x3)
    app.on_cursor_moved(PhysicalPosition::new(10.0, 10.0));
    app.on_press();
    app.on_cursor_moved(PhysicalPosition::new(13.0, 13.0));
    assert!(!app.on_release());
    assert_eq!(app.start_pos, None, "failed drag resets start_pos");
    // Second attempt succeeds from the current cursor position
    app.on_press();
    assert_eq!(app.start_pos, Some((13, 13)));
    app.on_cursor_moved(PhysicalPosition::new(113.0, 213.0));
    assert!(app.on_release());
    let region = app.result.unwrap();
    assert_eq!(region.physical_x, 13);
    assert_eq!(region.physical_y, 13);
    assert_eq!(region.physical_width, 100);
    assert_eq!(region.physical_height, 200);
}

#[test]
fn region_copy_does_not_alias() {
    // Region implements Copy: two copies are fully independent values
    let r = Region {
        physical_x: 42,
        physical_y: 7,
        physical_width: 100,
        physical_height: 200,
    };
    let a = r;
    let b = r;
    // Both copies reflect the original values
    assert_eq!(a.physical_x, 42);
    assert_eq!(a.physical_y, 7);
    assert_eq!(a.physical_width, 100);
    assert_eq!(a.physical_height, 200);
    assert_eq!(b.physical_x, 42);
    assert_eq!(b.physical_y, 7);
    assert_eq!(b.physical_width, 100);
    assert_eq!(b.physical_height, 200);
    // The original is still usable (Copy, not moved)
    assert_eq!(r.physical_x, 42);
}

#[test]
fn region_i32_max_coords() {
    let r = Region {
        physical_x: i32::MAX,
        physical_y: i32::MIN,
        physical_width: 1,
        physical_height: 1,
    };
    assert_eq!(r.physical_x, i32::MAX);
    assert_eq!(r.physical_y, i32::MIN);
}

#[test]
fn selection_app_screenshot_len_large() {
    let size = 1920usize * 1080 * 4;
    let app = SelectionApp::new(vec![0u8; size], 1920, 1080, 0, 0);
    assert_eq!(app.screenshot.len(), size);
}

#[test]
fn region_debug_contains_x_value() {
    let r = Region {
        physical_x: 42,
        physical_y: 0,
        physical_width: 1,
        physical_height: 1,
    };
    assert!(format!("{:?}", r).contains("42"));
}

#[test]
fn selection_app_draw_called_multiple_times_without_surface() {
    // draw() must be a no-op (no panic) when surface is None, even called repeatedly
    let mut app = SelectionApp::new(vec![0u8; 64], 8, 8, 0, 0);
    app.draw();
    app.draw();
    app.draw();
}

#[test]
fn selection_app_all_init_fields() {
    // Verify all non-window/surface fields initialised by SelectionApp::new
    let data: Vec<u8> = vec![1, 2, 3, 4];
    let app = SelectionApp::new(data.clone(), 1280, 720, -1920, 100);

    // screenshot field
    assert_eq!(app.screenshot, data);
    // screen_width / screen_height
    assert_eq!(app.screen_width, 1280);
    assert_eq!(app.screen_height, 720);
    // screen_x / screen_y
    assert_eq!(app.screen_x, -1920);
    assert_eq!(app.screen_y, 100);
    // start_pos / current_pos
    assert_eq!(app.start_pos, None);
    assert_eq!(app.current_pos, None);
    // is_dragging
    assert!(!app.is_dragging);
    // result
    assert!(app.result.is_none());
    // cancelled
    assert!(!app.cancelled);
}

#[test]
fn find_screen_index_at_position_returns_matching_screen() {
    let screens = vec![(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
    assert_eq!(find_screen_index_at_position(screens, (2000, 200)), Some(1));
}

#[test]
fn find_screen_index_at_position_returns_first_screen_for_origin() {
    let screens = vec![(0, 0, 1920, 1080), (1920, 0, 2560, 1440)];
    assert_eq!(find_screen_index_at_position(screens, (0, 0)), Some(0));
}

#[test]
fn find_screen_index_at_position_supports_negative_coordinates() {
    let screens = vec![(-1920, 0, 1920, 1080), (0, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (-100, 10)), Some(0));
}

#[test]
fn find_screen_index_at_position_returns_none_when_no_match_exists() {
    let screens = vec![(0, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (3000, 3000)), None);
}

#[test]
fn find_screen_index_at_position_treats_right_boundary_as_exclusive() {
    let screens = vec![(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (1920, 50)), Some(1));
}

#[test]
fn find_screen_index_at_position_treats_bottom_boundary_as_exclusive() {
    let screens = vec![(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (100, 1080)), Some(1));
}

#[test]
fn find_screen_index_at_position_returns_none_for_empty_input() {
    let screens = Vec::<(i32, i32, u32, u32)>::new();
    assert_eq!(find_screen_index_at_position(screens, (0, 0)), None);
}

#[test]
fn find_screen_index_at_position_treats_left_boundary_as_inclusive() {
    let screens = vec![(100, 50, 300, 200)];
    assert_eq!(find_screen_index_at_position(screens, (100, 75)), Some(0));
}

#[test]
fn find_screen_index_at_position_treats_top_boundary_as_inclusive() {
    let screens = vec![(100, 50, 300, 200)];
    assert_eq!(find_screen_index_at_position(screens, (150, 50)), Some(0));
}

#[test]
fn find_screen_index_at_position_treats_point_above_top_as_outside() {
    let screens = vec![(100, 50, 300, 200)];
    assert_eq!(find_screen_index_at_position(screens, (150, 49)), None);
}

#[test]
fn find_screen_index_at_position_treats_point_left_of_screen_as_outside() {
    let screens = vec![(100, 50, 300, 200)];
    assert_eq!(find_screen_index_at_position(screens, (99, 100)), None);
}

// --- find_screen_index_at_position: multi-monitor layouts ---

#[test]
fn find_screen_index_at_position_three_horizontal_monitors() {
    let screens = vec![
        (0, 0, 1920, 1080),
        (1920, 0, 1920, 1080),
        (3840, 0, 1920, 1080),
    ];
    assert_eq!(
        find_screen_index_at_position(screens.clone(), (100, 100)),
        Some(0)
    );
    assert_eq!(
        find_screen_index_at_position(screens.clone(), (2000, 500)),
        Some(1)
    );
    assert_eq!(find_screen_index_at_position(screens, (4000, 500)), Some(2));
}

#[test]
fn find_screen_index_at_position_vertical_stack() {
    let screens = vec![(0, 0, 1920, 1080), (0, 1080, 1920, 1080)];
    assert_eq!(
        find_screen_index_at_position(screens.clone(), (500, 500)),
        Some(0)
    );
    assert_eq!(find_screen_index_at_position(screens, (500, 1200)), Some(1));
}

#[test]
fn find_screen_index_at_position_gap_between_screens_returns_none() {
    // Two screens with a gap between them
    let screens = vec![(0, 0, 100, 100), (200, 0, 100, 100)];
    assert_eq!(find_screen_index_at_position(screens, (150, 50)), None);
}

#[test]
fn find_screen_index_at_position_single_pixel_screen() {
    let screens = vec![(5, 5, 1, 1)];
    assert_eq!(
        find_screen_index_at_position(screens.clone(), (5, 5)),
        Some(0)
    );
    assert_eq!(find_screen_index_at_position(screens, (6, 5)), None);
}

#[test]
fn find_screen_index_at_position_returns_first_match_for_overlapping() {
    // Overlapping screens: first match wins
    let screens = vec![(0, 0, 200, 200), (100, 100, 200, 200)];
    assert_eq!(find_screen_index_at_position(screens, (150, 150)), Some(0));
}
