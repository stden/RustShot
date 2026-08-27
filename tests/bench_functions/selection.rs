#[test]
fn build_region_from_drag_valid_drag_returns_region() {
    let region = rustshot::selection::build_region_from_drag((10, 20), (50, 80)).unwrap();
    assert_eq!(region.physical_x, 10);
    assert_eq!(region.physical_y, 20);
    assert_eq!(region.physical_width, 40);
    assert_eq!(region.physical_height, 60);
}

#[test]
fn build_region_from_drag_too_small_returns_none() {
    assert!(rustshot::selection::build_region_from_drag((10, 20), (15, 25)).is_none());
}

#[test]
fn virtual_desktop_scale_factor_prefers_primary_screen() {
    let scale = rustshot::selection::virtual_desktop_scale_factor([
        (false, 1.25),
        (true, 1.5),
        (false, 2.0),
    ]);
    assert_eq!(scale, Some(1.5));
}

#[test]
fn virtual_desktop_scale_factor_empty_returns_none() {
    assert_eq!(
        rustshot::selection::virtual_desktop_scale_factor(Vec::<(bool, f32)>::new()),
        None
    );
}

#[test]
fn logical_to_physical_scales_negative_values() {
    assert_eq!(rustshot::selection::logical_to_physical(-100, 1.5), -150);
}

#[test]
fn find_screen_index_at_position_hits_second_screen() {
    let screens = [(0, 0, 1920_u32, 1080_u32), (1920, 0, 2560_u32, 1440_u32)];
    assert_eq!(
        rustshot::selection::find_screen_index_at_position(screens, (2500, 500)),
        Some(1)
    );
}

#[test]
fn find_screen_index_at_position_empty_screens_returns_none() {
    assert_eq!(
        rustshot::selection::find_screen_index_at_position(
            Vec::<(i32, i32, u32, u32)>::new(),
            (0, 0)
        ),
        None
    );
}

#[test]
fn preferred_screen_index_cursor_hit_beats_primary_fallback() {
    let screens = [
        (0, 0, 1920_u32, 1080_u32, true),
        (1920, 0, 2560_u32, 1440_u32, false),
    ];
    assert_eq!(
        rustshot::selection::preferred_screen_index(screens, Some((2500, 500))),
        Some(1)
    );
}

#[test]
fn preferred_screen_index_no_cursor_falls_back_to_primary() {
    let screens = [
        (0, 0, 1920_u32, 1080_u32, false),
        (1920, 0, 2560_u32, 1440_u32, true),
    ];
    assert_eq!(
        rustshot::selection::preferred_screen_index(screens, None),
        Some(1)
    );
}

#[test]
fn benchmark_scaled_physical_dimension_uses_ceil() {
    assert_eq!(
        rustshot::selection::benchmark_scaled_physical_dimension(1000, 1.333).unwrap(),
        1333
    );
}

#[test]
fn benchmark_scaled_physical_dimension_zero_scale_is_error() {
    let err = rustshot::selection::benchmark_scaled_physical_dimension(1000, 0.0).unwrap_err();
    assert!(err.contains("positive"));
}

#[test]
fn benchmark_build_screen_layout_inputs_reports_target_and_scaled_sizes() {
    let sizes = rustshot::selection::benchmark_build_screen_layout_inputs_physical_sizes(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1707, 960, 1.5, false),
        ],
        1,
        (2560, 1440),
    )
    .unwrap();

    assert_eq!(sizes, vec![(1920, 1080), (2560, 1440)]);
}

#[test]
fn benchmark_build_screen_layout_inputs_out_of_range_screen_index_is_error() {
    let err = rustshot::selection::benchmark_build_screen_layout_inputs_physical_sizes(
        [(0, 0, 1920, 1080, 1.0, true)],
        1,
        (1920, 1080),
    )
    .unwrap_err();

    assert!(err.contains("Unknown screen index"));
}

#[test]
fn benchmark_build_screen_layout_inputs_invalid_non_target_scale_is_error() {
    let err = rustshot::selection::benchmark_build_screen_layout_inputs_physical_sizes(
        [
            (0, 0, 1920, 1080, 1.0, true),
            (1920, 0, 1707, 960, 0.0, false),
        ],
        0,
        (1920, 1080),
    )
    .unwrap_err();

    assert!(err.contains("positive"));
}

// ---------------------------------------------------------------------------
// selection layout benchmark helpers (consumed by benches/benchmarks/selection_layout.rs)
// ---------------------------------------------------------------------------

#[test]
fn benchmark_build_physical_screen_layouts_places_horizontal_neighbor() {
    // Primary anchored at physical (0, 0); the 1x right neighbour is placed at
    // the primary's physical right edge.
    let layouts = rustshot::selection::benchmark_build_physical_screen_layouts(&[
        (0, 0, 1920, 1080, 1920, 1080, true),
        (1920, 0, 1920, 1080, 1920, 1080, false),
    ])
    .unwrap();
    assert_eq!(layouts, vec![(0, 0, 1920, 1080), (1920, 0, 1920, 1080)]);

    let err = rustshot::selection::benchmark_build_physical_screen_layouts(&[]).unwrap_err();
    assert!(err.contains("No screens"));
}

#[test]
fn benchmark_normalize_physical_screen_layouts_shifts_negative_origin() {
    // Left-of-primary monitor gives a negative physical origin; normalization
    // shifts everything so the virtual desktop starts at (0, 0) and reports
    // the original origin plus the total desktop size.
    let (normalized, origin_x, origin_y, width, height) =
        rustshot::selection::benchmark_normalize_physical_screen_layouts(&[
            (0, 0, 1920, 1080, 1920, 1080, true),
            (-1920, 0, 1920, 1080, 1920, 1080, false),
        ])
        .unwrap();
    assert_eq!(normalized, vec![(1920, 0, 1920, 1080), (0, 0, 1920, 1080)]);
    assert_eq!(origin_x, -1920);
    assert_eq!(origin_y, 0);
    assert_eq!(width, 3840);
    assert_eq!(height, 1080);
}

