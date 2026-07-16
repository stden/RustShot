// ---------------------------------------------------------------------------
// record::frame_duration_hns edge cases
// ---------------------------------------------------------------------------

#[test]
fn frame_duration_hns_24fps_film() {
    assert_eq!(rustshot::record::frame_duration_hns(24), 416666);
}

#[test]
fn frame_duration_hns_120fps() {
    assert_eq!(rustshot::record::frame_duration_hns(120), 83333);
}

#[test]
fn frame_duration_hns_240fps() {
    assert_eq!(rustshot::record::frame_duration_hns(240), 41666);
}

#[test]
fn frame_duration_hns_is_positive_for_nonzero() {
    for fps in [1, 10, 24, 30, 60, 120, 240] {
        assert!(
            rustshot::record::frame_duration_hns(fps) > 0,
            "frame_duration_hns({fps}) should be positive"
        );
    }
}

#[test]
fn frame_duration_hns_decreases_with_higher_fps() {
    let d30 = rustshot::record::frame_duration_hns(30);
    let d60 = rustshot::record::frame_duration_hns(60);
    let d120 = rustshot::record::frame_duration_hns(120);
    assert!(d30 > d60);
    assert!(d60 > d120);
}

// ---------------------------------------------------------------------------
// record::round_to_even more values
// ---------------------------------------------------------------------------

#[test]
fn round_to_even_common_resolutions() {
    // All standard resolutions are already even
    for w in [640, 720, 1280, 1920, 2560, 3840] {
        assert_eq!(
            rustshot::record::round_to_even(w),
            w,
            "even value {w} should be unchanged"
        );
    }
}

#[test]
fn round_to_even_three_becomes_four() {
    assert_eq!(rustshot::record::round_to_even(3), 4);
}

#[test]
fn round_to_even_result_is_always_even() {
    for v in 0..=100 {
        let r = rustshot::record::round_to_even(v);
        assert_eq!(r % 2, 0, "round_to_even({v}) = {r} is not even");
    }
}

#[test]
fn round_to_even_result_gte_input() {
    for v in 0..=100 {
        assert!(
            rustshot::record::round_to_even(v) >= v,
            "round_to_even({v}) should be >= {v}"
        );
    }
}

#[test]
fn round_to_even_result_at_most_one_more() {
    for v in 0..=100 {
        let r = rustshot::record::round_to_even(v);
        assert!(r <= v + 1, "round_to_even({v}) = {r}, too large");
    }
}

// ---------------------------------------------------------------------------
// record::pack_u32_pair — roundtrip extraction
// ---------------------------------------------------------------------------

#[test]
fn pack_u32_pair_extract_hi_and_lo() {
    for (hi, lo) in [(0, 0), (1, 2), (1920, 1080), (u32::MAX, 0), (0, u32::MAX)] {
        let packed = rustshot::record::pack_u32_pair(hi, lo);
        assert_eq!((packed >> 32) as u32, hi, "hi mismatch for ({hi}, {lo})");
        assert_eq!(packed as u32, lo, "lo mismatch for ({hi}, {lo})");
    }
}

#[test]
fn pack_u32_pair_frame_rate_30_over_1() {
    let packed = rustshot::record::pack_u32_pair(30, 1);
    assert_eq!((packed >> 32) as u32, 30);
    assert_eq!(packed as u32, 1);
}

// ---------------------------------------------------------------------------
// record::validate_recording_start edge cases
// ---------------------------------------------------------------------------

#[test]
fn validate_recording_start_zero_error_mentions_greater_than_0() {
    let err = rustshot::record::validate_recording_start(0).unwrap_err();
    assert!(
        err.contains("greater than 0"),
        "error should mention 'greater than 0': {err}"
    );
}

#[cfg(windows)]
#[test]
fn validate_recording_start_1fps_ok() {
    assert!(rustshot::record::validate_recording_start(1).is_ok());
}

#[cfg(windows)]
#[test]
fn validate_recording_start_240fps_ok() {
    assert!(rustshot::record::validate_recording_start(240).is_ok());
}

#[test]
fn validate_recording_start_241fps_mentions_max_limit() {
    let err = rustshot::record::validate_recording_start(241).unwrap_err();
    assert!(
        err.contains("240"),
        "error should mention the max supported FPS: {err}"
    );
}

// ---------------------------------------------------------------------------
// frame_duration_hns — mathematical properties
// ---------------------------------------------------------------------------

#[test]
fn frame_duration_hns_times_fps_approximates_one_second() {
    // 10_000_000 HNS = 1 second. fps * frame_duration ≈ 10_000_000
    for fps in [1u32, 10, 24, 30, 60, 120] {
        let duration = rustshot::record::frame_duration_hns(fps);
        let product = duration * (fps as i64);
        // Due to integer division truncation, product may be slightly less
        let error = (10_000_000i64 - product).abs();
        assert!(
            error < fps as i64,
            "fps={fps}: {fps}*{duration}={product}, error={error} should be < {fps}"
        );
    }
}

#[test]
fn frame_duration_hns_10fps() {
    assert_eq!(rustshot::record::frame_duration_hns(10), 1_000_000);
}

#[test]
fn frame_duration_hns_25fps_pal() {
    assert_eq!(rustshot::record::frame_duration_hns(25), 400_000);
}

#[test]
fn frame_duration_hns_u32_max_no_panic() {
    // Should not panic or overflow
    let _ = rustshot::record::frame_duration_hns(u32::MAX);
}

// ---------------------------------------------------------------------------
// pack_u32_pair — WMF encoding patterns
// ---------------------------------------------------------------------------

#[test]
fn pack_u32_pair_aspect_ratio_1_1() {
    let packed = rustshot::record::pack_u32_pair(1, 1);
    assert_eq!((packed >> 32) as u32, 1);
    assert_eq!(packed as u32, 1);
}

#[test]
fn pack_u32_pair_common_frame_sizes() {
    for (w, h) in [
        (640, 480),
        (1280, 720),
        (1920, 1080),
        (2560, 1440),
        (3840, 2160),
    ] {
        let packed = rustshot::record::pack_u32_pair(w, h);
        assert_eq!((packed >> 32) as u32, w);
        assert_eq!(packed as u32, h);
    }
}

#[test]
fn pack_u32_pair_is_deterministic() {
    let a = rustshot::record::pack_u32_pair(1920, 1080);
    let b = rustshot::record::pack_u32_pair(1920, 1080);
    assert_eq!(a, b);
}

#[test]
fn pack_u32_pair_order_matters() {
    let ab = rustshot::record::pack_u32_pair(100, 200);
    let ba = rustshot::record::pack_u32_pair(200, 100);
    assert_ne!(ab, ba);
}
