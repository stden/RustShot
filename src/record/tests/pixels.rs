//! Tests for [`rgba_to_bgra_bottom_up`] pixel-format conversion.
//!
//! One canonical test per behavior; the involution property (double
//! application restores the input) is covered for arbitrary buffers by
//! `prop_rgba_to_bgra_bottom_up_roundtrip` in `props.rs`.

use crate::record::*;

#[test]
fn rgba_to_bgra_bottom_up_swaps_rb_and_flips() {
    // 1x2 image: top row = [R=255,G=0,B=0,A=255], bottom row = [R=0,G=0,B=255,A=255]
    let rgba = vec![
        255, 0, 0, 255, // top row: red
        0, 0, 255, 255, // bottom row: blue
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 2).unwrap();
    // Bottom-up: output row 0 = src row 1 (blue), output row 1 = src row 0 (red)
    // Blue pixel in BGRA = [B=255, G=0, R=0, A=255]
    assert_eq!(
        &out[0..4],
        &[255, 0, 0, 255],
        "bottom row (blue) should be first in output"
    );
    // Red pixel in BGRA = [B=0, G=0, R=255, A=255]
    assert_eq!(
        &out[4..8],
        &[0, 0, 255, 255],
        "top row (red) should be second in output"
    );
}

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
fn rgba_to_bgra_bottom_up_output_is_all_zero_for_zero_input() {
    // All-zero input (transparent black) must produce all-zero output
    let rgba = vec![0u8; 4 * 5 * 5];
    let out = rgba_to_bgra_bottom_up(&rgba, 5, 5).unwrap();
    assert!(
        out.iter().all(|&b| b == 0),
        "all-zero input should produce all-zero output"
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
