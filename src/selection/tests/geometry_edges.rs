//! Edge-case tests for scale factors, screen lookup, ranges, and gaps.

use super::*;

// --- virtual_desktop_scale_factor edge cases ---

#[test]
fn virtual_desktop_scale_factor_single_primary_2x() {
    let scale = virtual_desktop_scale_factor([(true, 2.0)]);
    assert_eq!(scale, Some(2.0));
}

#[test]
fn virtual_desktop_scale_factor_single_non_primary_1_5x() {
    let scale = virtual_desktop_scale_factor([(false, 1.5)]);
    assert_eq!(scale, Some(1.5));
}

#[test]
fn virtual_desktop_scale_factor_primary_in_middle_of_four() {
    let scale =
        virtual_desktop_scale_factor([(false, 1.0), (false, 1.25), (true, 1.5), (false, 2.0)]);
    assert_eq!(scale, Some(1.5));
}

#[test]
fn virtual_desktop_scale_factor_two_primaries_returns_first() {
    let scale = virtual_desktop_scale_factor([(true, 1.0), (true, 2.0)]);
    assert_eq!(scale, Some(1.0));
}

// --- find_screen_index_at_position edge cases ---

#[test]
fn find_screen_index_at_position_exact_top_left_corner() {
    let screens = vec![(0, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (0, 0)), Some(0));
}

#[test]
fn find_screen_index_at_position_just_outside_right_boundary() {
    let screens = vec![(0, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (1920, 0)), None);
}

#[test]
fn find_screen_index_at_position_just_outside_bottom_boundary() {
    let screens = vec![(0, 0, 1920, 1080)];
    assert_eq!(find_screen_index_at_position(screens, (0, 1080)), None);
}

#[test]
fn find_screen_index_at_position_last_valid_pixel() {
    let screens = vec![(0, 0, 1920, 1080)];
    assert_eq!(
        find_screen_index_at_position(screens, (1919, 1079)),
        Some(0)
    );
}

#[test]
fn find_screen_index_at_position_empty_screens() {
    let screens: Vec<(i32, i32, u32, u32)> = vec![];
    assert_eq!(find_screen_index_at_position(screens, (100, 100)), None);
}

#[test]
fn find_screen_index_at_position_stacked_monitors() {
    let screens = vec![
        (0, 0, 1920, 1080),    // top
        (0, 1080, 1920, 1080), // bottom
    ];
    assert_eq!(
        find_screen_index_at_position(screens.clone(), (500, 500)),
        Some(0)
    );
    assert_eq!(find_screen_index_at_position(screens, (500, 1500)), Some(1));
}

#[test]
fn find_screen_index_at_position_gap_between_monitors_large_suite() {
    let screens = vec![
        (0, 0, 1920, 1080),
        (2000, 0, 1920, 1080), // 80px gap
    ];
    assert_eq!(find_screen_index_at_position(screens, (1950, 500)), None);
}

