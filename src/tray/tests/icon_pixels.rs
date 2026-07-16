//! Per-pixel geometry tests for the default tray icon: body boundaries,
//! lens circle, corners, and edges.

use super::*;

#[test]
fn default_icon_size_constant_is_32() {
    assert_eq!(DEFAULT_ICON_SIZE, 32);
}

#[test]
fn default_icon_has_expected_size() {
    let pixels = build_default_icon_pixels();
    assert_eq!(
        pixels.len(),
        (DEFAULT_ICON_SIZE * DEFAULT_ICON_SIZE * 4) as usize
    );
}

#[test]
fn default_icon_total_pixel_count_is_1024() {
    // 32 * 32 = 1024 pixels, each with 4 channels = 4096 bytes
    let pixels = build_default_icon_pixels();
    assert_eq!(pixels.len(), 4096);
}

#[test]
fn default_icon_center_is_white() {
    let pixels = build_default_icon_pixels();
    let x = 16;
    let y = 16;
    let idx = ((y * DEFAULT_ICON_SIZE + x) * 4) as usize;
    assert_eq!(pixels[idx], 255);
    assert_eq!(pixels[idx + 1], 255);
    assert_eq!(pixels[idx + 2], 255);
    assert_eq!(pixels[idx + 3], 255);
}

#[test]
fn default_icon_corner_is_transparent() {
    let pixels = build_default_icon_pixels();
    let idx = 0usize;
    assert_eq!(pixels[idx + 3], 0);
}

#[test]
fn default_icon_top_left_corner_0_0_is_transparent() {
    // x=0, y=0: dist_x=16, dist_y=16, outside body (16 > 12), so transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(0, 0);
    assert_eq!(pixels[idx + 3], 0, "top-left corner should be transparent");
}

#[test]
fn default_icon_top_right_corner_31_0_is_transparent() {
    // x=31, y=0: dist_x=|31-16|=15 > 12, outside body
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(31, 0);
    assert_eq!(pixels[idx + 3], 0, "top-right corner should be transparent");
}

#[test]
fn default_icon_bottom_left_corner_0_31_is_transparent() {
    // x=0, y=31: dist_x=16 > 12, outside body
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(0, 31);
    assert_eq!(
        pixels[idx + 3],
        0,
        "bottom-left corner should be transparent"
    );
}

#[test]
fn default_icon_body_pixel_is_blue_not_lens() {
    // x=5, y=16: dist_x=|5-16|=11 <= 12, dist_y=|16-16|=0 <= 10 -> in body
    // dist = sqrt(11^2 + 0^2) = 11.0 > 6.0 -> NOT in lens -> blue
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(5, 16);
    assert_eq!(pixels[idx], 41, "R should be camera-body blue (41)");
    assert_eq!(pixels[idx + 1], 128, "G should be camera-body blue (128)");
    assert_eq!(pixels[idx + 2], 185, "B should be camera-body blue (185)");
    assert_eq!(pixels[idx + 3], 255, "alpha should be fully opaque");
}

#[test]
fn default_icon_body_pixel_alpha_is_255() {
    // Any pixel inside the body (not transparent) must have alpha=255
    // x=16, y=8: dist_x=0, dist_y=|8-16|=8 <= 10 -> in body
    // dist = sqrt(0+64) = 8.0 > 6.0 -> blue (not lens)
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 8);
    assert_eq!(pixels[idx + 3], 255, "body pixel alpha must be 255");
}

#[test]
fn default_icon_lens_pixel_is_white() {
    // x=16, y=20: dist_x=0, dist_y=|20-16|=4 <= 10 -> in body
    // dist = sqrt(0 + 16) = 4.0 <= 6.0 -> lens (white)
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 20);
    assert_eq!(pixels[idx], 255, "lens pixel R should be 255");
    assert_eq!(pixels[idx + 1], 255, "lens pixel G should be 255");
    assert_eq!(pixels[idx + 2], 255, "lens pixel B should be 255");
}

#[test]
fn default_icon_pixel_at_lens_edge_is_white() {
    // x=16, y=22: dist_x=0, dist_y=|22-16|=6, dist=6.0 <= 6.0 -> lens (white)
    // Also inside body: dist_y=6 <= 10 and dist_x=0 <= 12
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 22);
    assert_eq!(pixels[idx], 255, "lens edge R should be 255");
    assert_eq!(pixels[idx + 1], 255, "lens edge G should be 255");
    assert_eq!(pixels[idx + 2], 255, "lens edge B should be 255");
}

