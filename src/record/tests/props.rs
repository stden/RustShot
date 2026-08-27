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
    /// For all odd u32 values below u32::MAX it returns value + 1. u32::MAX gets
    /// its own branch rather than being skipped: nothing overflows there (the add
    /// saturates), but the even value it needs is 2^32, so the result is one
    /// BELOW the input. Skipping it is how the rustdoc's unconditional "rounds
    /// odd values up" claim survived unchallenged.
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
            // Odd values below the limit round up by 1
            prop_assert_eq!(
                round_to_even(value), value + 1,
                "round_to_even({}) should return {} (odd input rounds up)",
                value, value + 1
            );
        } else {
            // u32::MAX: saturating_add(1) gives u32::MAX back, then clearing bit 0
            // yields u32::MAX - 1. Asserted instead of skipped so the property is
            // total over u32 and no reader infers a >= input rule from a gap.
            prop_assert_eq!(
                round_to_even(value), u32::MAX - 1,
                "round_to_even(u32::MAX) saturates to {} rather than rounding up",
                u32::MAX - 1
            );
        }
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

        let once = rgba_to_bgra_bottom_up(&buf, w, h).unwrap();
        let twice = rgba_to_bgra_bottom_up(&once, w, h).unwrap();

        prop_assert_eq!(
            twice, buf,
            "applying rgba_to_bgra_bottom_up twice should produce the original buffer \
             (w={}, h={}, buf_len={})",
            w, h, buf_len
        );
    }

    /// The production frame clock (`frame_timing_hns`) must be exact for any
    /// frame rate and frame index: timestamps are non-negative, durations are
    /// positive, and each frame ends exactly where the next one starts — no
    /// cumulative drift even after 10 million frames.
    #[test]
    fn prop_frame_timing_hns_is_exact_and_gapless(
        fps in 1u32..=240,
        index in 0u64..=10_000_000,
    ) {
        let fps = Fps::new(fps).unwrap();
        let timing = frame_timing_hns(FrameIndex::new(index), fps);
        let next_timing = frame_timing_hns(FrameIndex::new(index + 1), fps);
        let start = timing.timestamp().get();
        let duration = timing.duration().get();
        let next_start = next_timing.timestamp().get();

        prop_assert!(
            start >= 0,
            "start must be non-negative (fps={}, index={}, start={})",
            fps, index, start
        );
        prop_assert!(
            duration > 0,
            "duration must be positive (fps={}, index={}, duration={})",
            fps, index, duration
        );
        prop_assert_eq!(
            start + duration, next_start,
            "frame {} must end exactly where frame {} starts (fps={})",
            index, index + 1, fps
        );
    }
}
