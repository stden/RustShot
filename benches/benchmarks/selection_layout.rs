//! Benchmarks — selection: physical layout inference and normalization.

use criterion::Criterion;
use rustshot::selection;
use std::hint::black_box;

// ---------------------------------------------------------------------------
// selection::build_physical_screen_layouts
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn bench_build_physical_screen_layouts(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_physical_screen_layouts");

    group.bench_function("single_1080p", |b| {
        let inputs = vec![(0, 0, 1920u32, 1080u32, 1920u32, 1080u32, true)];
        b.iter(|| selection::benchmark_build_physical_screen_layouts(black_box(&inputs)));
    });

    group.bench_function("two_horizontal", |b| {
        let inputs = vec![
            (0, 0, 1920, 1080, 1920, 1080, true),
            (1920, 0, 2560, 1440, 2560, 1440, false),
        ];
        b.iter(|| selection::benchmark_build_physical_screen_layouts(black_box(&inputs)));
    });

    group.bench_function("three_mixed_dpi", |b| {
        let inputs = vec![
            (-2560, 0, 1707, 960, 2560, 1440, false),
            (0, 0, 1920, 1080, 1920, 1080, true),
            (1920, 0, 1920, 1080, 1920, 1080, false),
        ];
        b.iter(|| selection::benchmark_build_physical_screen_layouts(black_box(&inputs)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::normalize_physical_screen_layouts
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn bench_normalize_physical_screen_layouts(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_physical_screen_layouts");

    group.bench_function("single_screen", |b| {
        let inputs = vec![(0, 0, 1920u32, 1080u32, 1920u32, 1080u32, true)];
        b.iter(|| selection::benchmark_normalize_physical_screen_layouts(black_box(&inputs)));
    });

    group.bench_function("two_horizontal_with_offset", |b| {
        let inputs = vec![
            (0, 0, 1920, 1080, 1920, 1080, true),
            (1920, 0, 2560, 1440, 2560, 1440, false),
        ];
        b.iter(|| selection::benchmark_normalize_physical_screen_layouts(black_box(&inputs)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::resolve_recording_selection
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn bench_resolve_recording_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_recording_selection");

    let layouts = vec![
        (0, 0, 1920, 1080, 0, 0, 1920u32, 1080u32),
        (1920, 0, 2560, 1440, 1920, 0, 2560u32, 1440u32),
    ];

    group.bench_function("region_in_first_screen", |b| {
        b.iter(|| {
            selection::benchmark_resolve_recording_selection(
                black_box(&layouts),
                black_box((100, 100, 800, 600)),
            )
        });
    });

    group.bench_function("region_in_second_screen", |b| {
        b.iter(|| {
            selection::benchmark_resolve_recording_selection(
                black_box(&layouts),
                black_box((2000, 100, 800, 600)),
            )
        });
    });

    group.bench_function("region_spanning_two_screens_error", |b| {
        b.iter(|| {
            selection::benchmark_resolve_recording_selection(
                black_box(&layouts),
                black_box((1800, 100, 400, 600)),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::ranges_overlap
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn bench_ranges_overlap(c: &mut Criterion) {
    let mut group = c.benchmark_group("ranges_overlap");

    group.bench_function("overlapping", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(0),
                black_box(100),
                black_box(50),
                black_box(100),
            )
        });
    });

    group.bench_function("disjoint", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(0),
                black_box(100),
                black_box(200),
                black_box(100),
            )
        });
    });

    group.bench_function("touching", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(0),
                black_box(100),
                black_box(100),
                black_box(100),
            )
        });
    });

    group.bench_function("negative_start", |b| {
        b.iter(|| {
            selection::benchmark_ranges_overlap(
                black_box(-100),
                black_box(200),
                black_box(50),
                black_box(100),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::monitor_scale_ratio
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn bench_monitor_scale_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("monitor_scale_ratio");

    group.bench_function("1x", |b| {
        b.iter(|| selection::benchmark_monitor_scale_ratio(black_box(1920), black_box(1920)));
    });

    group.bench_function("2x", |b| {
        b.iter(|| selection::benchmark_monitor_scale_ratio(black_box(1920), black_box(3840)));
    });

    group.bench_function("zero_logical", |b| {
        b.iter(|| selection::benchmark_monitor_scale_ratio(black_box(0), black_box(1920)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::scaled_gap
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn bench_scaled_gap(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaled_gap");

    group.bench_function("zero_gap", |b| {
        b.iter(|| selection::benchmark_scaled_gap(black_box(0), black_box(1920), black_box(1920)));
    });

    group.bench_function("positive_gap_2x", |b| {
        b.iter(|| {
            selection::benchmark_scaled_gap(black_box(100), black_box(1920), black_box(3840))
        });
    });

    group.bench_function("negative_gap", |b| {
        b.iter(|| {
            selection::benchmark_scaled_gap(black_box(-50), black_box(1920), black_box(1920))
        });
    });

    group.finish();
}