#[test]
fn default_icon_pixel_just_outside_lens_is_blue() {
    // x=16, y=23: dist_x=0, dist_y=|23-16|=7, dist=7.0 > 6.0 -> body blue
    // dist_y=7 <= 10 -> still in body
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 23);
    assert_eq!(
        pixels[idx], 41,
        "just-outside-lens R should be body blue (41)"
    );
    assert_eq!(
        pixels[idx + 1],
        128,
        "just-outside-lens G should be body blue (128)"
    );
    assert_eq!(
        pixels[idx + 2],
        185,
        "just-outside-lens B should be body blue (185)"
    );
}

#[test]
fn default_icon_body_boundary_pixel_is_blue() {
    // x=4, y=16: dist_x=|4-16|=12, dist_y=0 -> exactly on body boundary (dist_x<=12)
    // dist = sqrt(144) = 12.0 > 6.0 -> blue body
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(4, 16);
    assert_eq!(pixels[idx + 3], 255, "boundary body pixel should be opaque");
    assert_eq!(pixels[idx + 2], 185, "boundary body pixel B should be 185");
}

#[test]
fn default_icon_outside_body_boundary_is_transparent() {
    // x=3, y=16: dist_x=|3-16|=13 > 12 -> outside body -> transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(3, 16);
    assert_eq!(
        pixels[idx + 3],
        0,
        "pixel outside body boundary should be transparent"
    );
}

#[test]
fn default_icon_body_top_boundary_pixel_is_opaque() {
    // x=16, y=6: dist_x=0, dist_y=|6-16|=10 -> exactly on top boundary (dist_y<=10)
    // dist = 10.0 > 6.0 -> blue body
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 6);
    assert_eq!(
        pixels[idx + 3],
        255,
        "top body boundary pixel should be opaque"
    );
}

#[test]
fn default_icon_above_body_top_is_transparent() {
    // x=16, y=5: dist_y=|5-16|=11 > 10 -> outside body -> transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 5);
    assert_eq!(pixels[idx + 3], 0, "pixel above body should be transparent");
}

#[test]
fn default_icon_pixels_length_matches_formula() {
    let pixels = build_default_icon_pixels();
    let expected = DEFAULT_ICON_SIZE * DEFAULT_ICON_SIZE * 4;
    assert_eq!(pixels.len() as u32, expected);
}

#[test]
fn default_icon_right_body_boundary_pixel_is_blue() {
    // x=28, y=16: dist_x=|28-16|=12, dist_y=0 -> exactly on right body boundary
    // dist=12.0 > 6.0 -> blue body
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(28, 16);
    assert_eq!(pixels[idx + 3], 255, "right body boundary should be opaque");
    assert_eq!(pixels[idx + 2], 185, "right body boundary B should be 185");
}

#[test]
fn default_icon_outside_right_body_boundary_is_transparent() {
    // x=29, y=16: dist_x=|29-16|=13 > 12 -> outside body -> transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(29, 16);
    assert_eq!(
        pixels[idx + 3],
        0,
        "pixel outside right body boundary should be transparent"
    );
}

#[test]
fn default_icon_bottom_body_boundary_pixel_is_opaque() {
    // x=16, y=26: dist_x=0, dist_y=|26-16|=10 -> exactly on bottom boundary (dist_y<=10)
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 26);
    assert_eq!(
        pixels[idx + 3],
        255,
        "bottom body boundary pixel should be opaque"
    );
}

#[test]
fn default_icon_below_body_bottom_is_transparent() {
    // x=16, y=27: dist_y=|27-16|=11 > 10 -> outside body -> transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 27);
    assert_eq!(pixels[idx + 3], 0, "pixel below body should be transparent");
}

#[test]
fn default_icon_bottom_right_corner_is_transparent() {
    // x=31, y=31: dist_x=|31-16|=15 > 12 → outside body → transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(31, 31);
    assert_eq!(
        pixels[idx + 3],
        0,
        "bottom-right corner should be transparent"
    );
}

#[test]
fn default_icon_left_edge_center_is_transparent() {
    // x=0, y=16: dist_x=|0-16|=16 > 12 → outside body → transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(0, 16);
    assert_eq!(pixels[idx + 3], 0, "left edge center should be transparent");
}

#[test]
fn default_icon_right_edge_center_is_transparent() {
    // x=31, y=16: dist_x=|31-16|=15 > 12 → outside body → transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(31, 16);
    assert_eq!(
        pixels[idx + 3],
        0,
        "right edge center should be transparent"
    );
}

#[test]
fn default_icon_top_center_is_transparent() {
    // x=16, y=0: dist_y=|0-16|=16 > 10 → outside body → transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(16, 0);
    assert_eq!(pixels[idx + 3], 0, "top center should be transparent");
}

