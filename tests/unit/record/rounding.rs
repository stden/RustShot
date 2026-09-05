//! Tests for [`round_to_even`] (H.264 even-dimension requirement).
//!
//! `prop_round_to_even_correctness` in `props.rs` now specifies the function for
//! every `u32`, `u32::MAX` included — nothing overflows there, the add saturates.
//! The examples here are canonical anchors plus that edge as a named case:
//! uniform `any::<u32>()` sampling will realistically never draw `u32::MAX`, so
//! the proptest branch alone does not defend it.

use crate::record::*;

#[test]
fn round_to_even_zero_stays_zero() {
    assert_eq!(round_to_even(0), 0);
}

#[test]
fn round_to_even_u32_max_minus_1_stays() {
    // u32::MAX - 1 = 4294967294, which is even
    assert_eq!(round_to_even(u32::MAX - 1), u32::MAX - 1);
}

#[test]
fn round_to_even_u32_max_saturates_one_below_its_input() {
    // The only input this helper maps DOWNWARD: u32::MAX is odd, 2^32 does not
    // fit, so saturating_add(1) returns u32::MAX and clearing bit 0 gives
    // u32::MAX - 1. Pinned because the rustdoc used to promise an unconditional
    // round-up: a caller trusting "result >= value" would hand H.264 a frame one
    // row shorter than requested with no error anywhere. The checked form below
    // is what the pipeline uses precisely so this case cannot reach an encoder.
    assert_eq!(round_to_even(u32::MAX), u32::MAX - 1);
}

#[test]
fn checked_round_to_even_rejects_u32_max() {
    assert_eq!(
        checked_round_to_even(u32::MAX),
        Err("Recording dimension is too large to round to an even value".to_string())
    );
}
