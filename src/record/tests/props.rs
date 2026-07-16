//! Property-based tests (proptest) for the record module helpers.

use crate::record::*;

// -------------------------------------------------------------------------
// Property-based tests
// -------------------------------------------------------------------------

use proptest::prelude::*;

proptest! {
    /// **Validates: Requirements 17.3, 7.8, 7.9, 7.11**
    ///
    /// For all even u32 values, round_to_even returns the value unchanged.
    /// For all odd u32 values (excluding u32::MAX which overflows), round_to_even
    /// returns value + 1.
    #[test]
    fn prop_round_to_even_correctness(value in any::<u32>()) {
        if value % 2 == 0 {
            // Even values are returned unchanged
            prop_assert_eq!(
                round_to_even(value), value,
                "round_to_even({}) should return {} unchanged (even input)",
                value, value
            );
        } else if value < u32::MAX {
            // Odd values (excluding u32::MAX to avoid overflow) round up by 1
            prop_assert_eq!(
                round_to_even(value), value + 1,
                "round_to_even({}) should return {} (odd input rounds up)",
                value, value + 1
            );
        }
        // u32::MAX is odd and would overflow on +1, so we skip it
    }

    /// **Validates: Requirements 17.4, 9.3**
    ///
    /// For all valid RGBA buffers of dimensions (w, h), applying
    /// `rgba_to_bgra_bottom_up` twice produces the original buffer.
    #[test]
    fn prop_rgba_to_bgra_bottom_up_roundtrip(
        w in 1u32..=16,
        h in 1u32..=16,
        pixel_data in proptest::collection::vec(any::<u8>(), 0..=1024),
    ) {
        let buf_len = (w as usize) * (h as usize) * 4;
        // Build a buffer of exactly w*h*4 bytes, padding or truncating as needed
        let mut buf = pixel_data;
        buf.resize(buf_len, 0u8);

        let once = rgba_to_bgra_bottom_up(&buf, w, h);
        let twice = rgba_to_bgra_bottom_up(&once, w, h);

        prop_assert_eq!(
            twice, buf,
            "applying rgba_to_bgra_bottom_up twice should produce the original buffer \
             (w={}, h={}, buf_len={})",
            w, h, buf_len
        );
    }
}
