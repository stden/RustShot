//! Tests for [`pack_u32_pair`] (Media Foundation u64 attribute packing).

use crate::record::*;

#[test]
fn pack_u32_pair_30fps_encodes_correctly() {
    // pack_u32_pair(30, 1) should encode 30 fps as ((30 << 32) | 1)
    let result = pack_u32_pair(30, 1);
    let expected: u64 = ((30u64) << 32) | 1u64;
    assert_eq!(
        result, expected,
        "pack_u32_pair(30, 1) should equal 0x{:016X}",
        expected
    );
}

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
fn pack_u32_pair_hi_and_lo_independent() {
    // hi=1, lo=1: only bit 32 and bit 0 set
    let result = pack_u32_pair(1, 1);
    assert_eq!(result, (1u64 << 32) | 1u64);
}

#[test]
fn pack_u32_pair_frame_size_1920x1080() {
    let result = pack_u32_pair(1920, 1080);
    let expected = ((1920u64) << 32) | 1080u64;
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
fn pack_u32_pair_hi_only_uses_upper_bits() {
    // If << were replaced with >>, hi=1 would give 0 instead of 1<<32
    let result = pack_u32_pair(1, 0);
    assert_eq!(result, 1u64 << 32);
    assert!(result > u32::MAX as u64, "hi must be in upper 32 bits");
}

#[test]
fn pack_u32_pair_lo_only_uses_lower_bits() {
    // Ensure lo doesn't bleed into upper bits
    let result = pack_u32_pair(0, 42);
    assert_eq!(result, 42u64);
}

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

#[test]
fn pack_u32_pair_4k_frame_size() {
    let packed = pack_u32_pair(3840, 2160);
    assert_eq!((packed >> 32) as u32, 3840);
    assert_eq!(packed as u32, 2160);
}
