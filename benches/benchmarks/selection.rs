//! Benchmarks — selection: geometry, drag regions, screen lookup.

use criterion::Criterion;
use rustshot::selection;
use std::hint::black_box;

// ---------------------------------------------------------------------------
// selection::build_region_from_drag
// ---------------------------------------------------------------------------

pub(crate) fn bench_build_region_from_drag(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_region_from_drag");

    group.bench_function("valid_region", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((0, 0)), black_box((1920, 1080)));
        });
    });

    group.bench_function("too_small", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((0, 0)), black_box((3, 3)));
        });
    });

    group.bench_function("reversed_coords", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((1920, 1080)), black_box((0, 0)));
        });
    });

    group.bench_function("large_region", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((0, 0)), black_box((3840, 2160)));
        });
    });

    group.bench_function("negative_coordinates", |b| {
        b.iter(|| {
            selection::build_region_from_drag(black_box((-1200, -800)), black_box((-20, -10)));
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::virtual_desktop_scale_factor
// ---------------------------------------------------------------------------

pub(crate) fn bench_virtual_desktop_scale_factor(c: &mut Criterion) {
    let mut group = c.benchmark_group("virtual_desktop_scale_factor");

    group.bench_function("primary_first", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(vec![
                (true, 1.0),
                (false, 2.0),
                (false, 1.5),
            ]));
        });
    });

    group.bench_function("no_primary", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(vec![
                (false, 1.0),
                (false, 2.0),
                (false, 1.5),
            ]));
        });
    });

    group.bench_function("single_screen", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(vec![(true, 1.5)]));
        });
    });

    group.bench_function("empty", |b| {
        b.iter(|| {
            selection::virtual_desktop_scale_factor(black_box(Vec::<(bool, f32)>::new()));
        });
    });

    group.bench_function("many_monitors_primary_last", |b| {
        let screens = vec![
            (false, 1.0),
            (false, 1.25),
            (false, 1.5),
            (false, 2.0),
            (true, 1.75),
        ];
        b.iter(|| selection::virtual_desktop_scale_factor(black_box(screens.clone())));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::logical_to_physical
// ---------------------------------------------------------------------------

pub(crate) fn bench_logical_to_physical(c: &mut Criterion) {
    let mut group = c.benchmark_group("logical_to_physical");

    group.bench_function("scale_1.0", |b| {
        b.iter(|| selection::logical_to_physical(black_box(1920), black_box(1.0)));
    });

    group.bench_function("scale_1.5", |b| {
        b.iter(|| selection::logical_to_physical(black_box(1920), black_box(1.5)));
    });

    group.bench_function("scale_2.0", |b| {
        b.iter(|| selection::logical_to_physical(black_box(1920), black_box(2.0)));
    });

    group.bench_function("negative_value", |b| {
        b.iter(|| selection::logical_to_physical(black_box(-1920), black_box(1.5)));
    });

    group.bench_function("negative_125pct", |b| {
        b.iter(|| selection::logical_to_physical(black_box(-1440), black_box(1.25)));
    });

    group.bench_function("zero_200pct", |b| {
        b.iter(|| selection::logical_to_physical(black_box(0), black_box(2.0)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::find_screen_index_at_position
// ---------------------------------------------------------------------------

pub(crate) fn bench_find_screen_index_at_position(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_screen_index_at_position");

    group.bench_function("single_screen_hit", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32)];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((960, 540)),
            );
        });
    });

    group.bench_function("single_screen_miss", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32)];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((5000, 5000)),
            );
        });
    });

    group.bench_function("two_screens_hit_second", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32), (1920, 0, 2560u32, 1440u32)];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((2500, 500)),
            );
        });
    });

    group.bench_function("four_screens_hit_last", |b| {
        let screens = vec![
            (0, 0, 1920u32, 1080u32),
            (1920, 0, 1920u32, 1080u32),
            (0, 1080, 1920u32, 1080u32),
            (1920, 1080, 1920u32, 1080u32),
        ];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((2500, 1500)),
            );
        });
    });

    group.bench_function("empty_screens", |b| {
        let screens: Vec<(i32, i32, u32, u32)> = vec![];
        b.iter(|| {
            selection::find_screen_index_at_position(
                black_box(screens.clone()),
                black_box((500, 500)),
            );
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::preferred_screen_index
// ---------------------------------------------------------------------------

pub(crate) fn bench_preferred_screen_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("preferred_screen_index");

    group.bench_function("cursor_on_screen", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, true),
            (1920, 0, 2560u32, 1440u32, false),
        ];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((2500, 500))),
            );
        });
    });

    group.bench_function("cursor_miss_fallback_primary", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, false),
            (1920, 0, 2560u32, 1440u32, true),
        ];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((9999, 9999))),
            );
        });
    });

    group.bench_function("no_cursor_fallback_primary", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, false),
            (1920, 0, 2560u32, 1440u32, true),
        ];
        b.iter(|| {
            selection::preferred_screen_index(black_box(screens.clone()), black_box(None));
        });
    });

    group.bench_function("no_cursor_no_primary_fallback_first", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, false),
            (1920, 0, 2560u32, 1440u32, false),
        ];
        b.iter(|| {
            selection::preferred_screen_index(black_box(screens.clone()), black_box(None));
        });
    });

    group.bench_function("three_screens_cursor_on_middle", |b| {
        let screens = vec![
            (0i32, 0i32, 1920u32, 1080u32, true),
            (1920, 0, 2560u32, 1440u32, false),
            (4480, 0, 1920u32, 1080u32, false),
        ];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((3000, 700))),
            );
        });
    });

    group.bench_function("empty_screens", |b| {
        let screens: Vec<(i32, i32, u32, u32, bool)> = vec![];
        b.iter(|| {
            selection::preferred_screen_index(
                black_box(screens.clone()),
                black_box(Some((500, 500))),
            );
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::get_cursor_position
// ---------------------------------------------------------------------------

pub(crate) fn bench_get_cursor_position(c: &mut Criterion) {
    c.bench_function("get_cursor_position", |b| {
        b.iter(selection::get_cursor_position);
    });
}

// ---------------------------------------------------------------------------
// selection::scaled_physical_dimension
// ---------------------------------------------------------------------------

pub(crate) fn bench_scaled_physical_dimension(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaled_physical_dimension");

    group.bench_function("scale_1_0", |b| {
        b.iter(|| selection::benchmark_scaled_physical_dimension(black_box(1920), black_box(1.0)));
    });

    group.bench_function("scale_1_333", |b| {
        b.iter(|| {
            selection::benchmark_scaled_physical_dimension(black_box(1000), black_box(1.333))
        });
    });

    group.bench_function("scale_1_5", |b| {
        b.iter(|| selection::benchmark_scaled_physical_dimension(black_box(1707), black_box(1.5)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// selection::build_screen_layout_inputs_physical_sizes
// ---------------------------------------------------------------------------

pub(crate) fn bench_build_screen_layout_inputs_physical_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_screen_layout_inputs_physical_sizes");

    group.bench_function("single_monitor", |b| {
        let screens = vec![(0, 0, 1920u32, 1080u32, 1.0f32, true)];
        b.iter(|| {
            selection::benchmark_build_screen_layout_inputs_physical_sizes(
                black_box(screens.clone()),
                black_box(0),
                black_box((1920, 1080)),
            )
        });
    });

    group.bench_function("two_monitors_mixed_dpi", |b| {
        let screens = vec![
            (0, 0, 1920u32, 1080u32, 1.0f32, true),
            (1920, 0, 1707u32, 960u32, 1.5f32, false),
        ];
        b.iter(|| {
            selection::benchmark_build_screen_layout_inputs_physical_sizes(
                black_box(screens.clone()),
                black_box(1),
                black_box((2560, 1440)),
            )
        });
    });

    group.bench_function("three_monitors", |b| {
        let screens = vec![
            (-1707, 0, 1707u32, 960u32, 1.5f32, false),
            (0, 0, 1920u32, 1080u32, 1.0f32, true),
            (1920, 0, 1920u32, 1080u32, 1.0f32, false),
        ];
        b.iter(|| {
            selection::benchmark_build_screen_layout_inputs_physical_sizes(
                black_box(screens.clone()),
                black_box(1),
                black_box((1920, 1080)),
            )
        });
    });

    group.finish();
}