#[test]
fn benchmark_resolve_recording_selection_translates_to_screen_local() {
    // Same two-monitor layout as the resolve_recording_selection benchmark.
    let layouts = [
        (0, 0, 1920, 1080, 0, 0, 1920u32, 1080u32),
        (1920, 0, 2560, 1440, 1920, 0, 2560u32, 1440u32),
    ];

    // Region inside the first screen keeps its coordinates.
    assert_eq!(
        rustshot::selection::benchmark_resolve_recording_selection(&layouts, (100, 100, 800, 600))
            .unwrap(),
        (0, (100, 100, 800, 600))
    );

    // Region inside the second screen is translated to that screen's origin.
    assert_eq!(
        rustshot::selection::benchmark_resolve_recording_selection(&layouts, (2000, 100, 800, 600))
            .unwrap(),
        (1, (80, 100, 800, 600))
    );
}

#[test]
fn benchmark_resolve_recording_selection_rejects_cross_monitor_region() {
    let layouts = [
        (0, 0, 1920, 1080, 0, 0, 1920u32, 1080u32),
        (1920, 0, 2560, 1440, 1920, 0, 2560u32, 1440u32),
    ];
    let err =
        rustshot::selection::benchmark_resolve_recording_selection(&layouts, (1800, 100, 400, 600))
            .unwrap_err();
    assert!(
        err.contains("single monitor"),
        "error should explain the single-monitor rule: {err}"
    );
}

#[test]
fn benchmark_ranges_overlap_distinguishes_touching_from_overlapping() {
    assert!(rustshot::selection::benchmark_ranges_overlap(
        0, 100, 50, 100
    ));
    assert!(rustshot::selection::benchmark_ranges_overlap(
        -100, 200, 50, 100
    ));
    // Touching at a boundary is not overlap.
    assert!(!rustshot::selection::benchmark_ranges_overlap(
        0, 100, 100, 100
    ));
    assert!(!rustshot::selection::benchmark_ranges_overlap(
        0, 100, 200, 100
    ));
}

#[test]
fn benchmark_monitor_scale_ratio_computes_physical_over_logical() {
    assert_eq!(
        rustshot::selection::benchmark_monitor_scale_ratio(1920, 1920),
        1.0
    );
    assert_eq!(
        rustshot::selection::benchmark_monitor_scale_ratio(1920, 3840),
        2.0
    );
    // Zero logical size falls back to 1.0 instead of dividing by zero.
    assert_eq!(
        rustshot::selection::benchmark_monitor_scale_ratio(0, 1920),
        1.0
    );
}

#[test]
fn benchmark_scaled_gap_scales_and_rounds() {
    assert_eq!(rustshot::selection::benchmark_scaled_gap(0, 1920, 1920), 0);
    assert_eq!(
        rustshot::selection::benchmark_scaled_gap(100, 1920, 3840),
        200
    );
    assert_eq!(
        rustshot::selection::benchmark_scaled_gap(-50, 1920, 1920),
        -50
    );
    // Zero logical size means scale 1.0, so the gap is unchanged.
    assert_eq!(rustshot::selection::benchmark_scaled_gap(75, 0, 1920), 75);
}

// ---------------------------------------------------------------------------
// Overlay benchmark fixtures
//
// `benches/benchmarks.rs` drives these, so a change that breaks one is only
// discovered when someone runs the benchmarks. These smoke tests assert the
// fixtures produce the frame they claim to, which is what makes a benchmark
// number mean anything.
// ---------------------------------------------------------------------------

/// A `width` x `height` RGBA screenshot with a distinguishable first pixel.
fn rgba_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; (width * height * 4) as usize];
    bytes[0..4].copy_from_slice(&[255, 0, 0, 255]);
    bytes
}

#[test]
fn benchmark_build_overlay_background_returns_the_overlay_pixel_count() {
    let screenshot = rgba_fixture(4, 4);
    assert_eq!(
        rustshot::selection::benchmark_build_overlay_background(&screenshot, (4, 4), (2, 2)),
        4
    );
}

#[test]
fn overlay_background_benchmark_rebuilds_into_the_same_sized_buffer() {
    let screenshot = rgba_fixture(4, 4);
    let mut fixture =
        rustshot::selection::OverlayBackgroundBenchmark::new(screenshot, (4, 4), (2, 2));

    // The point of the fixture is that a rebuild reuses the buffer, so every
    // call must report the same length rather than growing it.
    assert_eq!(fixture.rebuild(), 4);
    assert_eq!(fixture.rebuild(), 4);
}

#[test]
fn overlay_redraw_benchmark_matches_a_full_compose_after_an_incremental_redraw() {
    let screenshot = rgba_fixture(4, 4);
    let mut incremental =
        rustshot::selection::OverlayRedrawBenchmark::new(screenshot.clone(), (4, 4), (4, 4));
    let mut full = rustshot::selection::OverlayRedrawBenchmark::new(screenshot, (4, 4), (4, 4));

    // Steady state: compose once, then move the crosshair incrementally.
    incremental.compose_full_frame((0, 0));
    incremental.redraw_crosshair((2, 3));

    // Reference: compose the destination position from scratch.
    full.compose_full_frame((2, 3));

    assert_eq!(incremental.output(), full.output());
    assert_eq!(incremental.output().len(), 16);
}
