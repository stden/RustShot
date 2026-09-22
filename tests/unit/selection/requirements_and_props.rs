//! Task 2.3 requirement tests and property-based tests.

use super::*;

// --- Task 2.3: build_region_from_drag, resolve_recording_selection,
//     find_screen_index_at_position, preferred_screen_index ---
//
// Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8

/// Requirement 3.1: identical start and end points produce None.
#[test]
fn req3_1_build_region_from_drag_identical_start_end_returns_none() {
    assert!(build_region_from_drag((500, 300), (500, 300)).is_none());
}

/// Requirement 3.2: negative coordinates spanning a valid area produce a
/// correct Region with the minimum coordinates as origin.
#[test]
fn req3_2_build_region_from_drag_negative_coords_spanning_valid_area() {
    let region = build_region_from_drag((-50, -30), (60, 80)).expect("should produce a region");
    assert_eq!(region.physical_x, -50, "origin x should be the minimum");
    assert_eq!(region.physical_y, -30, "origin y should be the minimum");
    assert_eq!(region.physical_width, 110);
    assert_eq!(region.physical_height, 110);
}

/// Requirement 3.4: region spanning two monitors returns an error.
#[test]
fn req3_4_resolve_recording_selection_spanning_two_monitors_returns_error() {
    let layouts = two_monitor_layouts();
    // Region crosses the boundary between monitor 0 and monitor 1
    let region = Region {
        physical_x: 1800,
        physical_y: 100,
        physical_width: 300,
        physical_height: 200,
    };
    let err = resolve_recording_selection(&layouts, region).unwrap_err();
    assert!(
        err.contains("single monitor"),
        "error should mention single monitor constraint: {err}"
    );
}

/// Requirement 3.5: region at the exact boundary of a monitor is accepted.
#[test]
fn req3_5_resolve_recording_selection_exact_boundary_accepted() {
    let layouts = vec![make_layout((0, 0), (1920, 1080), (0, 0), (1920, 1080))];
    // Region fills the entire monitor exactly
    let region = Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: 1920,
        physical_height: 1080,
    };
    let (index, local) = resolve_recording_selection(&layouts, region).unwrap();
    assert_eq!(index, 0);
    assert_eq!(local.physical_x, 0);
    assert_eq!(local.physical_y, 0);
    assert_eq!(local.physical_width, 1920);
    assert_eq!(local.physical_height, 1080);
}

/// Requirement 3.7: cursor outside all monitors returns None.
#[test]
fn req3_7_find_screen_index_at_position_cursor_outside_all_monitors() {
    let screens = vec![(0, 0, 1920u32, 1080u32), (1920, 0, 2560u32, 1440u32)];
    assert_eq!(
        find_screen_index_at_position(screens, (9999, 9999)),
        None,
        "cursor far outside all monitors should return None"
    );
}

/// Requirement 3.8: preferred_screen_index with None cursor falls back to
/// the primary monitor.
#[test]
fn req3_8_preferred_screen_index_none_cursor_falls_back_to_primary() {
    let screens = vec![
        (0, 0, 1920, 1080, false),
        (1920, 0, 2560, 1440, true), // primary
    ];
    assert_eq!(
        preferred_screen_index(screens, None),
        Some(1),
        "with no cursor, should fall back to the primary monitor (index 1)"
    );
}

// --- Property-based tests ---

use proptest::prelude::*;

