#[test]
fn frame_duration_hns_times_fps_approximates_one_second() {
    for fps in [1_u32, 10, 24, 30, 60, 120, 240] {
        let duration = rustshot::record::frame_duration_hns(fps);
        let product = duration * i64::from(fps);
        let error = (10_000_000_i64 - product).abs();
        assert!(
            error < i64::from(fps),
            "fps={fps}: {fps}*{duration}={product}, error={error}"
        );
    }
}

#[test]
fn frame_duration_hns_u32_max_does_not_panic() {
    let _ = rustshot::record::frame_duration_hns(u32::MAX);
}

#[test]
fn round_to_even_result_is_always_even() {
    for value in 0..=100 {
        assert_eq!(rustshot::record::round_to_even(value) % 2, 0);
    }
}

#[test]
fn round_to_even_result_is_not_below_input_except_at_u32_max() {
    for value in 0..=100 {
        assert!(rustshot::record::round_to_even(value) >= value);
    }
    // The name carries the exception because the invariant genuinely has one, and
    // a 0..=100 loop under an unqualified name is what made it look universal.
    // At u32::MAX the even value needed is 2^32, so the saturating add returns the
    // input and clearing bit 0 lands one below it. Anyone who reads this test as
    // an unconditional ">= input" promise would feel safe dropping the
    // checked_round_to_even call the recording pipeline depends on.
    assert_eq!(
        rustshot::record::round_to_even(u32::MAX),
        u32::MAX - 1,
        "u32::MAX is the documented exception to the >= input rule"
    );
}

#[test]
fn round_to_even_result_is_at_most_one_larger() {
    for value in 0..=100 {
        assert!(rustshot::record::round_to_even(value) <= value + 1);
    }
}

#[test]
fn pack_u32_pair_roundtrips_both_values() {
    for (high, low) in [
        (0, 0),
        (1, 2),
        (1920, 1080),
        (u32::MAX, 0),
        (0, u32::MAX),
        (u32::MAX, u32::MAX),
    ] {
        let packed = rustshot::record::pack_u32_pair(high, low);
        assert_eq!((packed >> 32) as u32, high);
        assert_eq!(packed as u32, low);
    }
}

#[test]
fn validate_recording_start_zero_reports_lower_bound() {
    let error = rustshot::record::validate_recording_start(0).unwrap_err();
    assert!(error.contains("greater than 0"));
}

#[test]
fn validate_recording_start_above_maximum_reports_limit() {
    let error = rustshot::record::validate_recording_start(241).unwrap_err();
    assert!(error.contains("240"));
}
