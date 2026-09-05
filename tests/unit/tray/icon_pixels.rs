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
fn default_icon_corners_are_transparent() {
    let pixels = build_default_icon_pixels();
    let last = DEFAULT_ICON_SIZE - 1;
    for (x, y) in [(0, 0), (last, 0), (0, last), (last, last)] {
        assert_eq!(
            pixels[pixel_idx(x, y) + 3],
            0,
            "corner ({x},{y}) should be transparent"
        );
    }
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