proptest! {
    /// **Validates: Requirements 17.1, 1.8, 1.9**
    ///
    /// For all i32 values and positive f32 scale factors,
    /// logical_to_physical result is within 1 unit of value * scale.
    ///
    /// The function uses f32 arithmetic: `((value as f32) * scale).round() as i32`.
    /// When the f32 product is within i32 range, the result is within 1 unit
    /// of the exact f64 product (accounting for f32 precision loss and rounding).
    /// When the product overflows i32 range, the `as i32` cast saturates to
    /// i32::MIN or i32::MAX, which we accept as correct clamping behavior.
    #[test]
    fn prop_logical_to_physical_rounding_tolerance(
        value in any::<i32>(),
        scale in (0.0f32..1e10).prop_filter("positive scale", |s| *s > 0.0 && s.is_finite()),
    ) {
        let result = logical_to_physical(value, scale);
        let exact = (value as f64) * (scale as f64);

        // When the exact product is outside i32 range, the function saturates
        // via `as i32` cast — that is expected clamping behavior.
        if exact > i32::MAX as f64 + 0.5 {
            prop_assert_eq!(result, i32::MAX,
                "expected saturation to i32::MAX for value={}, scale={}", value, scale);
        } else if exact < i32::MIN as f64 - 0.5 {
            prop_assert_eq!(result, i32::MIN,
                "expected saturation to i32::MIN for value={}, scale={}", value, scale);
        } else {
            // Within representable range: result is within 1 unit of exact product
            let diff = (result as f64 - exact).abs();
            prop_assert!(
                diff <= 1.0,
                "logical_to_physical({}, {}) = {}, exact = {}, diff {} exceeds 1.0",
                value, scale, result, exact, diff
            );
        }
    }

    /// **Validates: Requirements 17.2, 1.1, 1.2, 1.3**
    ///
    /// For all (start_a, size_a, start_b, size_b) tuples, `ranges_overlap`
    /// returns the correct result: two half-open ranges [start, start+size)
    /// overlap iff start_a < start_b + size_b AND start_b < start_a + size_a.
    ///
    /// The reference implementation uses i64 arithmetic to avoid overflow,
    /// matching the production code's use of `logical_end`.
    #[test]
    fn prop_ranges_overlap_correctness(
        start_a in any::<i32>(),
        size_a in any::<u32>(),
        start_b in any::<i32>(),
        size_b in any::<u32>(),
    ) {
        let result = ranges_overlap(start_a, size_a, start_b, size_b);

        // Reference: half-open interval overlap using i64 to avoid overflow
        let end_a = i64::from(start_a) + i64::from(size_a);
        let end_b = i64::from(start_b) + i64::from(size_b);
        let expected = i64::from(start_a) < end_b && i64::from(start_b) < end_a;

        prop_assert_eq!(
            result, expected,
            "ranges_overlap({}, {}, {}, {}) = {}, expected {}",
            start_a, size_a, start_b, size_b, result, expected
        );
    }

    /// Generalizes the `build_region_from_drag` examples: for any two points
    /// within a 4000x4000 overlay, a produced region contains both points,
    /// has positive dimensions, and stays within the overlay bounds. A drag
    /// is rejected only when it spans ≤5 px in either dimension.
    #[test]
    fn prop_build_region_from_drag_contains_points_within_bounds(
        sx in 0i32..4000,
        sy in 0i32..4000,
        ex in 0i32..4000,
        ey in 0i32..4000,
    ) {
        match build_region_from_drag((sx, sy), (ex, ey)) {
            Some(region) => {
                // Dimensions are positive (in fact above the 5 px threshold)
                prop_assert!(region.physical_width > 5);
                prop_assert!(region.physical_height > 5);

                // The region is exactly the normalized drag rectangle …
                let right = region.physical_x + region.physical_width as i32;
                let bottom = region.physical_y + region.physical_height as i32;
                prop_assert_eq!(region.physical_x, sx.min(ex));
                prop_assert_eq!(region.physical_y, sy.min(ey));
                prop_assert_eq!(right, sx.max(ex));
                prop_assert_eq!(bottom, sy.max(ey));

                // … so it contains both points and stays within the overlay.
                prop_assert!(region.physical_x <= sx && sx <= right);
                prop_assert!(region.physical_y <= sy && sy <= bottom);
                prop_assert!(region.physical_x <= ex && ex <= right);
                prop_assert!(region.physical_y <= ey && ey <= bottom);
                prop_assert!(region.physical_x >= 0 && region.physical_y >= 0);
                prop_assert!(right < 4000 && bottom < 4000);
            }
            None => {
                // Rejected only for drags ≤5 px wide or ≤5 px tall
                prop_assert!(
                    (sx - ex).abs() <= 5 || (sy - ey).abs() <= 5,
                    "drag ({}, {}) → ({}, {}) was rejected but exceeds 5 px in both axes",
                    sx, sy, ex, ey
                );
            }
        }
    }
}
