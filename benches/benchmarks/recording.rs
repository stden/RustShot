//! Benchmarks — record: timing, pixel conversion, paths, recording stages.

use crate::{benchmark_screens, benchmark_skip, patterned_bytes};
use criterion::Criterion;
use image::RgbaImage;
use rustshot::{capture, record};
use std::hint::black_box;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// record::frame_duration_hns
// ---------------------------------------------------------------------------

pub(crate) fn bench_frame_duration_hns(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_duration_hns");

    group.bench_function("30fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(30)));
    });

    group.bench_function("60fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(60)));
    });

    group.bench_function("0fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(0)));
    });

    group.bench_function("120fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(120)));
    });

    group.bench_function("240fps", |b| {
        b.iter(|| record::frame_duration_hns(black_box(240)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::round_to_even
// ---------------------------------------------------------------------------

pub(crate) fn bench_round_to_even(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_to_even");

    group.bench_function("even_value", |b| {
        b.iter(|| record::round_to_even(black_box(1920)));
    });

    group.bench_function("odd_value", |b| {
        b.iter(|| record::round_to_even(black_box(1921)));
    });

    group.bench_function("zero", |b| {
        b.iter(|| record::round_to_even(black_box(0)));
    });

    group.bench_function("one", |b| {
        b.iter(|| record::round_to_even(black_box(1)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::rgba_to_bgra_bottom_up
// ---------------------------------------------------------------------------

pub(crate) fn bench_rgba_to_bgra_bottom_up(c: &mut Criterion) {
    let mut group = c.benchmark_group("rgba_to_bgra_bottom_up");

    // 1x1
    {
        let data: Vec<u8> = vec![255, 0, 0, 255];
        group.bench_function("1x1", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(1), black_box(1)));
        });
    }

    // 64x64
    {
        let w: u32 = 64;
        let h: u32 = 64;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("64x64", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 640x480
    {
        let w: u32 = 640;
        let h: u32 = 480;
        let data: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i % 256) as u8).collect();
        group.bench_function("640x480", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 1920x1080
    {
        let w: u32 = 1920;
        let h: u32 = 1080;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("1920x1080", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 1919x1079 (odd dimensions)
    {
        let w: u32 = 1919;
        let h: u32 = 1079;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("1919x1079", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 2560x1440
    {
        let w: u32 = 2560;
        let h: u32 = 1440;
        let data: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i % 256) as u8).collect();
        group.bench_function("2560x1440", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    // 3840x2160
    {
        let w: u32 = 3840;
        let h: u32 = 2160;
        let data: Vec<u8> = (0..(w * h * 4) as usize).map(|i| (i % 256) as u8).collect();
        group.bench_function("3840x2160", |b| {
            b.iter(|| record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h)));
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// record::pack_u32_pair
// ---------------------------------------------------------------------------

pub(crate) fn bench_pack_u32_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("pack_u32_pair");

    group.bench_function("frame_size_1920x1080", |b| {
        b.iter(|| record::pack_u32_pair(black_box(1920), black_box(1080)));
    });

    group.bench_function("fps_30_1", |b| {
        b.iter(|| record::pack_u32_pair(black_box(30), black_box(1)));
    });

    group.bench_function("max_values", |b| {
        b.iter(|| record::pack_u32_pair(black_box(u32::MAX), black_box(u32::MAX)));
    });

    group.bench_function("zero_zero", |b| {
        b.iter(|| record::pack_u32_pair(black_box(0), black_box(0)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::validate_recording_start
// ---------------------------------------------------------------------------

pub(crate) fn bench_validate_recording_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate_recording_start");

    group.bench_function("valid_fps", |b| {
        b.iter(|| record::validate_recording_start(black_box(30)));
    });

    group.bench_function("zero_fps", |b| {
        b.iter(|| record::validate_recording_start(black_box(0)));
    });

    group.bench_function("high_fps", |b| {
        b.iter(|| record::validate_recording_start(black_box(240)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// record::get_recording_dir / get_recording_path / get_recording_path_in_dir
// ---------------------------------------------------------------------------

pub(crate) fn bench_get_recording_dir(c: &mut Criterion) {
    c.bench_function("get_recording_dir", |b| {
        b.iter(record::get_recording_dir);
    });
}

pub(crate) fn bench_get_recording_path(c: &mut Criterion) {
    c.bench_function("get_recording_path", |b| {
        b.iter(record::get_recording_path);
    });
}

pub(crate) fn bench_get_recording_path_in_dir(c: &mut Criterion) {
    let dir = PathBuf::from("/tmp/custom-recordings");
    c.bench_function("get_recording_path_in_dir", |b| {
        b.iter(|| record::get_recording_path_in_dir(black_box(&dir)));
    });
}

// ---------------------------------------------------------------------------
// recording_stages — per-stage profiling of the video recording pipeline
// ---------------------------------------------------------------------------

pub(crate) fn bench_recording_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("recording_stages");

    // Stage 1: screen capture raw (same as screenshot, confirms recording context)
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => {
            group.bench_function("screen_capture_raw", |b| {
                b.iter(|| {
                    let _ = black_box(screen.capture());
                });
            });
        }
        Err(err) => benchmark_skip("recording_stages/screen_capture_raw", &err),
    }

    // Stage 2: image_from_raw — RgbaImage::from_raw on a captured frame
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => match screen.capture() {
            Ok(captured) => {
                let w = captured.width();
                let h = captured.height();
                let raw = captured.into_raw();
                group.bench_function("image_from_raw", |b| {
                    b.iter(|| {
                        let _ = black_box(RgbaImage::from_raw(w, h, raw.clone()));
                    });
                });
            }
            Err(err) => benchmark_skip(
                "recording_stages/image_from_raw",
                &format!("Failed to capture screen: {err}"),
            ),
        },
        Err(err) => benchmark_skip("recording_stages/image_from_raw", &err),
    }

    // Stage 3: crop_region — crop a 800x600 region from a 1920x1080 frame
    {
        let frame = RgbaImage::from_pixel(1920, 1080, image::Rgba([100, 150, 200, 255]));
        group.bench_function("crop_region", |b| {
            b.iter(|| {
                let cropped =
                    image::imageops::crop_imm(black_box(&frame), 100, 100, 800, 600).to_image();
                black_box(cropped);
            });
        });
    }

    // Stage 4: pad_to_even — pad a cropped image to even dimensions
    {
        let cropped = RgbaImage::from_pixel(799, 599, image::Rgba([50, 100, 150, 255]));
        let even_w = record::round_to_even(799);
        let even_h = record::round_to_even(599);
        group.bench_function("pad_to_even", |b| {
            b.iter(|| {
                let mut padded = RgbaImage::new(even_w, even_h);
                image::imageops::replace(&mut padded, black_box(&cropped), 0, 0);
                black_box(padded);
            });
        });
    }

    // Stage 5: rgba_to_bgra_bottom_up — 1920x1080
    {
        let w: u32 = 1920;
        let h: u32 = 1080;
        let data = patterned_bytes((w * h * 4) as usize);
        group.bench_function("rgba_to_bgra_bottom_up_1080p", |b| {
            b.iter(|| {
                let _ =
                    record::rgba_to_bgra_bottom_up(black_box(&data), black_box(w), black_box(h));
            });
        });
    }

    // Full per-frame pipeline (capture + convert + crop + pad + color-convert), no WMF write
    match capture::benchmark_first_screen_from_result(benchmark_screens()) {
        Ok(screen) => match screen.capture() {
            Ok(captured) => {
                let info = screen.display_info;
                // Use a region that's a subset of the screen
                let region_w = (info.width / 2).min(800);
                let region_h = (info.height / 2).min(600);
                let even_w = record::round_to_even(region_w);
                let even_h = record::round_to_even(region_h);
                let probe_raw = captured.into_raw();

                group.bench_function("recording_frame_pipeline", |b| {
                    b.iter(|| {
                        if let Ok(captured) = screen.capture() {
                            if let Some(img) = RgbaImage::from_raw(
                                captured.width(),
                                captured.height(),
                                captured.into_raw(),
                            ) {
                                let cropped =
                                    image::imageops::crop_imm(&img, 0, 0, region_w, region_h)
                                        .to_image();
                                let mut padded = RgbaImage::new(even_w, even_h);
                                image::imageops::replace(&mut padded, &cropped, 0, 0);
                                let bgra =
                                    record::rgba_to_bgra_bottom_up(padded.as_raw(), even_w, even_h);
                                let _ = black_box(bgra);
                            } else {
                                black_box(&probe_raw);
                            }
                        } else {
                            black_box(&probe_raw);
                        }
                    });
                });
            }
            Err(err) => benchmark_skip(
                "recording_stages/recording_frame_pipeline",
                &format!("Failed to capture screen: {err}"),
            ),
        },
        Err(err) => benchmark_skip("recording_stages/recording_frame_pipeline", &err),
    }

    group.finish();
}
