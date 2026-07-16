//! Whole-buffer invariant tests for the default tray icon: alpha values,
//! pixel counts, and symmetry.

use super::*;

#[test]
fn default_icon_no_pixel_has_partial_alpha() {
    // Every pixel is either fully opaque (255) or fully transparent (0)
    let pixels = build_default_icon_pixels();
    for i in (3..pixels.len()).step_by(4) {
        let alpha = pixels[i];
        assert!(
            alpha == 0 || alpha == 255,
            "alpha at byte {i} should be 0 or 255, got {alpha}"
        );
    }
}

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
fn default_icon_opaque_pixel_count_is_positive() {
    // At least one pixel must have alpha == 255 (the camera body)
    let pixels = build_default_icon_pixels();
    let count = (3..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 255)
        .count();
    assert!(count > 0, "there should be at least one opaque pixel");
}

#[test]
fn default_icon_transparent_pixel_count_is_positive() {
    // At least one pixel must have alpha == 0 (the rounded corners / outside the body)
    let pixels = build_default_icon_pixels();
    let count = (3..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 0)
        .count();
    assert!(count > 0, "there should be at least one transparent pixel");
}

#[test]
fn default_icon_opaque_count_exceeds_transparent_count() {
    // Body is 25×21=525 pixels; total=1024; so opaque > transparent
    let pixels = build_default_icon_pixels();
    let opaque = (3..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 255)
        .count();
    let transparent = (3..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 0)
        .count();
    assert!(
        opaque > transparent,
        "opaque({opaque}) should exceed transparent({transparent})"
    );
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

#[test]
fn default_icon_pixel_count_sums_to_total() {
    // Every pixel is either fully opaque or fully transparent; the sum must equal total pixels
    let pixels = build_default_icon_pixels();
    let total_pixels = (DEFAULT_ICON_SIZE * DEFAULT_ICON_SIZE) as usize;
    let opaque_count = (3..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 255)
        .count();
    let transparent_count = (3..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 0)
        .count();
    assert_eq!(
        opaque_count + transparent_count,
        total_pixels,
        "opaque + transparent must equal total pixels (1024)"
    );
}

// --- icon pixel statistics ---

#[test]
fn default_icon_lens_pixel_count_is_positive() {
    // Count white (lens) pixels: R=255, G=255, B=255
    let pixels = build_default_icon_pixels();
    let lens_count = (0..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 255 && pixels[i + 1] == 255 && pixels[i + 2] == 255)
        .count();
    assert!(
        lens_count > 0,
        "there should be at least one lens (white) pixel"
    );
}

#[test]
fn default_icon_body_pixel_count_is_positive() {
    // Count body (camera blue) pixels: R=41, G=128, B=185
    let pixels = build_default_icon_pixels();
    let body_count = (0..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 41 && pixels[i + 1] == 128 && pixels[i + 2] == 185)
        .count();
    assert!(
        body_count > 0,
        "there should be at least one body (blue) pixel"
    );
}

#[test]
fn default_icon_lens_is_smaller_than_body() {
    // Lens (white circle radius 6) should have fewer pixels than the body (25x21 rect minus lens)
    let pixels = build_default_icon_pixels();
    let lens_count = (0..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 255 && pixels[i + 1] == 255 && pixels[i + 2] == 255)
        .count();
    let body_count = (0..pixels.len())
        .step_by(4)
        .filter(|&i| pixels[i] == 41 && pixels[i + 1] == 128 && pixels[i + 2] == 185)
        .count();
    assert!(
        lens_count < body_count,
        "lens ({lens_count}) should have fewer pixels than body ({body_count})"
    );
}

// --- icon symmetry ---

#[test]
fn default_icon_has_opaque_pixels_on_both_horizontal_halves() {
    let pixels = build_default_icon_pixels();
    let midpoint = DEFAULT_ICON_SIZE / 2;

    let left_opaque = (0..DEFAULT_ICON_SIZE).any(|y| {
        (0..midpoint).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx + 3] == 255
        })
    });
    let right_opaque = (0..DEFAULT_ICON_SIZE).any(|y| {
        (midpoint..DEFAULT_ICON_SIZE).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx + 3] == 255
        })
    });

    assert!(left_opaque, "left half should contain opaque icon pixels");
    assert!(right_opaque, "right half should contain opaque icon pixels");
}

#[test]
fn default_icon_has_opaque_pixels_on_both_vertical_halves() {
    let pixels = build_default_icon_pixels();
    let midpoint = DEFAULT_ICON_SIZE / 2;

    let top_opaque = (0..midpoint).any(|y| {
        (0..DEFAULT_ICON_SIZE).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx + 3] == 255
        })
    });
    let bottom_opaque = (midpoint..DEFAULT_ICON_SIZE).any(|y| {
        (0..DEFAULT_ICON_SIZE).any(|x| {
            let idx = pixel_idx(x, y);
            pixels[idx + 3] == 255
        })
    });

    assert!(top_opaque, "top half should contain opaque icon pixels");
    assert!(
        bottom_opaque,
        "bottom half should contain opaque icon pixels"
    );
}

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
fn default_icon_size_is_power_of_two() {
    assert!(DEFAULT_ICON_SIZE.is_power_of_two());
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
