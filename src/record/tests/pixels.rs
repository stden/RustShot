//! Tests for [`rgba_to_bgra_bottom_up`] pixel-format conversion.

use crate::record::*;

// -------------------------------------------------------------------------
// rgba_to_bgra_bottom_up tests (original 3 kept + new tests)
// -------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_swaps_rb_and_flips() {
    // 1x2 image: top row = [R=255,G=0,B=0,A=255], bottom row = [R=0,G=0,B=255,A=255]
    let rgba = vec![
        255, 0, 0, 255, // top row: red
        0, 0, 255, 255, // bottom row: blue
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 2);
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
fn rgba_to_bgra_bottom_up_single_black_pixel() {
    let rgba = vec![0u8, 0, 0, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(out, vec![0, 0, 0, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_output_length_matches_input() {
    let rgba = vec![1u8; 4 * 10 * 10];
    let out = rgba_to_bgra_bottom_up(&rgba, 10, 10);
    assert_eq!(out.len(), rgba.len());
}

#[test]
fn rgba_to_bgra_bottom_up_single_white_pixel() {
    // White pixel: R=255, G=255, B=255, A=255 — R/B swap is symmetric for white
    let rgba = vec![255u8, 255, 255, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(out, vec![255, 255, 255, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_green_pixel() {
    // Green pixel: R=0, G=255, B=0, A=255 — G channel stays at position 1
    // RGBA [0,255,0,255] → BGRA [B=0,G=255,R=0,A=255]
    let rgba = vec![0u8, 255, 0, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(
        out,
        vec![0, 255, 0, 255],
        "green pixel BGRA should be [0,255,0,255]"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_red_pixel_becomes_blue_channel() {
    // Pure red RGBA [255,0,0,255] → BGRA: out[0]=B=0, out[2]=R=255
    let rgba = vec![255u8, 0, 0, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(
        out[0], 0,
        "blue channel (index 0) should be 0 for pure red input"
    );
    assert_eq!(
        out[2], 255,
        "red channel (index 2) should be 255 for pure red input"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_alpha_preserved() {
    // Arbitrary alpha value — must survive conversion unchanged
    let rgba = vec![10u8, 20, 30, 128];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(
        out[3], 128,
        "alpha channel must be unchanged after conversion"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_1x1_arbitrary_color() {
    // RGBA [100, 150, 200, 128] → BGRA [200, 150, 100, 128]
    let rgba = vec![100u8, 150, 200, 128];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(
        out,
        vec![200, 150, 100, 128],
        "expected BGRA [200,150,100,128] for RGBA [100,150,200,128]"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_2x2_row_order_reversed() {
    // 2x2 image with two distinct rows
    // Row 0 (top):    [R=255, G=0,   B=0,   A=255] [R=255, G=0,   B=0,   A=255]  (red)
    // Row 1 (bottom): [R=0,   G=0,   B=255, A=255] [R=0,   G=0,   B=255, A=255]  (blue)
    let rgba = vec![
        255, 0, 0, 255, 255, 0, 0, 255, // row 0: red, red
        0, 0, 255, 255, 0, 0, 255, 255, // row 1: blue, blue
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 2, 2);
    // Bottom-up output: row 0 of output = row 1 of input (blue → BGRA [255,0,0,255])
    assert_eq!(
        &out[0..4],
        &[255, 0, 0, 255],
        "output row 0 pixel 0 should be blue→BGRA"
    );
    assert_eq!(
        &out[4..8],
        &[255, 0, 0, 255],
        "output row 0 pixel 1 should be blue→BGRA"
    );
    // Row 1 of output = row 0 of input (red → BGRA [0,0,255,255])
    assert_eq!(
        &out[8..12],
        &[0, 0, 255, 255],
        "output row 1 pixel 0 should be red→BGRA"
    );
    assert_eq!(
        &out[12..16],
        &[0, 0, 255, 255],
        "output row 1 pixel 1 should be red→BGRA"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_1x1_is_identity_for_bgra_symmetric() {
    // Black pixel [0,0,0,255]: after R/B swap it remains the same (both are 0)
    let rgba = vec![0u8, 0, 0, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(out, vec![0, 0, 0, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_output_is_all_zero_for_zero_input() {
    // All-zero input (transparent black) must produce all-zero output
    let rgba = vec![0u8; 4 * 5 * 5];
    let out = rgba_to_bgra_bottom_up(&rgba, 5, 5);
    assert!(
        out.iter().all(|&b| b == 0),
        "all-zero input should produce all-zero output"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_large_image_correct_length() {
    // 1920x1080 input — output must be exactly 1920*1080*4 bytes
    let width: u32 = 1920;
    let height: u32 = 1080;
    let rgba = vec![0u8; (width * height * 4) as usize];
    let out = rgba_to_bgra_bottom_up(&rgba, width, height);
    assert_eq!(
        out.len(),
        (width * height * 4) as usize,
        "output length must equal width*height*4 for 1920x1080"
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
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 3);

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
fn rgba_to_bgra_bottom_up_preserves_all_alpha_values() {
    // 2x1 image with different alpha values
    let rgba = vec![100, 150, 200, 0, 10, 20, 30, 128];
    let out = rgba_to_bgra_bottom_up(&rgba, 2, 1);
    assert_eq!(out[3], 0, "first pixel alpha should be 0");
    assert_eq!(out[7], 128, "second pixel alpha should be 128");
}

#[test]
fn rgba_to_bgra_bottom_up_single_row_not_flipped() {
    // 3x1 image: only one row, so bottom-up flip is identity
    let rgba = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 3, 1);
    // Red RGBA→BGRA: [0, 0, 255, 255]
    assert_eq!(&out[0..4], &[0, 0, 255, 255]);
    // Green stays same in BGRA
    assert_eq!(&out[4..8], &[0, 255, 0, 255]);
    // Blue RGBA→BGRA: [255, 0, 0, 255]
    assert_eq!(&out[8..12], &[255, 0, 0, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_double_conversion_restores_original_row_order() {
    // Applying bottom-up flip twice should restore original row order
    // (R/B swap applied twice also restores channels)
    let rgba = vec![10, 20, 30, 255, 40, 50, 60, 128];
    let first = rgba_to_bgra_bottom_up(&rgba, 1, 2);
    let second = rgba_to_bgra_bottom_up(&first, 1, 2);
    assert_eq!(second, rgba);
}

// --- rgba_to_bgra_bottom_up additional tests ---

#[test]
fn rgba_to_bgra_bottom_up_wide_single_row() {
    // 4x1 image: single row, no flip needed
    let rgba = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
        128, 128, 128, 255, // gray
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 4, 1);
    // Red → BGRA: [0, 0, 255, 255]
    assert_eq!(&out[0..4], &[0, 0, 255, 255]);
    // Green stays
    assert_eq!(&out[4..8], &[0, 255, 0, 255]);
    // Blue → BGRA: [255, 0, 0, 255]
    assert_eq!(&out[8..12], &[255, 0, 0, 255]);
    // Gray → symmetric
    assert_eq!(&out[12..16], &[128, 128, 128, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_4x4_first_and_last_row_swapped() {
    // 1x4 image
    let rgba = vec![
        10, 20, 30, 255, // row 0
        40, 50, 60, 255, // row 1
        70, 80, 90, 255, // row 2
        100, 110, 120, 255, // row 3
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 4);
    // Output row 0 = src row 3: RGBA[100,110,120,255] → BGRA[120,110,100,255]
    assert_eq!(&out[0..4], &[120, 110, 100, 255]);
    // Output row 3 = src row 0: RGBA[10,20,30,255] → BGRA[30,20,10,255]
    assert_eq!(&out[12..16], &[30, 20, 10, 255]);
}

// --- rgba_to_bgra_bottom_up asymmetric dimensions ---

#[test]
fn rgba_to_bgra_bottom_up_3x1_wide_image() {
    // 3 pixels wide, 1 pixel tall
    let rgba = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 3, 1);
    assert_eq!(out.len(), 12);
    // Red → BGRA [0, 0, 255, 255]
    assert_eq!(&out[0..4], &[0, 0, 255, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_1x3_tall_image_flips_rows() {
    let rgba = vec![
        10, 20, 30, 255, // row 0
        40, 50, 60, 255, // row 1
        70, 80, 90, 255, // row 2
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 3);
    // Output row 0 = src row 2: RGBA[70,80,90,255] → BGRA[90,80,70,255]
    assert_eq!(&out[0..4], &[90, 80, 70, 255]);
    // Output row 2 = src row 0: RGBA[10,20,30,255] → BGRA[30,20,10,255]
    assert_eq!(&out[8..12], &[30, 20, 10, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_asymmetric_5x2() {
    // 5 pixels wide, 2 rows tall
    let mut rgba = vec![0u8; 5 * 2 * 4];
    // Row 0: all red (255, 0, 0, 255)
    for x in 0..5 {
        rgba[x * 4] = 255;
        rgba[x * 4 + 3] = 255;
    }
    // Row 1: all blue (0, 0, 255, 255)
    for x in 0..5 {
        let offset = 5 * 4 + x * 4;
        rgba[offset + 2] = 255;
        rgba[offset + 3] = 255;
    }
    let out = rgba_to_bgra_bottom_up(&rgba, 5, 2);
    // Output row 0 = src row 1 (blue). Blue RGBA→BGRA: [255, 0, 0, 255]
    assert_eq!(&out[0..4], &[255, 0, 0, 255]);
    // Output row 1 = src row 0 (red). Red RGBA→BGRA: [0, 0, 255, 255]
    assert_eq!(&out[5 * 4..5 * 4 + 4], &[0, 0, 255, 255]);
}

// =========================================================================
// rgba_to_bgra_bottom_up — additional coverage
// =========================================================================

#[test]
fn rgba_to_bgra_bottom_up_2x2_correct_flip_and_swap() {
    // 2x2 image:
    // Row 0: (255, 0, 0, 255), (0, 255, 0, 255)
    // Row 1: (0, 0, 255, 255), (128, 128, 128, 255)
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, // row 0
        0, 0, 255, 255, 128, 128, 128, 255, // row 1
    ];
    let result = rgba_to_bgra_bottom_up(&rgba, 2, 2);
    // After bottom-up flip: row 1 becomes row 0, row 0 becomes row 1
    // After RGBA→BGRA swap (R↔B):
    // New row 0 (was row 1): (255, 0, 0, 255), (128, 128, 128, 255)
    // New row 1 (was row 0): (0, 0, 255, 255), (0, 255, 0, 255)
    assert_eq!(result[0], 255); // B channel of pixel (0,0,255) = 255
    assert_eq!(result[1], 0); // G
    assert_eq!(result[2], 0); // R→B swap: was R=0
    assert_eq!(result[3], 255); // A

    // Second pixel of new row 0: (128,128,128,255) → BGRA swap: still (128,128,128,255)
    assert_eq!(result[4], 128);
    assert_eq!(result[5], 128);
    assert_eq!(result[6], 128);
    assert_eq!(result[7], 255);
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_alpha() {
    let rgba = vec![100, 150, 200, 50]; // 1x1 with alpha=50
    let result = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(result[3], 50); // alpha preserved
}

#[test]
fn rgba_to_bgra_bottom_up_3x1_no_flip_only_swap() {
    // Single row: no vertical flip needed
    let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255];
    let result = rgba_to_bgra_bottom_up(&rgba, 3, 1);
    // Pixel 0: R=10, G=20, B=30 → BGRA: B=30, G=20, R=10
    assert_eq!(result[0], 30);
    assert_eq!(result[1], 20);
    assert_eq!(result[2], 10);
}

// --- rgba_to_bgra_bottom_up edge cases ---

#[test]
fn rgba_to_bgra_bottom_up_1x1_pure_red() {
    let rgba = [255, 0, 0, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    // BGRA: blue=0, green=0, red=255, alpha=255 but swapped R and B
    assert_eq!(out, [0, 0, 255, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_1x1_pure_blue() {
    let rgba = [0, 0, 255, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(out, [255, 0, 0, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_green_channel() {
    let rgba = [0, 128, 0, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(out[1], 128);
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_alpha_channel() {
    let rgba = [10, 20, 30, 200];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    assert_eq!(out[3], 200);
}

// -------------------------------------------------------------------------
// rgba_to_bgra_bottom_up additional edge cases
// -------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_empty_input() {
    let data: Vec<u8> = vec![];
    let out = rgba_to_bgra_bottom_up(&data, 0, 0);
    assert!(out.is_empty());
}

#[test]
fn rgba_to_bgra_bottom_up_pure_white_pixel() {
    let rgba = [255, 255, 255, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    // White: R=255,G=255,B=255 → B=255,G=255,R=255 — same
    assert_eq!(out, vec![255, 255, 255, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_pure_red_becomes_blue_channel() {
    let rgba = [255, 0, 0, 255];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
    // BGRA: B=255, G=0, R=0, A=255
    assert_eq!(out[0], 0); // B (was R=255 → now in position 2)
    assert_eq!(out[2], 255); // R (was B=0 → now has old R)
}
