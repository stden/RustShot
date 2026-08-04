//! Tests for frame-timing math ([`frame_duration_hns`], `frame_timing_hns`)
//! and the `recording_bitrate` heuristic.

use crate::record::*;
use std::time::Duration;

fn fps(value: u32) -> Fps {
    Fps::new(value).unwrap()
}

fn dimensions(width: u32, height: u32) -> RecordingDimensions {
    RecordingDimensions::try_from(Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: width,
        physical_height: height,
    })
    .unwrap()
}

fn timing(frame_index: u64, fps_value: u32) -> (i64, i64) {
    let timing = frame_timing_hns(FrameIndex::new(frame_index), fps(fps_value));
    (timing.timestamp().get(), timing.duration().get())
}

fn bitrate(width: u32, height: u32, fps_value: u32) -> u32 {
    recording_bitrate(dimensions(width, height), fps(fps_value)).get()
}

// -------------------------------------------------------------------------
// frame_index_for_elapsed — wall-clock anchored recording timeline
// -------------------------------------------------------------------------

#[test]
fn elapsed_frame_index_tracks_wall_clock_when_encoder_falls_behind() {
    assert_eq!(frame_index_for_elapsed(Duration::ZERO, fps(60)).get(), 0);
    assert_eq!(
        frame_index_for_elapsed(Duration::from_millis(16), fps(60)).get(),
        0
    );
    assert_eq!(
        frame_index_for_elapsed(Duration::from_millis(17), fps(60)).get(),
        1
    );
    assert_eq!(
        frame_index_for_elapsed(Duration::from_millis(500), fps(60)).get(),
        30
    );
    assert_eq!(
        frame_index_for_elapsed(Duration::from_secs(10), fps(30)).get(),
        300
    );
}

#[test]
fn elapsed_frame_index_saturates_for_extreme_elapsed_time() {
    assert_eq!(
        frame_index_for_elapsed(Duration::MAX, Fps::DEFAULT).get(),
        i64::MAX
    );
}

// -------------------------------------------------------------------------
// frame_timing_hns — exact rational frame clock
// -------------------------------------------------------------------------

#[test]
fn rational_frame_timing_has_no_one_second_drift_at_30fps() {
    let frames: Vec<_> = (0..30).map(|index| timing(index, 30)).collect();

    assert_eq!(frames[0], (0, 333_333));
    assert_eq!(frames[1], (333_333, 333_333));
    assert_eq!(frames[2], (666_666, 333_334));
    assert_eq!(
        frames.last().unwrap().0 + frames.last().unwrap().1,
        10_000_000
    );
}

#[test]
fn frame_timing_hns_non_divisor_rates_close_each_second_exactly() {
    // 24 and 144 do not divide 10^7 evenly: per-frame durations vary by one
    // tick, but the last frame of the first second must end exactly on the
    // one-second boundary.
    for fps in [24u32, 144] {
        let (start, duration) = timing(u64::from(fps - 1), fps);
        assert_eq!(start + duration, 10_000_000, "fps={fps}");
    }
}

#[test]
fn frame_timing_hns_large_index_at_240fps_is_exact() {
    // 10^7 frames at 240 fps: start = 10^7 * 10^7 / 240 with no accumulated
    // rounding error, and the frame still abuts its successor exactly.
    let (start, duration) = timing(10_000_000, 240);
    assert_eq!(start, 416_666_666_666);
    assert_eq!(start + duration, timing(10_000_001, 240).0);
}

// -------------------------------------------------------------------------
// recording_bitrate — scaling and clamp boundaries
// -------------------------------------------------------------------------

#[test]
fn recording_bitrate_scales_with_frame_size_and_rate() {
    let hd = bitrate(1280, 720, 30);
    let full_hd = bitrate(1920, 1080, 30);
    let high_refresh = bitrate(1920, 1080, 60);
    let ultra_hd = bitrate(3840, 2160, 30);

    assert!(hd < full_hd);
    assert!(full_hd < high_refresh);
    assert!(high_refresh < ultra_hd);
    assert!((7_000_000..=8_000_000).contains(&full_hd));
}

#[test]
fn recording_bitrate_unclamped_value_is_exact() {
    // 1920 * 1080 * 30 * 12 / 100 = 7_464_960 — inside the clamp range,
    // so the raw pixel-throughput value is returned unchanged.
    assert_eq!(bitrate(1920, 1080, 30), 7_464_960);
}

#[test]
fn recording_bitrate_clamps_small_input_to_floor() {
    // Raw value 100 * 100 * 30 * 12 / 100 = 36_000 is below the 2 Mbit floor.
    assert_eq!(bitrate(100, 100, 30), 2_000_000);
}

#[test]
fn recording_bitrate_smallest_valid_input_clamps_to_floor() {
    assert_eq!(bitrate(1, 1, 1), 2_000_000);
}

#[test]
fn recording_bitrate_clamps_huge_input_to_ceiling() {
    // Raw value 7680 * 4320 * 120 * 12 / 100 = 477_757_440 exceeds the
    // 50 Mbit ceiling.
    assert_eq!(bitrate(7680, 4320, 120), 50_000_000);
}

// -------------------------------------------------------------------------
// frame_duration_hns — legacy fixed frame duration
// -------------------------------------------------------------------------

#[test]
fn frame_duration_hns_matches_truncated_integer_division() {
    // (fps, expected 100ns ticks) — exact divisors and truncating rates alike.
    let cases: [(u32, i64); 14] = [
        (1, 10_000_000),
        (2, 5_000_000),
        (3, 3_333_333),
        (5, 2_000_000),
        (10, 1_000_000),
        (15, 666_666),
        (24, 416_666),
        (25, 400_000),
        (30, 333_333),
        (50, 200_000),
        (60, 166_666),
        (120, 83_333),
        (144, 69_444),
        (240, 41_666),
    ];
    for (fps, expected) in cases {
        assert_eq!(frame_duration_hns(fps), expected, "fps={fps}");
    }
}

#[test]
fn frame_duration_hns_0fps_returns_zero() {
    // Edge case: 0 fps should not panic, returns 0
    assert_eq!(frame_duration_hns(0), 0);
}

#[test]
fn frame_duration_hns_max_fps_no_panic() {
    // Should not panic even for very high fps
    let _ = frame_duration_hns(u32::MAX);
}

#[test]
fn frame_duration_hns_monotonically_decreasing() {
    let fps_values = [1, 10, 24, 30, 60, 120, 240];
    for pair in fps_values.windows(2) {
        assert!(
            frame_duration_hns(pair[0]) > frame_duration_hns(pair[1]),
            "Expected duration at {}fps > duration at {}fps",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn frame_duration_hns_always_positive_for_nonzero() {
    for fps in 1..=300 {
        assert!(
            frame_duration_hns(fps) > 0,
            "fps={fps} should yield positive duration"
        );
    }
}
