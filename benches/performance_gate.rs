//! Stable, deterministic benchmarks used by the pull-request performance gate.

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
use rustshot::{capture, record, selection};
use std::hint::black_box;
use std::time::Duration;

const WIDTH_1080P: u32 = 1920;
const HEIGHT_1080P: u32 = 1080;
const WIDTH_4K: u32 = 3840;
const HEIGHT_4K: u32 = 2160;

fn patterned_bytes(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index.wrapping_mul(31).wrapping_add(17)) & 0xff) as u8)
        .collect()
}

fn performance_gate(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_gate");
    group.sampling_mode(SamplingMode::Flat);

    let screenshot = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    let mut overlay = selection::OverlayRedrawBenchmark::new(
        screenshot,
        (WIDTH_1080P, HEIGHT_1080P),
        (WIDTH_1080P, HEIGHT_1080P),
    );
    // Steady-state cursor-move redraw. Deliberately declared before any
    // throughput is set on the group: an incremental repaint touches a crosshair
    // and its predecessor, not the frame, so a full-frame byte rate would be
    // meaningless here.
    group.bench_function("selection_overlay_redraw_1080p", |b| {
        b.iter(|| {
            overlay.redraw_crosshair(black_box((960, 540)));
            black_box(overlay.output());
        });
    });

    let mut bgra_4k = patterned_bytes(WIDTH_4K as usize * HEIGHT_4K as usize * 4);
    group.throughput(Throughput::Bytes(
        u64::from(WIDTH_4K) * u64::from(HEIGHT_4K) * 4,
    ));
    group.bench_function("bgra_to_rgba_4k", |b| {
        b.iter(|| capture::bgra_to_rgba(black_box(&mut bgra_4k)));
    });

    let native_frame = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    let mut media_foundation_frame = vec![0; native_frame.len()];
    group.throughput(Throughput::Bytes(native_frame.len() as u64));
    group.bench_function("recording_frame_copy_1080p", |b| {
        b.iter(|| {
            record::benchmark_copy_bgra_frame_exact_fit(
                black_box(&native_frame),
                WIDTH_1080P,
                HEIGHT_1080P,
                black_box(&mut media_foundation_frame),
            )
            .unwrap();
        });
    });

    // The screenshot conversion path: every captured monitor frame is turned
    // from the capture backend's bottom-up BGRA into top-down RGBA through here,
    // so its cost is part of how long a PrintScreen takes. Measured with both
    // buffers preallocated, because the caller reuses them and an allocation
    // inside the timing loop would hide the conversion itself.
    let captured_frame = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    let mut converted_frame = vec![0u8; captured_frame.len()];
    group.throughput(Throughput::Bytes(captured_frame.len() as u64));
    group.bench_function("screenshot_bgra_to_rgba_1080p", |b| {
        b.iter(|| {
            record::rgba_to_bgra_bottom_up_into(
                black_box(&captured_frame),
                black_box(&mut converted_frame),
                WIDTH_1080P,
                HEIGHT_1080P,
            )
            .unwrap();
        });
    });

    // 4K matters more than 1080p here: this conversion is per-pixel work on the
    // critical path of a single keypress, so it grows with monitor size.
    let captured_4k = patterned_bytes(WIDTH_4K as usize * HEIGHT_4K as usize * 4);
    let mut converted_4k = vec![0u8; captured_4k.len()];
    group.throughput(Throughput::Bytes(captured_4k.len() as u64));
    group.bench_function("screenshot_bgra_to_rgba_4k", |b| {
        b.iter(|| {
            record::rgba_to_bgra_bottom_up_into(
                black_box(&captured_4k),
                black_box(&mut converted_4k),
                WIDTH_4K,
                HEIGHT_4K,
            )
            .unwrap();
        });
    });

    // Overlay startup: converting the frozen screenshot into softbuffer pixels
    // is what the user waits through before the selection overlay appears. The
    // cold case allocates; the warm case is what each subsequent resize costs.
    let overlay_screenshot = patterned_bytes(WIDTH_1080P as usize * HEIGHT_1080P as usize * 4);
    group.throughput(Throughput::Bytes(overlay_screenshot.len() as u64));
    group.bench_function("selection_overlay_background_1080p", |b| {
        b.iter(|| {
            black_box(selection::benchmark_build_overlay_background(
                black_box(&overlay_screenshot),
                (WIDTH_1080P, HEIGHT_1080P),
                (WIDTH_1080P, HEIGHT_1080P),
            ));
        });
    });

    let mut overlay_background = selection::OverlayBackgroundBenchmark::new(
        overlay_screenshot,
        (WIDTH_1080P, HEIGHT_1080P),
        (WIDTH_1080P, HEIGHT_1080P),
    );
    group.bench_function("selection_overlay_background_rebuild_1080p", |b| {
        b.iter(|| black_box(overlay_background.rebuild()));
    });

    // Appended last on purpose: inserting a benchmark ahead of the three above
    // changes the cache state they start from, which shows up as a phantom
    // regression against their recorded baselines.
    group.throughput(Throughput::Bytes(
        u64::from(WIDTH_1080P) * u64::from(HEIGHT_1080P) * 4,
    ));
    group.bench_function("selection_overlay_first_frame_1080p", |b| {
        b.iter(|| {
            overlay.compose_full_frame(black_box((960, 540)));
            black_box(overlay.output());
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(40)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .noise_threshold(0.05)
        .significance_level(0.05);
    targets = performance_gate
}
criterion_main!(benches);
