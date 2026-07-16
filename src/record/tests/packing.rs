//! Tests for [`pack_u32_pair`] (Media Foundation u64 attribute packing).

use crate::record::*;

// -------------------------------------------------------------------------
// pack_u32_pair tests
// -------------------------------------------------------------------------

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
// Additional pack_u32_pair mutation-killing tests
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
fn pack_u32_pair_or_not_xor() {
    // For non-overlapping bits, ^ gives same result as |, but test with overlapping
    // hi=0, lo=0 -> result should be 0 (both & and ^ give 0 too, so test non-zero)
    let result = pack_u32_pair(0xFF, 0xFF);
    assert_eq!(result, (0xFFu64 << 32) | 0xFFu64);
}

#[test]
fn pack_u32_pair_both_max() {
    let result = pack_u32_pair(u32::MAX, u32::MAX);
    assert_eq!(result, u64::MAX);
}

#[test]
fn pack_u32_pair_extracts_hi_correctly() {
    let packed = pack_u32_pair(0xDEAD, 0xBEEF);
    assert_eq!((packed >> 32) as u32, 0xDEAD);
}

#[test]
fn pack_u32_pair_extracts_lo_correctly() {
    let packed = pack_u32_pair(0xDEAD, 0xBEEF);
    assert_eq!(packed as u32, 0xBEEF);
}

// =========================================================================
// pack_u32_pair — additional coverage
// =========================================================================

#[test]
fn pack_u32_pair_both_zero() {
    assert_eq!(pack_u32_pair(0, 0), 0u64);
}

#[test]
fn pack_u32_pair_hi_only() {
    assert_eq!(pack_u32_pair(1, 0), 1u64 << 32);
}

#[test]
fn pack_u32_pair_lo_only() {
    assert_eq!(pack_u32_pair(0, 42), 42u64);
}

#[test]
fn pack_u32_pair_max_hi_zero_lo() {
    assert_eq!(pack_u32_pair(u32::MAX, 0), (u32::MAX as u64) << 32);
}

#[test]
fn pack_u32_pair_4k_frame_size() {
    let packed = pack_u32_pair(3840, 2160);
    assert_eq!((packed >> 32) as u32, 3840);
    assert_eq!(packed as u32, 2160);
}

#[test]
fn pack_u32_pair_roundtrip_arbitrary() {
    let packed = pack_u32_pair(12345, 67890);
    assert_eq!((packed >> 32) as u32, 12345);
    assert_eq!(packed as u32, 67890);
}

// -------------------------------------------------------------------------
// pack_u32_pair additional tests
// -------------------------------------------------------------------------

#[test]
fn pack_u32_pair_roundtrip_1920_1080() {
    let packed = pack_u32_pair(1920, 1080);
    let hi = (packed >> 32) as u32;
    let lo = packed as u32;
    assert_eq!(hi, 1920);
    assert_eq!(lo, 1080);
}

#[test]
fn pack_u32_pair_roundtrip_zero_max() {
    let packed = pack_u32_pair(0, u32::MAX);
    let hi = (packed >> 32) as u32;
    let lo = packed as u32;
    assert_eq!(hi, 0);
    assert_eq!(lo, u32::MAX);
}

#[test]
fn pack_u32_pair_roundtrip_max_zero() {
    let packed = pack_u32_pair(u32::MAX, 0);
    let hi = (packed >> 32) as u32;
    let lo = packed as u32;
    assert_eq!(hi, u32::MAX);
    assert_eq!(lo, 0);
}

// -------------------------------------------------------------------------
// pack_u32_pair tests
// -------------------------------------------------------------------------

#[test]
fn pack_u32_pair_zero_zero() {
    assert_eq!(pack_u32_pair(0, 0), 0u64);
}

#[test]
fn pack_u32_pair_one_zero() {
    // hi=1 in upper 32 bits: 1 << 32 = 4294967296
    assert_eq!(pack_u32_pair(1, 0), 1u64 << 32);
}

#[test]
fn pack_u32_pair_zero_one() {
    assert_eq!(pack_u32_pair(0, 1), 1u64);
}

#[test]
fn pack_u32_pair_typical_frame_size() {
    // 1920x1080
    let packed = pack_u32_pair(1920, 1080);
    let hi = (packed >> 32) as u32;
    let lo = packed as u32;
    assert_eq!(hi, 1920);
    assert_eq!(lo, 1080);
}

#[test]
fn pack_u32_pair_max_values() {
    let packed = pack_u32_pair(u32::MAX, u32::MAX);
    assert_eq!(packed, u64::MAX);
}

// -------------------------------------------------------------------------
// pack_u32_pair: asymmetric values
// -------------------------------------------------------------------------

#[test]
fn pack_u32_pair_asymmetric_3840x2160() {
    let packed = pack_u32_pair(3840, 2160);
    assert_eq!((packed >> 32) as u32, 3840);
    assert_eq!(packed as u32, 2160);
}
