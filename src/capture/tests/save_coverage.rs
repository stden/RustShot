//! Semantic image round-trips and live screen-list formatting coverage.

use crate::capture::*;
use crate::test_support::TestDir;
use image::GenericImageView;
use std::path::PathBuf;

// -------------------------------------------------------------------------
// list_screens — does not panic and every line is well-formed
// -------------------------------------------------------------------------

/// This deliberately does NOT assert that two consecutive calls return
/// identical lists. That is a property of the display server, not of our
/// code — and virtual displays (xvfb on CI) can legitimately differ between
/// back-to-back enumerations. What RustShot owns is: never panic, and
/// format every screen as "Screen <index>: <w>x<h>".
#[test]
fn list_screens_never_panics_and_lines_are_well_formed() {
    for screens in [list_screens(), list_screens()] {
        for (i, line) in screens.iter().enumerate() {
            let expected_prefix = format!("Screen {i}: ");
            assert!(
                line.starts_with(&expected_prefix),
                "line {i} must start with {expected_prefix:?}, got {line:?}"
            );
            let dims = &line[expected_prefix.len()..];
            let (w, h) = dims
                .split_once('x')
                .unwrap_or_else(|| panic!("line {i} must contain WxH, got {line:?}"));
            assert!(
                w.parse::<u32>().is_ok() && h.parse::<u32>().is_ok(),
                "dimensions must be numeric, got {line:?}"
            );
        }
    }
}

// --- save_image_optimized (additional coverage) ---

fn make_patterned_image(width: u32, height: u32) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255]);
    }
    img
}

#[test]
fn save_image_optimized_png_is_lossless_roundtrip() {
    let dir = TestDir::new("rustshot_test_png_lossless");
    let path = dir.file("exact.png");
    let img = make_patterned_image(16, 16);
    save_image_optimized(&img, &path, Format::Png).unwrap();
    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(img.as_raw(), loaded.as_raw());
}

#[test]
fn save_image_optimized_bmp_non_square_dimensions() {
    let dir = TestDir::new("rustshot_test_bmp_nonsquare");
    let path = dir.file("rect.bmp");
    let img = make_patterned_image(13, 7);
    save_image_optimized(&img, &path, Format::Bmp).unwrap();
    let loaded = image::open(&path).unwrap();
    assert_eq!(loaded.width(), 13);
    assert_eq!(loaded.height(), 7);
}

#[test]
fn save_image_optimized_all_formats_same_image() {
    // One TestDir shared by all three files this test writes.
    let dir = TestDir::new("rustshot_test_all_fmts");
    let img = make_patterned_image(8, 8);
    for (fmt, ext) in [
        (Format::Png, "png"),
        (Format::Jpg, "jpg"),
        (Format::Bmp, "bmp"),
    ] {
        let path = dir.file(&format!("out.{ext}"));
        save_image_optimized(&img, &path, fmt).unwrap();
        let loaded = image::open(&path).unwrap();
        assert_eq!(loaded.width(), 8);
        assert_eq!(loaded.height(), 8);
    }
}

#[test]
fn save_image_optimized_1x1_bmp_roundtrip_pixel() {
    // Verify 1x1 pixel edge case for BMP (lossless, should preserve pixel)
    let dir = TestDir::new("rustshot-1x1-bmp-rt");
    let path = dir.file("1x1.bmp");

    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
    save_image_optimized(&img, &path, Format::Bmp).unwrap();

    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (1, 1));
    assert_eq!(loaded.get_pixel(0, 0), &image::Rgba([128, 64, 32, 255]));
}

#[test]
fn save_image_optimized_1x1_jpg_roundtrip_dimensions() {
    let dir = TestDir::new("rustshot-1x1-jpg-rt");
    let path = dir.file("1x1.jpg");
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));

    save_image_optimized(&img, &path, Format::Jpg).unwrap();

    assert_eq!(image::open(&path).unwrap().dimensions(), (1, 1));
}

#[test]
fn resolve_output_path_in_dir_two_calls_produce_different_names() {
    let dir = PathBuf::from("/tmp/test-shots");
    let first = resolve_output_path_in_dir(None, Format::Png, &dir);
    std::thread::sleep(std::time::Duration::from_millis(1));
    let second = resolve_output_path_in_dir(None, Format::Png, &dir);

    assert_ne!(first, second);
}
