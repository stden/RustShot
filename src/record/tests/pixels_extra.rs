//! Additional edge-case tests for [`rgba_to_bgra_bottom_up`].

use crate::record::*;

// -------------------------------------------------------------------------
// rgba_to_bgra_bottom_up: 1x4 tall image row reversal
// -------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_1x4_rows_fully_reversed() {
    // 1x4 image: 4 rows of distinct colors (1 pixel each)
    let rgba = vec![
        10, 20, 30, 255, // row 0
        40, 50, 60, 255, // row 1
        70, 80, 90, 255, // row 2
        100, 110, 120, 255, // row 3
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 1, 4).unwrap();
    // Row 3 becomes row 0 (bottom-up), with R↔B swap
    assert_eq!(&out[0..4], &[120, 110, 100, 255]); // was [100,110,120,255]
    assert_eq!(&out[4..8], &[90, 80, 70, 255]); // was [70,80,90,255]
    assert_eq!(&out[8..12], &[60, 50, 40, 255]); // was [40,50,60,255]
    assert_eq!(&out[12..16], &[30, 20, 10, 255]); // was [10,20,30,255]
}

// -------------------------------------------------------------------------
// rgba_to_bgra_bottom_up — identity for 0-height
// -------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_zero_height_returns_empty() {
    let result = rgba_to_bgra_bottom_up(&[], 100, 0).unwrap();
    assert!(result.is_empty());
}

#[test]
fn rgba_to_bgra_bottom_up_zero_width_returns_empty() {
    let result = rgba_to_bgra_bottom_up(&[], 0, 100).unwrap();
    assert!(result.is_empty());
}

// -------------------------------------------------------------------------
// rgba_to_bgra_bottom_up — 2x2 image row reversal
// -------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_2x2_reverses_rows_and_swaps_channels() {
    // Row 0: pixel(R=1,G=2,B=3,A=4), pixel(R=5,G=6,B=7,A=8)
    // Row 1: pixel(R=9,G=10,B=11,A=12), pixel(R=13,G=14,B=15,A=16)
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let bgra = rgba_to_bgra_bottom_up(&rgba, 2, 2).unwrap();
    // After bottom-up: row 1 becomes row 0, row 0 becomes row 1
    // After R↔B swap:
    // New row 0 (originally row 1): (B=9,G=10,R=11,A=12), (B=13,G=14,R=15,A=16)
    //   → (11,10,9,12), (15,14,13,16)
    // New row 1 (originally row 0): (B=1,G=2,R=3,A=4), (B=5,G=6,R=7,A=8)
    //   → (3,2,1,4), (7,6,5,8)
    assert_eq!(
        bgra,
        vec![11, 10, 9, 12, 15, 14, 13, 16, 3, 2, 1, 4, 7, 6, 5, 8]
    );
}

// -------------------------------------------------------------------------
// Task 5.3: rgba_to_bgra_bottom_up and pack_u32_pair — requirement coverage
// Requirements: 9.1, 9.2, 9.4, 9.5, 9.6, 9.7, 9.8
// -------------------------------------------------------------------------

/// Req 9.5: alpha channel preserved for every pixel in a multi-row buffer
/// with varied alpha values (0, 128, 200, 255).
#[test]
fn rgba_to_bgra_bottom_up_alpha_preserved_for_every_pixel() {
    // 3x2 image with distinct alpha per pixel
    let rgba = vec![
        10, 20, 30, 0, 40, 50, 60, 128, 70, 80, 90, 200, // row 0
        100, 110, 120, 255, 130, 140, 150, 1, 160, 170, 180, 77, // row 1
    ];
    let out = rgba_to_bgra_bottom_up(&rgba, 3, 2).unwrap();

    // Collect original alpha values per pixel (row-major order)
    let original_alphas: Vec<u8> = rgba.chunks(4).map(|px| px[3]).collect();
    // After bottom-up flip, row 1 becomes row 0 and vice versa
    // So output pixel order is: row1_px0, row1_px1, row1_px2, row0_px0, row0_px1, row0_px2
    let expected_alphas = vec![
        original_alphas[3],
        original_alphas[4],
        original_alphas[5], // row 1 pixels
        original_alphas[0],
        original_alphas[1],
        original_alphas[2], // row 0 pixels
    ];

    let output_alphas: Vec<u8> = out.chunks(4).map(|px| px[3]).collect();
    assert_eq!(
        output_alphas, expected_alphas,
        "alpha channel must be preserved for every pixel after conversion"
    );
}
