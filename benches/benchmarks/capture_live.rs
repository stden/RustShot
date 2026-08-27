//! Benchmarks — capture: live screen/window capture (skips headless).

use crate::{benchmark_screens, benchmark_skip};
use criterion::Criterion;
use image::RgbaImage;
use rustshot::capture;
use rustshot::capture::Format;
use std::hint::black_box;

// ---------------------------------------------------------------------------
// capture::capture_screenshot (end-to-end: capture screen + save to file)
// ---------------------------------------------------------------------------

pub(crate) fn bench_capture_screenshot(c: &mut Criterion) {
    let dir = std::env::temp_dir().join("rustshot_bench");
    std::fs::create_dir_all(&dir).ok();

    let screen_count = match capture::benchmark_screen_count_from_result(benchmark_screens()) {
        Ok(screen_count) => screen_count,
        Err(err) => {
            benchmark_skip("capture_screenshot", &err);
            std::fs::remove_dir_all(&dir).ok();
            return;
        }
    };

    if let Err(err) = capture::capture_screenshot_in_dir(None, Format::Png, None, &dir) {
        benchmark_skip("capture_screenshot", &err);
        std::fs::remove_dir_all(&dir).ok();
        return;
    }
    for index in 0..screen_count {
        if let Err(err) = capture::capture_screenshot_in_dir(None, Format::Png, Some(index), &dir) {
            benchmark_skip("capture_screenshot", &err);
            std::fs::remove_dir_all(&dir).ok();
            return;
        }
    }

    let mut group = c.benchmark_group("capture_screenshot");

    // Benchmark: default screen (cursor-based or primary)
    group.bench_function("default_screen", |b| {
        b.iter(|| {
            capture::capture_screenshot_in_dir(
                black_box(None),
                black_box(Format::Png),
                black_box(None),
                black_box(&dir),
            )
            .unwrap();
        });
    });

    // Benchmark: each screen individually
    for i in 0..screen_count {
        group.bench_function(format!("screen_{}", i), |b| {
            b.iter(|| {
                capture::capture_screenshot_in_dir(
                    black_box(None),
                    black_box(Format::Png),
                    black_box(Some(i)),
                    black_box(&dir),
                )
                .unwrap();
            });
        });
    }

    // Benchmark: all screens sequentially
    group.bench_function("all_screens", |b| {
        b.iter(|| {
            for i in 0..screen_count {
                capture::capture_screenshot_in_dir(
                    black_box(None),
                    black_box(Format::Png),
                    black_box(Some(i)),
                    black_box(&dir),
                )
                .unwrap();
            }
        });
    });

    group.finish();

    // Cleanup
    std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------
// capture_screenshot_stages — per-stage profiling of the screenshot pipeline
// ---------------------------------------------------------------------------

pub(crate) fn bench_capture_screenshot_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("capture_screenshot_stages");

    // Stage 1: production monitor capture only (no encoding or file I/O).
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => {
            if let Err(err) = capture::benchmark_capture_monitor_image(&screen) {
                benchmark_skip("capture_screenshot_stages/monitor_capture_only", &err);
                group.finish();
                return;
            }
            group.bench_function("screen_capture_only", |b| {
                b.iter(|| {
                    black_box(capture::benchmark_capture_monitor_image(black_box(&screen)))
                        .unwrap();
                });
            });
        }
        Err(err) => benchmark_skip("capture_screenshot_stages/screen_capture_only", &err),
    }

    // Encoding stages on a synthetic 1920x1080 image.
    let synth = RgbaImage::from_pixel(1920, 1080, image::Rgba([100, 150, 200, 255]));
    let synth_rgb = capture::benchmark_rgb_image_from_rgba(&synth);
    let bench_dir = std::env::temp_dir().join("rustshot_bench_stages");
    std::fs::create_dir_all(&bench_dir).ok();

    // save_png_default — image.save_with_format(Png)
    group.bench_function("save_png_default", |b| {
        let path = bench_dir.join("bench_default.png");
        b.iter(|| {
            synth
                .save_with_format(black_box(&path), image::ImageFormat::Png)
                .unwrap();
        });
    });

    // save_png_fast — PngEncoder with CompressionType::Fast, FilterType::Sub
    group.bench_function("save_png_fast", |b| {
        let path = bench_dir.join("bench_fast.png");
        b.iter(|| {
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            use std::io::BufWriter;
            let file = std::fs::File::create(black_box(&path)).unwrap();
            let writer = BufWriter::new(file);
            let encoder =
                PngEncoder::new_with_quality(writer, CompressionType::Fast, FilterType::Sub);
            synth.write_with_encoder(encoder).unwrap();
        });
    });

    // save_png_best — PngEncoder with BestCompression, Adaptive filter
    group.bench_function("save_png_best", |b| {
        let path = bench_dir.join("bench_best.png");
        b.iter(|| {
            use image::codecs::png::{CompressionType, FilterType, PngEncoder};
            use std::io::BufWriter;
            let file = std::fs::File::create(black_box(&path)).unwrap();
            let writer = BufWriter::new(file);
            let encoder =
                PngEncoder::new_with_quality(writer, CompressionType::Best, FilterType::Adaptive);
            synth.write_with_encoder(encoder).unwrap();
        });
    });

    // save_jpg_default — image.save_with_format(Jpeg)
    group.bench_function("save_jpg_default", |b| {
        let path = bench_dir.join("bench_default.jpg");
        b.iter(|| {
            synth_rgb
                .save_with_format(black_box(&path), image::ImageFormat::Jpeg)
                .unwrap();
        });
    });

    // save_jpg_q75 — JpegEncoder with quality 75
    group.bench_function("save_jpg_q75", |b| {
        let path = bench_dir.join("bench_q75.jpg");
        b.iter(|| {
            use image::codecs::jpeg::JpegEncoder;
            use std::io::BufWriter;
            let file = std::fs::File::create(black_box(&path)).unwrap();
            let writer = BufWriter::new(file);
            let encoder = JpegEncoder::new_with_quality(writer, 75);
            synth_rgb.write_with_encoder(encoder).unwrap();
        });
    });

    // save_bmp — image.save_with_format(Bmp)
    group.bench_function("save_bmp", |b| {
        let path = bench_dir.join("bench.bmp");
        b.iter(|| {
            synth
                .save_with_format(black_box(&path), image::ImageFormat::Bmp)
                .unwrap();
        });
    });

    group.finish();
    std::fs::remove_dir_all(&bench_dir).ok();
}

// ---------------------------------------------------------------------------
// capture::capture_active_window_image (WGC backend with GDI fallback)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn bench_capture_active_window(c: &mut Criterion) {
    // Probe once: headless environments and non-Windows platforms return an
    // error here, and the WGC-vs-GDI routing happens inside the call.
    if let Err(err) = capture::capture_active_window_image() {
        benchmark_skip("capture_active_window", &err);
        return;
    }

    let mut group = c.benchmark_group("capture_active_window");
    group.bench_function("foreground_window", |b| {
        b.iter(|| {
            let _ = black_box(capture::capture_active_window_image());
        });
    });
    group.finish();
}
