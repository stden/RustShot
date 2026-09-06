//! Tests for physical-position inference between neighboring screens.

use super::*;

// --- infer_physical_x tests ---

#[test]
fn infer_physical_x_same_logical_x_returns_reference() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 1080), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_x(&reference, &target, 0), Some(0));
}

#[test]
fn infer_physical_x_target_right_of_reference() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((1920, 0), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_x(&reference, &target, 0), Some(1920));
}

#[test]
fn infer_physical_x_target_left_of_reference() {
    let reference = make_screen((1920, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 0), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_x(&reference, &target, 1920), Some(0));
}

#[test]
fn infer_physical_x_no_y_overlap_returns_none() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((1920, 2000), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_x(&reference, &target, 0), None);
}

#[test]
fn infer_physical_x_non_adjacent_with_y_overlap_returns_none() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((3000, 0), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_x(&reference, &target, 0), None);
}

#[test]
fn infer_physical_x_high_dpi_target_right_of_reference() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((1920, 0), (1707, 960), (2560, 1440));
    assert_eq!(infer_physical_x(&reference, &target, 0), Some(1920));
}

#[test]
fn infer_physical_x_high_dpi_target_left_of_reference() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((-1707, 0), (1707, 960), (2560, 1440));
    assert_eq!(infer_physical_x(&reference, &target, 0), Some(-2560));
}

// --- infer_physical_y tests ---

#[test]
fn infer_physical_y_same_logical_y_returns_reference() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((1920, 0), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_y(&reference, &target, 0), Some(0));
}

#[test]
fn infer_physical_y_target_below_reference() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 1080), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_y(&reference, &target, 0), Some(1080));
}

#[test]
fn infer_physical_y_target_above_reference() {
    let reference = make_screen((0, 1080), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 0), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_y(&reference, &target, 1080), Some(0));
}

#[test]
fn infer_physical_y_no_x_overlap_returns_none() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((3000, 1080), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_y(&reference, &target, 0), None);
}

#[test]
fn infer_physical_y_non_adjacent_with_x_overlap_returns_none() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 3000), (1920, 1080), (1920, 1080));
    assert_eq!(infer_physical_y(&reference, &target, 0), None);
}

// --- infer_physical_y_from_horizontal_neighbor tests ---

#[test]
fn infer_y_from_horizontal_neighbor_adjacent_same_y() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((1920, 0), (1920, 1080), (1920, 1080));
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        Some(0)
    );
}

#[test]
fn infer_y_from_horizontal_neighbor_adjacent_different_y() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((1920, 100), (1920, 1080), (1920, 1080));
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        Some(100)
    );
}

#[test]
fn infer_y_from_horizontal_neighbor_not_adjacent_returns_none() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((3000, 0), (1920, 1080), (1920, 1080));
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        None
    );
}

#[test]
fn infer_y_from_horizontal_neighbor_target_left_of_reference() {
    let reference = make_screen((1920, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 50), (1920, 1080), (1920, 1080));
    assert_eq!(
        infer_physical_y_from_horizontal_neighbor(&reference, &target, 0),
        Some(50)
    );
}

#[test]
fn infer_y_from_horizontal_neighbor_high_dpi_scales_gap() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((1920, 100), (1707, 960), (2560, 1440));
    let result = infer_physical_y_from_horizontal_neighbor(&reference, &target, 0);
    // gap=100, scale=1440/960=1.5, so physical gap = 150
    assert_eq!(result, Some(150));
}

// --- infer_physical_x_from_vertical_neighbor tests ---

#[test]
fn infer_x_from_vertical_neighbor_adjacent_same_x() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 1080), (1920, 1080), (1920, 1080));
    assert_eq!(
        infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
        Some(0)
    );
}

#[test]
fn infer_x_from_vertical_neighbor_not_adjacent_returns_none() {
    let reference = make_screen((0, 0), (1920, 1080), (1920, 1080));
    let target = make_screen((0, 3000), (1920, 1080), (1920, 1080));
    assert_eq!(
        infer_physical_x_from_vertical_neighbor(&reference, &target, 0),
        None
    );
}

#[test]
fn infer_x_from_vertical_neighbor_target_above_reference() {
    let reference = make_screen((100, 1080), (1920, 1080), (1920, 1080));
    let target = make_screen((200, 0), (1920, 1080), (1920, 1080));
    assert_eq!(
        infer_physical_x_from_vertical_neighbor(&reference, &target, 100),
        Some(200)
    );
}

// --- resolve_recording_selection additional tests ---

#[test]
fn resolve_recording_selection_region_exceeds_single_monitor() {
    let layouts = vec![make_layout((0, 0), (1920, 1080), (0, 0), (1920, 1080))];
    let result = resolve_recording_selection(
        &layouts,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 2000,
            physical_height: 1080,
        },
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("single monitor"));
}

#[test]
fn resolve_recording_selection_empty_layouts() {
    let result = resolve_recording_selection(
        &[],
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        },
    );
    assert!(result.is_err());
}
