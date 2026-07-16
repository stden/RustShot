//! Tests for `save_image_optimized` output validity (magic bytes, JPEG pre-conversion, error paths) plus `bgra_to_rgba` and path-helper edge cases.

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
// save_image_optimized format tests
// -------------------------------------------------------------------------

#[test]
fn save_image_optimized_png_creates_valid_file() {
    let dir = TestDir::new("rustshot_test_save_png");
    let path = dir.file("test.png");

    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    save_image_optimized(&img, &path, Format::Png).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() > 8);
    // PNG magic bytes
    assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);
}

#[test]
fn save_image_optimized_jpg_creates_valid_file() {
    let dir = TestDir::new("rustshot_test_save_jpg");
    let path = dir.file("test.jpg");

    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
    save_image_optimized(&img, &path, Format::Jpg).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() > 2);
    // JPEG magic bytes
    assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
}

#[test]
fn save_image_optimized_bmp_creates_valid_file() {
    let dir = TestDir::new("rustshot_test_save_bmp");
    let path = dir.file("test.bmp");

    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 255]));
    save_image_optimized(&img, &path, Format::Bmp).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() > 2);
    // BMP magic bytes
    assert_eq!(&bytes[..2], b"BM");
}

#[test]
fn save_image_optimized_1x1_pixel_png() {
    let dir = TestDir::new("rustshot_test_save_1x1");
    let path = dir.file("1x1.png");

    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
    save_image_optimized(&img, &path, Format::Png).unwrap();

    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.get_pixel(0, 0), &image::Rgba([128, 64, 32, 255]));
}

// -------------------------------------------------------------------------
// bgra_to_rgba additional edge cases
// -------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_exact_8_pixels() {
    let mut buf: Vec<u8> = (0..32).collect();
    let original = buf.clone();
    bgra_to_rgba(&mut buf);
    // R and B swapped for each pixel
    for i in 0..8 {
        let base = i * 4;
        assert_eq!(buf[base], original[base + 2]); // R = old B
        assert_eq!(buf[base + 1], original[base + 1]); // G unchanged
        assert_eq!(buf[base + 2], original[base]); // B = old R
        assert_eq!(buf[base + 3], original[base + 3]); // A unchanged
    }
}

#[test]
fn bgra_to_rgba_5_extra_bytes_ignored() {
    let mut buf = vec![10, 20, 30, 40, 50, 60, 70, 80, 99];
    bgra_to_rgba(&mut buf);
    // First pixel swapped
    assert_eq!(buf[0], 30);
    assert_eq!(buf[2], 10);
    // Second pixel swapped
    assert_eq!(buf[4], 70);
    assert_eq!(buf[6], 50);
    // Extra byte untouched
    assert_eq!(buf[8], 99);
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
