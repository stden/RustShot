//! Whole-buffer invariant tests for the default tray icon: alpha values,
//! pixel counts, and symmetry.

use super::*;

#[test]
fn default_icon_body_pixel_rgb_values_are_camera_blue() {
    // Any opaque non-lens pixel must be exactly RGB(41, 128, 185)
    let pixels = build_default_icon_pixels();
    for i in (0..pixels.len()).step_by(4) {
        let r = pixels[i];
        let g = pixels[i + 1];
        let b = pixels[i + 2];
        let a = pixels[i + 3];
        if a == 255 && !(r == 255 && g == 255 && b == 255) {
            assert_eq!(r, 41, "body pixel R at {i}");
            assert_eq!(g, 128, "body pixel G at {i}");
            assert_eq!(b, 185, "body pixel B at {i}");
        }
    }
}

#[test]
fn default_icon_all_white_pixels_are_opaque() {
    // Any pixel that is RGB(255,255,255) must also have alpha=255
    let pixels = build_default_icon_pixels();
    for i in (0..pixels.len()).step_by(4) {
        if pixels[i] == 255 && pixels[i + 1] == 255 && pixels[i + 2] == 255 {
            assert_eq!(
                pixels[i + 3],
                255,
                "white pixel at byte {i} must be fully opaque"
            );
        }
    }
}

// --- icon symmetry ---

#[test]
fn default_icon_body_has_blue_pixels_on_both_horizontal_halves() {
    let pixels = build_default_icon_pixels();
    let midpoint = DEFAULT_ICON_SIZE / 2;

    let left_body = (0..DEFAULT_ICON_SIZE).any(|y| {
        (0..midpoint).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx] == 41
                && pixels[idx + 1] == 128
                && pixels[idx + 2] == 185
                && pixels[idx + 3] == 255
        })
    });
    let right_body = (0..DEFAULT_ICON_SIZE).any(|y| {
        (midpoint..DEFAULT_ICON_SIZE).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx] == 41
                && pixels[idx + 1] == 128
                && pixels[idx + 2] == 185
                && pixels[idx + 3] == 255
        })
    });

    assert!(left_body, "left half should contain blue body pixels");
    assert!(right_body, "right half should contain blue body pixels");
}

#[test]
fn default_icon_body_has_blue_pixels_on_both_vertical_halves() {
    let pixels = build_default_icon_pixels();
    let midpoint = DEFAULT_ICON_SIZE / 2;

    let top_body = (0..midpoint).any(|y| {
        (0..DEFAULT_ICON_SIZE).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx] == 41
                && pixels[idx + 1] == 128
                && pixels[idx + 2] == 185
                && pixels[idx + 3] == 255
        })
    });
    let bottom_body = (midpoint..DEFAULT_ICON_SIZE).any(|y| {
        (0..DEFAULT_ICON_SIZE).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx] == 41
                && pixels[idx + 1] == 128
                && pixels[idx + 2] == 185
                && pixels[idx + 3] == 255
        })
    });

    assert!(top_body, "top half should contain blue body pixels");
    assert!(bottom_body, "bottom half should contain blue body pixels");
}

#[test]
fn build_default_icon_pixels_all_alpha_zero_or_255() {
    let pixels = build_default_icon_pixels();
    for (i, chunk) in pixels.chunks(4).enumerate() {
        let alpha = chunk[3];
        assert!(
            alpha == 0 || alpha == 255,
            "pixel {i} has unexpected alpha {alpha}"
        );
    }
}

// --- Requirement 16.4: horizontal symmetry of opaque/transparent pattern ---

#[test]
fn default_icon_horizontal_symmetry_of_opacity() {
    // The icon is centered at x=16. For distance d from center, pixels at
    // x=(16-d) and x=(16+d) have the same |x-16| so the body/lens conditions
    // produce identical alpha values. We verify alpha(16-d, y) == alpha(16+d, y)
    // for d=1..=15 (d=0 is the center column, d=16 would be x=0 vs x=32 which
    // is out of bounds).
    let pixels = build_default_icon_pixels();
    for y in 0..DEFAULT_ICON_SIZE {
        for d in 1..=15u32 {
            let left_x = 16 - d;
            let right_x = 16 + d;
            let left_alpha = pixels[pixel_idx(left_x, y) + 3];
            let right_alpha = pixels[pixel_idx(right_x, y) + 3];
            assert_eq!(
                left_alpha, right_alpha,
                "horizontal symmetry broken at row {y}: pixel ({left_x}) alpha={left_alpha} vs pixel ({right_x}) alpha={right_alpha}"
            );
        }
    }
}
