//! Tests for JPEG pre-conversion, save errors, platform behavior, and Unicode paths.

use crate::capture::*;
use crate::test_support::TestDir;

// =========================================================================
// save_image_optimized
// =========================================================================

#[test]
fn save_image_optimized_jpg_matches_rgb_preconversion() {
    use image::codecs::jpeg::JpegEncoder;
    use std::io::BufWriter;

    let img = image::RgbaImage::from_fn(17, 9, |x, y| {
        image::Rgba([
            (x * 13) as u8,
            (y * 29) as u8,
            ((x + y) * 7) as u8,
            ((x * 11 + y * 17) % 256) as u8,
        ])
    });
    // One TestDir shared by both files this test writes.
    let dir = TestDir::new("rustshot_test_jpg_rgb_preconversion");
    let actual_path = dir.file("actual.jpg");
    let expected_path = dir.file("expected.jpg");

    save_image_optimized(&img, &actual_path, Format::Jpg).unwrap();

    let raw: Vec<u8> = img
        .pixels()
        .flat_map(|pixel| pixel.0[..3].iter().copied())
        .collect();
    let rgb = image::RgbImage::from_raw(img.width(), img.height(), raw).unwrap();
    let writer = BufWriter::new(std::fs::File::create(&expected_path).unwrap());
    rgb.write_with_encoder(JpegEncoder::new_with_quality(writer, 90))
        .unwrap();

    assert_eq!(
        std::fs::read(&actual_path).unwrap(),
        std::fs::read(&expected_path).unwrap()
    );
}

#[test]
fn save_image_optimized_invalid_path() {
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]));
    let path = std::path::Path::new("/nonexistent_dir_xyz/impossible/test.png");

    let result = save_image_optimized(&img, path, Format::Png);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to create file"));
}

// -------------------------------------------------------------------------
// capture_active_window platform tests
// -------------------------------------------------------------------------

#[cfg(not(windows))]
#[test]
fn capture_active_window_in_dir_returns_unsupported_on_linux() {
    let dir = TestDir::new("rustshot_test_active_window_unsupported");
    let result = capture_active_window_in_dir(None, Format::Png, dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}

// -------------------------------------------------------------------------
// ensure_parent_directory additional tests
// -------------------------------------------------------------------------

#[test]
fn ensure_parent_directory_unicode_path() {
    // The unicode-named directory itself must NOT pre-exist: the point of
    // the test is that ensure_parent_directory creates it. TestDir only
    // creates the (ASCII-named) root; the unicode child stays missing.
    let root = TestDir::new("rustshot_test_unicode");
    let dir = root.file("тест_юникод");
    let path = dir.join("file.png");
    assert!(ensure_parent_directory(&path).is_ok());
    assert!(dir.exists());
}