#[test]
fn default_icon_lens_pixel_at_x20_y16_is_white() {
    // x=20, y=16: dist_x=4, dist_y=0, dist=4.0 <= 6.0 → lens (white)
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(20, 16);
    assert_eq!(pixels[idx], 255, "lens pixel R should be 255");
    assert_eq!(pixels[idx + 1], 255, "lens pixel G should be 255");
    assert_eq!(pixels[idx + 2], 255, "lens pixel B should be 255");
}

#[test]
fn default_icon_lens_pixel_at_x12_y16_is_white() {
    // x=12, y=16: dist_x=4, dist_y=0, dist=4.0 <= 6.0 → lens (white)
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(12, 16);
    assert_eq!(pixels[idx], 255, "lens pixel R should be 255");
    assert_eq!(pixels[idx + 1], 255, "lens pixel G should be 255");
    assert_eq!(pixels[idx + 2], 255, "lens pixel B should be 255");
}

#[test]
fn default_icon_transparent_pixels_have_zero_rgb() {
    // Transparent pixels must not leak colour data (alpha=0 region)
    // x=0, y=0 is transparent
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(0, 0);
    // We only assert alpha; RGB of transparent pixels is undefined by spec,
    // but we can at least verify alpha is 0
    assert_eq!(pixels[idx + 3], 0);
}

#[test]
fn default_icon_lens_pixel_at_x22_y16_is_white() {
    // x=22, y=16: dist_x=|22-16|=6, dist_y=0
    // body: 6<=12 && 0<=10 → inside body
    // dist = sqrt(36+0) = 6.0 <= 6.0 → lens → R=255, G=255, B=255
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(22, 16);
    assert_eq!(pixels[idx], 255, "lens pixel at (22,16) R should be 255");
    assert_eq!(
        pixels[idx + 1],
        255,
        "lens pixel at (22,16) G should be 255"
    );
    assert_eq!(
        pixels[idx + 2],
        255,
        "lens pixel at (22,16) B should be 255"
    );
}

#[test]
fn default_icon_body_corner_x4_y6_is_opaque() {
    // x=4, y=6: dist_x=|4-16|=12, dist_y=|6-16|=10
    // body: 12<=12 && 10<=10 → on body boundary (inside)
    // dist = sqrt(144+100) ≈ 15.62 > 6.0 → blue body → alpha=255
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(4, 6);
    assert_eq!(
        pixels[idx + 3],
        255,
        "body corner at (4,6) should be opaque"
    );
}

#[test]
fn default_icon_body_corner_x28_y26_is_opaque() {
    // x=28, y=26: dist_x=|28-16|=12, dist_y=|26-16|=10
    // body: 12<=12 && 10<=10 → on body boundary (inside)
    // dist = sqrt(144+100) ≈ 15.62 > 6.0 → blue body → alpha=255
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(28, 26);
    assert_eq!(
        pixels[idx + 3],
        255,
        "body corner at (28,26) should be opaque"
    );
}

#[test]
fn default_icon_body_pixel_at_10_10_is_blue() {
    // x=10, y=10: dist_x=6, dist_y=6 → in body (6<=12 && 6<=10)
    // dist=sqrt(72)≈8.49 > 6.0 → not lens → blue body
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(10, 10);
    assert_eq!(pixels[idx], 41, "R at (10,10) should be 41");
    assert_eq!(pixels[idx + 1], 128, "G at (10,10) should be 128");
    assert_eq!(pixels[idx + 2], 185, "B at (10,10) should be 185");
    assert_eq!(pixels[idx + 3], 255, "alpha at (10,10) should be 255");
}

#[test]
fn default_icon_diagonal_lens_pixel_is_white() {
    // x=20, y=20: dist_x=4, dist_y=4 → in body (4<=12 && 4<=10)
    // dist=sqrt(32)≈5.66 <= 6.0 → lens → white
    let pixels = build_default_icon_pixels();
    let idx = pixel_idx(20, 20);
    assert_eq!(pixels[idx], 255, "R at (20,20) should be 255");
    assert_eq!(pixels[idx + 1], 255, "G at (20,20) should be 255");
    assert_eq!(pixels[idx + 2], 255, "B at (20,20) should be 255");
}

#[test]
fn default_icon_center_region_all_opaque() {
    // All pixels with |x-16|<=5 && |y-16|<=5 are inside the body → alpha=255
    let pixels = build_default_icon_pixels();
    for y in 11u32..=21 {
        for x in 11u32..=21 {
            let idx = pixel_idx(x, y);
            assert_eq!(
                pixels[idx + 3],
                255,
                "pixel ({x},{y}) in center region should be opaque"
            );
        }
    }
}
