//! Tests for [`round_to_even`] (H.264 even-dimension requirement).

use crate::record::*;

// -------------------------------------------------------------------------
// round_to_even tests (TC-H264-001 through TC-H264-003)
// -------------------------------------------------------------------------

#[test]
fn round_to_even_even_stays_even() {
    assert_eq!(round_to_even(1920), 1920);
    assert_eq!(round_to_even(1080), 1080);
    assert_eq!(round_to_even(100), 100);
}

#[test]
fn round_to_even_odd_rounds_up() {
    assert_eq!(round_to_even(1921), 1922);
    assert_eq!(round_to_even(1079), 1080);
    assert_eq!(round_to_even(101), 102);
}

#[test]
fn round_to_even_zero_stays_zero() {
    assert_eq!(round_to_even(0), 0);
}

#[test]
fn round_to_even_one_becomes_two() {
    assert_eq!(round_to_even(1), 2);
}

#[test]
fn round_to_even_two_stays_two() {
    assert_eq!(round_to_even(2), 2);
}

#[test]
fn round_to_even_result_is_always_even() {
    for v in 0..200 {
        let result = round_to_even(v);
        assert_eq!(
            result % 2,
            0,
            "round_to_even({v}) = {result}, which is not even"
        );
    }
}

#[test]
fn round_to_even_result_is_gte_input() {
    for v in 0..200 {
        assert!(round_to_even(v) >= v, "round_to_even({v}) should be >= {v}");
    }
}

#[test]
fn round_to_even_result_differs_by_at_most_one() {
    for v in 0..200 {
        let diff = round_to_even(v) - v;
        assert!(
            diff <= 1,
            "round_to_even({v}) differs from input by {diff}, expected at most 1"
        );
    }
}

// --- round_to_even additional tests ---

#[test]
fn round_to_even_large_odd() {
    assert_eq!(round_to_even(99999), 100000);
}

#[test]
fn round_to_even_large_even() {
    assert_eq!(round_to_even(100000), 100000);
}

#[test]
fn round_to_even_3_becomes_4() {
    assert_eq!(round_to_even(3), 4);
}

// --- round_to_even edge cases ---

#[test]
fn round_to_even_u32_max_minus_1_stays() {
    // u32::MAX - 1 = 4294967294, which is even
    assert_eq!(round_to_even(u32::MAX - 1), u32::MAX - 1);
}

#[test]
fn round_to_even_4_stays_4() {
    assert_eq!(round_to_even(4), 4);
}

#[test]
fn round_to_even_5_becomes_6() {
    assert_eq!(round_to_even(5), 6);
}

#[test]
fn round_to_even_common_resolutions() {
    assert_eq!(round_to_even(1920), 1920);
    assert_eq!(round_to_even(1080), 1080);
    assert_eq!(round_to_even(2560), 2560);
    assert_eq!(round_to_even(1440), 1440);
    assert_eq!(round_to_even(3840), 3840);
    assert_eq!(round_to_even(2160), 2160);
}

// =========================================================================
// round_to_even — additional coverage
// =========================================================================

#[test]
fn round_to_even_2_stays_2() {
    assert_eq!(round_to_even(2), 2);
}

#[test]
fn round_to_even_1080_is_unchanged() {
    assert_eq!(round_to_even(1080), 1080);
}

#[test]
fn round_to_even_1079_rounds_up_to_1080() {
    assert_eq!(round_to_even(1079), 1080);
}

#[test]
#[should_panic(expected = "overflow")]
fn round_to_even_u32_max_panics_in_debug() {
    round_to_even(u32::MAX);
}

#[test]
fn round_to_even_u32_max_minus_1_unchanged() {
    assert_eq!(round_to_even(u32::MAX - 1), u32::MAX - 1);
}

// --- round_to_even edge cases ---

#[test]
fn round_to_even_7_becomes_8() {
    assert_eq!(round_to_even(7), 8);
}

#[test]
fn round_to_even_10_stays_10() {
    assert_eq!(round_to_even(10), 10);
}

// -------------------------------------------------------------------------
// round_to_even additional edge cases
// -------------------------------------------------------------------------

#[test]
fn round_to_even_all_common_resolutions_unchanged() {
    let even_values = [720, 1080, 1440, 2160, 1920, 2560, 3840];
    for v in even_values {
        assert_eq!(round_to_even(v), v, "{v} should be unchanged");
    }
}

#[test]
fn round_to_even_odd_resolutions_round_up() {
    assert_eq!(round_to_even(719), 720);
    assert_eq!(round_to_even(1079), 1080);
    assert_eq!(round_to_even(1439), 1440);
    assert_eq!(round_to_even(2159), 2160);
}

#[test]
fn round_to_even_100_stays_100() {
    assert_eq!(round_to_even(100), 100);
}

#[test]
fn round_to_even_101_becomes_102() {
    assert_eq!(round_to_even(101), 102);
}

// -------------------------------------------------------------------------
// round_to_even tests
// -------------------------------------------------------------------------

#[test]
fn round_to_even_zero() {
    assert_eq!(round_to_even(0), 0);
}

#[test]
fn round_to_even_1_rounds_up_to_2() {
    assert_eq!(round_to_even(1), 2);
}

#[test]
fn round_to_even_2_unchanged() {
    assert_eq!(round_to_even(2), 2);
}

#[test]
fn round_to_even_1079_rounds_to_1080() {
    assert_eq!(round_to_even(1079), 1080);
}

#[test]
fn round_to_even_1920_stays_even() {
    assert_eq!(round_to_even(1920), 1920);
}

#[test]
fn round_to_even_4095_rounds_to_4096() {
    assert_eq!(round_to_even(4095), 4096);
}

// -------------------------------------------------------------------------
// round_to_even: additional edge cases
// -------------------------------------------------------------------------

#[test]
fn round_to_even_999_becomes_1000() {
    assert_eq!(round_to_even(999), 1000);
}

#[test]
fn round_to_even_1000_stays_1000() {
    assert_eq!(round_to_even(1000), 1000);
}

// -------------------------------------------------------------------------
// round_to_even — additional values
// -------------------------------------------------------------------------

#[test]
fn round_to_even_three_becomes_four() {
    assert_eq!(round_to_even(3), 4);
}

#[test]
fn round_to_even_four_stays_four() {
    assert_eq!(round_to_even(4), 4);
}

#[test]
fn round_to_even_large_even_value_unchanged() {
    // 4294967294 is even
    assert_eq!(round_to_even(4_294_967_294), 4_294_967_294);
}
