//! Tests for [`round_to_even`] (H.264 even-dimension requirement).
//!
//! `prop_round_to_even_correctness` in `props.rs` specifies the function for
//! every `u32` except `u32::MAX`; the examples here are canonical anchors
//! plus the overflow edge the proptest deliberately skips.

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
fn checked_round_to_even_rejects_u32_max() {
    assert_eq!(
        checked_round_to_even(u32::MAX),
        Err("Recording dimension is too large to round to an even value".to_string())
    );
}
