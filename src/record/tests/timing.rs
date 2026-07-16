//! Tests for [`frame_duration_hns`] frame-timing math.

use crate::record::*;

// -------------------------------------------------------------------------
// frame_duration_hns tests (TC-FPS-001 through TC-FPS-004)
// -------------------------------------------------------------------------

#[test]
fn frame_duration_hns_30fps() {
    // 10_000_000 / 30 = 333333
    assert_eq!(frame_duration_hns(30), 333333);
}

#[test]
fn frame_duration_hns_60fps() {
    // 10_000_000 / 60 = 166666
    assert_eq!(frame_duration_hns(60), 166666);
}

#[test]
fn frame_duration_hns_1fps() {
    // 10_000_000 / 1 = 10_000_000 (one full second)
    assert_eq!(frame_duration_hns(1), 10_000_000);
}

#[test]
fn frame_duration_hns_0fps_returns_zero() {
    // Edge case: 0 fps should not panic, returns 0
    assert_eq!(frame_duration_hns(0), 0);
}

#[test]
fn frame_duration_hns_24fps() {
    // 10_000_000 / 24 = 416666
    assert_eq!(frame_duration_hns(24), 416666);
}

#[test]
fn frame_duration_hns_is_positive_for_any_nonzero_fps() {
    for fps in [1, 2, 10, 15, 24, 25, 30, 50, 60, 120, 144, 240] {
        assert!(
            frame_duration_hns(fps) > 0,
            "frame_duration_hns({fps}) should be positive"
        );
    }
}

#[test]
fn frame_duration_hns_decreases_with_higher_fps() {
    let d30 = frame_duration_hns(30);
    let d60 = frame_duration_hns(60);
    assert!(
        d30 > d60,
        "30fps duration ({d30}) should be greater than 60fps duration ({d60})"
    );
}

// --- frame_duration_hns additional tests ---

#[test]
fn frame_duration_hns_120fps() {
    assert_eq!(frame_duration_hns(120), 10_000_000 / 120);
}

#[test]
fn frame_duration_hns_1_is_one_second() {
    assert_eq!(frame_duration_hns(1), 10_000_000);
}

#[test]
fn frame_duration_hns_max_fps_no_panic() {
    // Should not panic even for very high fps
    let _ = frame_duration_hns(u32::MAX);
}

#[test]
fn frame_duration_hns_10fps() {
    assert_eq!(frame_duration_hns(10), 1_000_000);
}

// --- frame_duration_hns high fps tests ---

#[test]
fn frame_duration_hns_240fps() {
    assert_eq!(frame_duration_hns(240), 10_000_000 / 240);
}

#[test]
fn frame_duration_hns_144fps() {
    assert_eq!(frame_duration_hns(144), 10_000_000 / 144);
}

#[test]
fn frame_duration_hns_25fps_pal() {
    assert_eq!(frame_duration_hns(25), 400_000);
}

#[test]
fn frame_duration_hns_15fps() {
    assert_eq!(frame_duration_hns(15), 10_000_000 / 15);
}

#[test]
fn frame_duration_hns_2fps() {
    assert_eq!(frame_duration_hns(2), 5_000_000);
}

// =========================================================================
// frame_duration_hns — additional coverage
// =========================================================================

#[test]
fn frame_duration_hns_1_fps() {
    assert_eq!(frame_duration_hns(1), 10_000_000);
}

#[test]
fn frame_duration_hns_10_fps() {
    assert_eq!(frame_duration_hns(10), 1_000_000);
}

#[test]
fn frame_duration_hns_24_fps() {
    // 10_000_000 / 24 = 416666
    assert_eq!(frame_duration_hns(24), 416666);
}

#[test]
fn frame_duration_hns_25_fps() {
    assert_eq!(frame_duration_hns(25), 400000);
}

#[test]
fn frame_duration_hns_120_fps() {
    // 10_000_000 / 120 = 83333
    assert_eq!(frame_duration_hns(120), 83333);
}

// --- frame_duration_hns edge cases ---

#[test]
fn frame_duration_hns_50fps() {
    assert_eq!(frame_duration_hns(50), 200_000);
}

#[test]
fn frame_duration_hns_higher_fps_yields_smaller_duration() {
    assert!(frame_duration_hns(60) < frame_duration_hns(30));
    assert!(frame_duration_hns(120) < frame_duration_hns(60));
}

// -------------------------------------------------------------------------
// frame_duration_hns additional edge cases
// -------------------------------------------------------------------------

#[test]
fn frame_duration_hns_30fps_is_333333() {
    assert_eq!(frame_duration_hns(30), 333_333);
}

#[test]
fn frame_duration_hns_60fps_is_166666() {
    assert_eq!(frame_duration_hns(60), 166_666);
}

#[test]
fn frame_duration_hns_1fps_is_10_million() {
    assert_eq!(frame_duration_hns(1), 10_000_000);
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

// -------------------------------------------------------------------------
// frame_duration_hns edge cases
// -------------------------------------------------------------------------

#[test]
fn frame_duration_hns_zero_fps_returns_zero() {
    assert_eq!(frame_duration_hns(0), 0);
}

#[test]
fn frame_duration_hns_30_fps() {
    assert_eq!(frame_duration_hns(30), 333333);
}

#[test]
fn frame_duration_hns_60_fps() {
    assert_eq!(frame_duration_hns(60), 166666);
}

#[test]
fn frame_duration_hns_1_fps_equals_10_million() {
    assert_eq!(frame_duration_hns(1), 10_000_000);
}

#[test]
fn frame_duration_hns_is_positive_for_nonzero_fps() {
    for fps in [1, 10, 24, 30, 60, 120, 240] {
        assert!(
            frame_duration_hns(fps) > 0,
            "fps={fps} should give positive duration"
        );
    }
}

// -------------------------------------------------------------------------
// frame_duration_hns: boundary fps values
// -------------------------------------------------------------------------

#[test]
fn frame_duration_hns_3fps() {
    assert_eq!(frame_duration_hns(3), 3_333_333);
}

#[test]
fn frame_duration_hns_5fps() {
    assert_eq!(frame_duration_hns(5), 2_000_000);
}

// -------------------------------------------------------------------------
// frame_duration_hns — 2 fps
// -------------------------------------------------------------------------

#[test]
fn frame_duration_hns_2fps_is_5_million() {
    assert_eq!(frame_duration_hns(2), 5_000_000);
}