#[test]
fn build_physical_screen_layouts_places_high_dpi_monitor_left_of_primary() {
    let layouts = build_physical_screen_layouts(&[
        ScreenLayoutInput {
            logical_x: -1707,
            logical_y: 0,
            logical_width: 1707,
            logical_height: 960,
            physical_width: 2560,
            physical_height: 1440,
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
    ])
    .expect("layout should build");

    assert_eq!(layouts[0].physical_x, -2560);
    assert_eq!(layouts[0].physical_y, 0);
    assert_eq!(layouts[1].physical_x, 0);
    assert_eq!(layouts[1].physical_y, 0);
}

#[test]
fn build_physical_screen_layouts_keeps_primary_aligned_to_high_dpi_monitor_on_right() {
    let layouts = build_physical_screen_layouts(&[
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
    ])
    .expect("layout should build");

    assert_eq!(layouts[0].physical_x, 0);
    assert_eq!(layouts[1].physical_x, 1920);
    assert_eq!(layouts[1].physical_y, 0);
}

#[test]
fn resolve_recording_selection_rejects_regions_crossing_monitors() {
    let layouts = vec![
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
        PhysicalScreenLayout {
            logical_x: 1920,
            logical_y: 0,
            logical_width: 1707,
            logical_height: 960,
            physical_x: 1920,
            physical_y: 0,
            physical_width: 2560,
            physical_height: 1440,
        },
    ];

    let error = resolve_recording_selection(
        &layouts,
        Region {
            physical_x: 1910,
            physical_y: 100,
            physical_width: 40,
            physical_height: 200,
        },
    )
    .unwrap_err();

    assert!(error.contains("single monitor"));
}

#[test]
fn resolve_recording_selection_returns_screen_local_physical_region() {
    let layouts = vec![
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
        PhysicalScreenLayout {
            logical_x: 1920,
            logical_y: 0,
            logical_width: 1707,
            logical_height: 960,
            physical_x: 1920,
            physical_y: 0,
            physical_width: 2560,
            physical_height: 1440,
        },
    ];

    let (screen_index, local_region) = resolve_recording_selection(
        &layouts,
        Region {
            physical_x: 2000,
            physical_y: 100,
            physical_width: 400,
            physical_height: 300,
        },
    )
    .expect("region should resolve inside the right monitor");

    assert_eq!(screen_index, 1);
    assert_eq!(local_region.physical_x, 80);
    assert_eq!(local_region.physical_y, 100);
    assert_eq!(local_region.physical_width, 400);
    assert_eq!(local_region.physical_height, 300);
}

// --- logical_end / physical_end tests ---

#[test]
fn logical_end_zero_start_zero_size() {
    assert_eq!(logical_end(0, 0), 0);
}

#[test]
fn logical_end_positive_start_and_size() {
    assert_eq!(logical_end(100, 1920), 2020);
}

#[test]
fn logical_end_negative_start() {
    assert_eq!(logical_end(-500, 1000), 500);
}

#[test]
fn logical_end_large_values_no_overflow() {
    assert_eq!(
        logical_end(i32::MAX, u32::MAX),
        i64::from(i32::MAX) + i64::from(u32::MAX)
    );
}

#[test]
fn physical_end_matches_logical_end_behavior() {
    assert_eq!(physical_end(10, 20), logical_end(10, 20));
}

// --- ranges_overlap tests ---

#[test]
fn ranges_overlap_identical_ranges() {
    assert!(ranges_overlap(0, 100, 0, 100));
}

#[test]
fn ranges_overlap_touching_ranges_do_not_overlap() {
    assert!(!ranges_overlap(0, 100, 100, 100));
}

#[test]
fn ranges_overlap_adjacent_reversed_do_not_overlap() {
    assert!(!ranges_overlap(100, 100, 0, 100));
}

#[test]
fn ranges_overlap_partial_overlap() {
    assert!(ranges_overlap(0, 150, 100, 100));
}

#[test]
fn ranges_overlap_one_inside_another() {
    assert!(ranges_overlap(0, 1000, 100, 200));
}

#[test]
fn ranges_overlap_disjoint_with_gap() {
    assert!(!ranges_overlap(0, 100, 200, 100));
}

#[test]
fn ranges_overlap_negative_start() {
    assert!(ranges_overlap(-100, 200, 0, 100));
}

#[test]
fn ranges_overlap_both_negative() {
    assert!(ranges_overlap(-200, 150, -100, 150));
}

#[test]
fn ranges_overlap_zero_size_never_overlaps() {
    assert!(!ranges_overlap(50, 0, 50, 100));
}

// --- monitor_scale_ratio tests ---

#[test]
fn monitor_scale_ratio_1x_scaling() {
    assert_eq!(monitor_scale_ratio(1920, 1920), 1.0);
}

#[test]
fn monitor_scale_ratio_2x_scaling() {
    assert_eq!(monitor_scale_ratio(960, 1920), 2.0);
}

#[test]
fn monitor_scale_ratio_150_percent() {
    assert!((monitor_scale_ratio(1707, 2560) - 1.5).abs() < 0.01);
}

#[test]
fn monitor_scale_ratio_zero_logical_returns_1() {
    assert_eq!(monitor_scale_ratio(0, 1920), 1.0);
}

#[test]
fn monitor_scale_ratio_both_zero_returns_1() {
    assert_eq!(monitor_scale_ratio(0, 0), 1.0);
}

// --- scaled_gap tests ---

#[test]
fn scaled_gap_zero_gap() {
    assert_eq!(scaled_gap(0, 1920, 1920), 0);
}

#[test]
fn scaled_gap_1x_scaling() {
    assert_eq!(scaled_gap(100, 1920, 1920), 100);
}

#[test]
fn scaled_gap_2x_scaling() {
    assert_eq!(scaled_gap(50, 960, 1920), 100);
}

#[test]
fn scaled_gap_negative_gap() {
    assert_eq!(scaled_gap(-100, 1920, 1920), -100);
}

#[test]
fn scaled_gap_zero_logical_size_returns_gap_unchanged() {
    assert_eq!(scaled_gap(50, 0, 1920), 50);
}
