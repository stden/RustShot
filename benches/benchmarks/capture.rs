//! Benchmarks — capture: pure helpers (formats, paths, screen-index resolution).

use crate::patterned_bytes;
use criterion::BatchSize;
use criterion::Criterion;
use image::RgbaImage;
use rustshot::capture;
use rustshot::capture::Format;
use std::hint::black_box;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// capture::bgra_to_rgba
// ---------------------------------------------------------------------------

pub(crate) fn bench_bgra_to_rgba(c: &mut Criterion) {
    let mut group = c.benchmark_group("bgra_to_rgba");

    // 4 pixels (16 bytes)
    let mut four_pixels: Vec<u8> = (0..16).collect();
    group.bench_function("4px", |b| {
        b.iter(|| capture::bgra_to_rgba(black_box(&mut four_pixels)));
    });

    // 1K pixels (4 096 bytes)
    let mut one_k_pixels: Vec<u8> = (0..4096).map(|i| (i % 256) as u8).collect();
    group.bench_function("1K_px", |b| {
        b.iter(|| capture::bgra_to_rgba(black_box(&mut one_k_pixels)));
    });

    // 1M pixels (4 194 304 bytes) — roughly a 1024x1024 image
    let mut one_m_pixels = patterned_bytes(4_194_304);
    group.bench_function("1M_px", |b| {
        b.iter(|| capture::bgra_to_rgba(black_box(&mut one_m_pixels)));
    });

    let mut frame_4k = patterned_bytes(3840 * 2160 * 4);
    group.bench_function("4k_frame", |b| {
        b.iter(|| capture::bgra_to_rgba(black_box(&mut frame_4k)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::resolve_screen_index
// ---------------------------------------------------------------------------

pub(crate) fn bench_resolve_screen_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_screen_index");

    group.bench_function("none_requested", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(3), black_box(None)));
    });

    group.bench_function("valid_index", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(3), black_box(Some(1))));
    });

    group.bench_function("out_of_range", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(3), black_box(Some(5))));
    });

    group.bench_function("zero_screens", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(0), black_box(None)));
    });

    group.bench_function("large_valid_index", |b| {
        b.iter(|| capture::resolve_screen_index(black_box(16), black_box(Some(15))));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::default_screen_index
// ---------------------------------------------------------------------------

pub(crate) fn bench_default_screen_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("default_screen_index");

    group.bench_function("primary_first", |b| {
        b.iter(|| capture::default_screen_index(black_box(vec![true, false, false])));
    });

    group.bench_function("primary_last", |b| {
        b.iter(|| capture::default_screen_index(black_box(vec![false, false, true])));
    });

    group.bench_function("no_primary", |b| {
        b.iter(|| capture::default_screen_index(black_box(vec![false, false, false])));
    });

    group.bench_function("primary_after_many_screens", |b| {
        let flags = vec![
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, true,
        ];
        b.iter(|| capture::default_screen_index(black_box(flags.clone())));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::resolve_output_path
// ---------------------------------------------------------------------------

pub(crate) fn bench_resolve_output_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_output_path");

    group.bench_function("with_explicit_path", |b| {
        b.iter(|| {
            capture::resolve_output_path(
                black_box(Some(PathBuf::from("/tmp/shot.png"))),
                black_box(Format::Png),
            )
        });
    });

    group.bench_function("generate_default", |b| {
        b.iter(|| capture::resolve_output_path(black_box(None), black_box(Format::Png)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::resolve_output_path_in_dir
// ---------------------------------------------------------------------------

pub(crate) fn bench_resolve_output_path_in_dir(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_output_path_in_dir");

    let dir = PathBuf::from("/tmp/custom-shots");

    group.bench_function("with_explicit_path", |b| {
        b.iter(|| {
            capture::resolve_output_path_in_dir(
                black_box(Some(PathBuf::from("/tmp/shot.jpg"))),
                black_box(Format::Jpg),
                black_box(&dir),
            )
        });
    });

    group.bench_function("generate_default_png", |b| {
        b.iter(|| {
            capture::resolve_output_path_in_dir(
                black_box(None),
                black_box(Format::Png),
                black_box(&dir),
            )
        });
    });

    group.bench_function("generate_default_bmp", |b| {
        b.iter(|| {
            capture::resolve_output_path_in_dir(
                black_box(None),
                black_box(Format::Bmp),
                black_box(&dir),
            )
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::Format::extension
// ---------------------------------------------------------------------------

pub(crate) fn bench_format_extension(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_extension");

    group.bench_function("png", |b| {
        b.iter(|| {
            let ext = Format::Png.extension();
            black_box(ext);
        });
    });

    group.bench_function("jpg", |b| {
        b.iter(|| {
            let ext = Format::Jpg.extension();
            black_box(ext);
        });
    });

    group.bench_function("bmp", |b| {
        b.iter(|| {
            let ext = Format::Bmp.extension();
            black_box(ext);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::Format::to_image_format
// ---------------------------------------------------------------------------

pub(crate) fn bench_format_to_image_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_to_image_format");

    group.bench_function("png", |b| {
        b.iter(|| black_box(Format::Png).to_image_format());
    });

    group.bench_function("jpg", |b| {
        b.iter(|| black_box(Format::Jpg).to_image_format());
    });

    group.bench_function("bmp", |b| {
        b.iter(|| black_box(Format::Bmp).to_image_format());
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// capture::get_screenshots_dir
// ---------------------------------------------------------------------------

pub(crate) fn bench_get_screenshots_dir(c: &mut Criterion) {
    c.bench_function("get_screenshots_dir", |b| {
        b.iter(capture::get_screenshots_dir);
    });
}

// ---------------------------------------------------------------------------
// capture::list_screens
// ---------------------------------------------------------------------------

pub(crate) fn bench_list_screens(c: &mut Criterion) {
    c.bench_function("list_screens", |b| {
        b.iter(capture::list_screens);
    });
}

// ---------------------------------------------------------------------------
// capture::benchmark_save_image_optimized / benchmark_ensure_parent_directory
// ---------------------------------------------------------------------------

pub(crate) fn bench_save_image_optimized(c: &mut Criterion) {
    let mut group = c.benchmark_group("save_image_optimized");
    let dir = std::env::temp_dir().join("rustshot_bench_save_image_optimized");
    std::fs::create_dir_all(&dir).ok();
    let image = RgbaImage::from_pixel(1920, 1080, image::Rgba([100, 150, 200, 255]));

    group.bench_function("png", |b| {
        let path = dir.join("save.png");
        b.iter(|| {
            capture::benchmark_save_image_optimized(
                black_box(&image),
                black_box(path.as_path()),
                black_box(Format::Png),
            )
            .unwrap();
        });
    });

    group.bench_function("jpg", |b| {
        let path = dir.join("save.jpg");
        b.iter(|| {
            capture::benchmark_save_image_optimized(
                black_box(&image),
                black_box(path.as_path()),
                black_box(Format::Jpg),
            )
            .unwrap();
        });
    });

    group.bench_function("bmp", |b| {
        let path = dir.join("save.bmp");
        b.iter(|| {
            capture::benchmark_save_image_optimized(
                black_box(&image),
                black_box(path.as_path()),
                black_box(Format::Bmp),
            )
            .unwrap();
        });
    });

    group.finish();
    std::fs::remove_dir_all(&dir).ok();
}

pub(crate) fn bench_ensure_parent_directory(c: &mut Criterion) {
    let mut group = c.benchmark_group("ensure_parent_directory");
    let root = std::env::temp_dir().join("rustshot_bench_ensure_parent_directory");

    group.bench_function("existing_parent", |b| {
        let existing_dir = root.join("existing");
        std::fs::create_dir_all(&existing_dir).ok();
        let path = existing_dir.join("shot.png");
        b.iter(|| {
            capture::benchmark_ensure_parent_directory(black_box(path.as_path())).unwrap();
        });
    });

    group.bench_function("create_nested_parent", |b| {
        let nested_root = root.join("recreate");
        let path = nested_root.join("deep/shot.png");
        b.iter_batched(
            || {
                let _ = std::fs::remove_dir_all(&nested_root);
                path.clone()
            },
            |path| {
                capture::benchmark_ensure_parent_directory(black_box(path.as_path())).unwrap();
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
    std::fs::remove_dir_all(&root).ok();
}
