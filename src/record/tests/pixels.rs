//! Tests for [`rgba_to_bgra_bottom_up`] pixel-format conversion, in both its
//! two-buffer and single-buffer (in-place) forms.
//!
//! One canonical test per behavior; the involution property (double
//! application restores the input) is covered for arbitrary buffers by
//! `prop_rgba_to_bgra_bottom_up_roundtrip` in `props.rs`.

use crate::record::*;

#[test]
fn rgba_to_bgra_bottom_up_output_length_matches_input() {
    let rgba = vec![1u8; 4 * 10 * 10];
    let out = rgba_to_bgra_bottom_up(&rgba, 10, 10).unwrap();
    assert_eq!(out.len(), rgba.len());
}

#[test]
fn rgba_to_bgra_bottom_up_1x1_arbitrary_color() {
    // Canonical single-pixel channel permutation: all four channels distinct.
    // RGBA [100, 150, 200, 128] → BGRA [200, 150, 100, 128]
    let rgba = vec![100u8, 150, 200, 128];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1).unwrap();
    assert_eq!(
        out,
        vec![200, 150, 100, 128],
        "expected BGRA [200,150,100,128] for RGBA [100,150,200,128]"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_3_row_image_middle_row_is_in_middle_output() {
    // 1x3 image: rows 0,1,2 — after bottom-up flip: output row 0 = src row 2, output row 2 = src row 0
    // Middle row (src row 1) must appear as output row 1 (middle stays middle in odd-height image)
    let red = [255u8, 0, 0, 255]; // row 0
    let green = [0u8, 255, 0, 255]; // row 1 (middle)
    let blue = [0u8, 0, 255, 255]; // row 2
    let rgba: Vec<u8> = [red, green, blue].concat();
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 3).unwrap();

    // output row 0 = src row 2 (blue) → BGRA [255, 0, 0, 255]
    assert_eq!(
        &out[0..4],
        &[255, 0, 0, 255],
        "output row 0 should be blue (src row 2)"
    );
    // output row 1 = src row 1 (green) → BGRA [0, 255, 0, 255]
    assert_eq!(
        &out[4..8],
        &[0, 255, 0, 255],
        "output row 1 should be green (src row 1, middle)"
    );
    // output row 2 = src row 0 (red) → BGRA [0, 0, 255, 255]
    assert_eq!(
        &out[8..12],
        &[0, 0, 255, 255],
        "output row 2 should be red (src row 0)"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_single_row_not_flipped() {
    // 3x1 image: only one row, so bottom-up flip is identity
    let rgba = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 3, 1).unwrap();
    // Red RGBA→BGRA: [0, 0, 255, 255]
    assert_eq!(&out[0..4], &[0, 0, 255, 255]);
    // Green stays same in BGRA
    assert_eq!(&out[4..8], &[0, 255, 0, 255]);
    // Blue RGBA→BGRA: [255, 0, 0, 255]
    assert_eq!(&out[8..12], &[255, 0, 0, 255]);
}

// -------------------------------------------------------------------------
// rgba_to_bgra_bottom_up_in_place — the single-buffer form
// -------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_in_place_flips_row_pairs_and_swizzles_them() {
    // 2x2 with bytes 1..=16, so every channel of every pixel is distinguishable.
    // Same input and same expectation as the two-buffer 2x2 test in
    // `pixels_extra.rs` and the `pixel_conversion.feature` scenario: an even
    // height is nothing but row pairs.
    let mut pixels: Vec<u8> = (1..=16).collect();
    rgba_to_bgra_bottom_up_in_place(&mut pixels, 2, 2).unwrap();
    assert_eq!(
        pixels,
        vec![11, 10, 9, 12, 15, 14, 13, 16, 3, 2, 1, 4, 7, 6, 5, 8]
    );
}

#[test]
fn rgba_to_bgra_bottom_up_in_place_swizzles_an_odd_middle_row_once() {
    // 2x3 with bytes 1..=24. The middle row is the one row with no partner to
    // swap with, so it is what an in-place flip gets wrong: skip it and it keeps
    // its original channel order, swizzle it twice and it keeps that order too,
    // because the swizzle is its own inverse. Either mistake leaves one scanline
    // in the middle of an otherwise perfect screenshot with red and blue crossed,
    // which no check on buffer length or corner pixels would catch. The channel
    // values are deliberately asymmetric — a grey or pure-green row looks
    // identical swizzled and unswizzled and would prove nothing.
    let mut pixels: Vec<u8> = (1..=24).collect();
    rgba_to_bgra_bottom_up_in_place(&mut pixels, 2, 3).unwrap();

    assert_eq!(
        &pixels[0..8],
        &[19, 18, 17, 20, 23, 22, 21, 24],
        "the top row must be the source bottom row, swizzled"
    );
    assert_eq!(
        &pixels[8..16],
        &[11, 10, 9, 12, 15, 14, 13, 16],
        "the middle row stays in the middle and is swizzled exactly once"
    );
    assert_eq!(
        &pixels[16..24],
        &[3, 2, 1, 4, 7, 6, 5, 8],
        "the bottom row must be the source top row, swizzled"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_in_place_single_row_keeps_pixel_order() {
    // 3x1: `height / 2` is zero, so no pair is formed and the only row is the
    // unpaired one. Pixels must keep their left-to-right order — a flip that
    // reversed bytes instead of whole rows would pass a 1-pixel-wide test and
    // fail here.
    let mut pixels: Vec<u8> = (1..=12).collect();
    rgba_to_bgra_bottom_up_in_place(&mut pixels, 3, 1).unwrap();
    assert_eq!(pixels, vec![3, 2, 1, 4, 7, 6, 5, 8, 11, 10, 9, 12]);
}

#[test]
fn rgba_to_bgra_bottom_up_in_place_matches_the_two_buffer_conversion() {
    // The in-place form is only a safe substitute for the two-buffer one if it is
    // byte-identical, so compare them over every shape that reaches a different
    // branch: even and odd heights, a single row, the zero-sized frames the
    // two-buffer form already accepts, and a 7-pixel row — 28 bytes, which lands
    // in the swizzle's sub-32-byte tail instead of on an AVX2 block boundary.
    //
    // The widths above 7 are what make a row cross a vector boundary at all, and
    // they were added because their absence hid a live blind spot: with only
    // 0/1/2/7 every row was pure tail (row_bytes <= 28 < 32), so a mutant that ran
    // the AVX2 scalar tail *only* when the block loop had not run passed this test
    // and the entire suite. 8 pixels is exactly one 32-byte block with no tail, 9
    // is a block plus a tail, 16 is two clean blocks and 17 two blocks plus a
    // tail. Keep the narrow widths as well: they are the only ones here that catch
    // the tail being dropped outright.
    for height in 0..=6u32 {
        for width in [0u32, 1, 2, 7, 8, 9, 16, 17] {
            let source: Vec<u8> = (0..width as usize * height as usize * 4)
                .map(|index| (index % 251) as u8)
                .collect();
            let mut expected = vec![0u8; source.len()];
            rgba_to_bgra_bottom_up_into(&source, &mut expected, width, height).unwrap();

            let mut in_place = source.clone();
            rgba_to_bgra_bottom_up_in_place(&mut in_place, width, height).unwrap();

            assert_eq!(
                in_place, expected,
                "the in-place form differs from the two-buffer one at {width}x{height}"
            );
        }
    }
}
