//! Tests for [`pack_u32_pair`] (Media Foundation u64 attribute packing).

use crate::record::*;

#[test]
fn pack_u32_pair_zero_both() {
    assert_eq!(pack_u32_pair(0, 0), 0u64);
}

#[test]
fn pack_u32_pair_max_hi() {
    let result = pack_u32_pair(u32::MAX, 0);
    let expected: u64 = (u32::MAX as u64) << 32;
    assert_eq!(result, expected);
}

#[test]
fn pack_u32_pair_max_lo() {
    let result = pack_u32_pair(0, u32::MAX);
    let expected: u64 = u32::MAX as u64;
    assert_eq!(result, expected);
}

#[test]
fn pack_u32_pair_is_reversible() {
    let packed = pack_u32_pair(1234, 5678);
    let hi = (packed >> 32) as u32;
    let lo = packed as u32;
    assert_eq!(hi, 1234);
    assert_eq!(lo, 5678);
}

// -------------------------------------------------------------------------
// Mutation-killing tests
// -------------------------------------------------------------------------

#[test]
fn pack_u32_pair_or_not_and() {
    // If | were replaced with &, pack(1, 2) would be 0 instead of (1<<32)|2
    let result = pack_u32_pair(1, 2);
    assert_ne!(result, 0, "| must not be replaced with &");
    assert_eq!(result, (1u64 << 32) | 2u64);
}

#[test]
fn pack_u32_pair_both_max() {
    let result = pack_u32_pair(u32::MAX, u32::MAX);
    assert_eq!(result, u64::MAX);
}
