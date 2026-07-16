//! Tests for `capture_screenshot_with` error propagation, `save_image_optimized`, and screenshots-directory helpers.

use crate::capture::*;

// =========================================================================
// capture_screenshot_with — DI tests
// =========================================================================

#[test]
fn capture_screenshot_with_save_failure_propagates() {
    let result = capture_screenshot_with(
        1,
        Some(0),
        || Ok(vec![0u8; 4]),
        None,
        Format::Png,
        std::path::Path::new("/tmp"),
        |_| Ok(()),
        |_img: &Vec<u8>, _path: &std::path::Path, _fmt: ImageFormat| Err("disk full".to_string()),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("disk full"));
}

#[test]
fn capture_screenshot_with_capture_failure_propagates() {
    let result = capture_screenshot_with::<_, _, _, Vec<u8>>(
        1,
        Some(0),
        || Err("no screen".to_string()),
        None,
        Format::Png,
        std::path::Path::new("/tmp"),
        |_| Ok(()),
        |_img, _path, _fmt| Ok(()),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no screen"));
}

#[test]
fn capture_screenshot_with_ensure_dir_failure_propagates() {
    let result = capture_screenshot_with(
        1,
        Some(0),
        || Ok(vec![0u8; 4]),
        None,
        Format::Png,
        std::path::Path::new("/tmp"),
        |_| Err("permission denied".to_string()),
        |_img: &Vec<u8>, _path: &std::path::Path, _fmt: ImageFormat| Ok(()),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("permission denied"));
}

#[test]
fn capture_screenshot_with_zero_screens_returns_error() {
    let result = capture_screenshot_with(
        0,
        None,
        || Ok(vec![0u8; 4]),
        None,
        Format::Png,
        std::path::Path::new("/tmp"),
        |_| Ok(()),
        |_img: &Vec<u8>, _path: &std::path::Path, _fmt: ImageFormat| Ok(()),
    );
    assert!(result.is_err());
}

// =========================================================================
// get_screenshots_dir
// =========================================================================

#[test]
fn get_screenshots_dir_ends_with_rustshot() {
    let dir = get_screenshots_dir();
    assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("RustShot"));
}

#[test]
fn get_screenshots_dir_is_consistent() {
    let d1 = get_screenshots_dir();
    let d2 = get_screenshots_dir();
    assert_eq!(d1, d2);
}

// =========================================================================
// list_screens
// =========================================================================

#[test]
fn list_screens_returns_string() {
    let result = list_screens();
    // Should either succeed with screen info or fail gracefully
    // On CI/headless it might fail, but should not panic
    let _ = result;
}

// =========================================================================
// save_image_optimized
// =========================================================================

#[test]
fn save_image_optimized_png() {
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 255]));
    let dir = std::env::temp_dir().join("rustshot_test_save_opt_png");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.png");

    save_image_optimized(&img, &path, Format::Png).unwrap();
    assert!(path.exists());
    // Verify it's a valid PNG by re-reading
    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (8, 8));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_jpg() {
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 255, 0, 255]));
    let dir = std::env::temp_dir().join("rustshot_test_save_opt_jpg");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.jpg");

    save_image_optimized(&img, &path, Format::Jpg).unwrap();
    assert!(path.exists());
    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (8, 8));

    let _ = std::fs::remove_dir_all(&dir);
}

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
    let dir = std::env::temp_dir().join("rustshot_test_jpg_rgb_preconversion");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let actual_path = dir.join("actual.jpg");
    let expected_path = dir.join("expected.jpg");

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

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_bmp() {
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 255, 255]));
    let dir = std::env::temp_dir().join("rustshot_test_save_opt_bmp");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.bmp");

    save_image_optimized(&img, &path, Format::Bmp).unwrap();
    assert!(path.exists());
    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (8, 8));

    let _ = std::fs::remove_dir_all(&dir);
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
fn capture_active_window_image_returns_unsupported_on_linux() {
    let result = capture_active_window_image();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}

#[cfg(not(windows))]
#[test]
fn capture_active_window_in_dir_returns_unsupported_on_linux() {
    let dir = std::env::temp_dir();
    let result = capture_active_window_in_dir(None, Format::Png, &dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("only supported on Windows"));
}

// -------------------------------------------------------------------------
// save_image_optimized format tests
// -------------------------------------------------------------------------

#[test]
fn save_image_optimized_png_creates_valid_file() {
    let dir = std::env::temp_dir().join("rustshot_test_save_png");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.png");

    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    save_image_optimized(&img, &path, Format::Png).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() > 8);
    // PNG magic bytes
    assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_jpg_creates_valid_file() {
    let dir = std::env::temp_dir().join("rustshot_test_save_jpg");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.jpg");

    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
    save_image_optimized(&img, &path, Format::Jpg).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() > 2);
    // JPEG magic bytes
    assert_eq!(&bytes[..2], &[0xFF, 0xD8]);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_bmp_creates_valid_file() {
    let dir = std::env::temp_dir().join("rustshot_test_save_bmp");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.bmp");

    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 255]));
    save_image_optimized(&img, &path, Format::Bmp).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.len() > 2);
    // BMP magic bytes
    assert_eq!(&bytes[..2], b"BM");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_1x1_pixel_png() {
    let dir = std::env::temp_dir().join("rustshot_test_save_1x1");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("1x1.png");

    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
    save_image_optimized(&img, &path, Format::Png).unwrap();

    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.get_pixel(0, 0), &image::Rgba([128, 64, 32, 255]));

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------
// capture_screenshot_in_dir / capture_screenshot delegation
// -------------------------------------------------------------------------

#[test]
fn capture_screenshot_with_explicit_path_in_dir_returns_explicit_path() {
    let dir = std::env::temp_dir().join("rustshot_test_csid");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let explicit_path = dir.join("explicit.png");
    let result = capture_screenshot_with(
        2,
        Some(0),
        || {
            Ok(image::RgbaImage::from_pixel(
                2,
                2,
                image::Rgba([0, 0, 0, 255]),
            ))
        },
        Some(explicit_path.clone()),
        Format::Png,
        &dir,
        |_| Ok(()),
        |_img, _path, _fmt| Ok(()),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), explicit_path);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn capture_screenshot_with_no_path_generates_path_in_dir() {
    let dir = std::env::temp_dir().join("rustshot_test_csid2");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let result = capture_screenshot_with(
        1,
        Some(0),
        || {
            Ok(image::RgbaImage::from_pixel(
                2,
                2,
                image::Rgba([0, 0, 0, 255]),
            ))
        },
        None,
        Format::Jpg,
        &dir,
        |_| Ok(()),
        |_img, _path, _fmt| Ok(()),
    );
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.extension().unwrap(), "jpg");
    assert_eq!(path.parent().unwrap(), dir);

    let _ = std::fs::remove_dir_all(&dir);
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
// Format additional tests
// -------------------------------------------------------------------------

#[test]
fn format_all_variants_have_distinct_extensions() {
    let exts: Vec<&str> = [Format::Png, Format::Jpg, Format::Bmp]
        .iter()
        .map(|f| f.extension())
        .collect();
    assert_eq!(exts.len(), 3);
    assert_ne!(exts[0], exts[1]);
    assert_ne!(exts[1], exts[2]);
    assert_ne!(exts[0], exts[2]);
}

#[test]
fn format_all_variants_have_distinct_image_formats() {
    let fmts: Vec<image::ImageFormat> = [Format::Png, Format::Jpg, Format::Bmp]
        .iter()
        .map(|f| f.to_image_format())
        .collect();
    assert_ne!(fmts[0], fmts[1]);
    assert_ne!(fmts[1], fmts[2]);
}

// -------------------------------------------------------------------------
// ensure_parent_directory additional tests
// -------------------------------------------------------------------------

#[test]
fn ensure_parent_directory_unicode_path() {
    let dir = std::env::temp_dir().join("rustshot_тест_юникод");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("file.png");
    assert!(ensure_parent_directory(&path).is_ok());
    assert!(dir.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ensure_parent_directory_already_exists_is_ok() {
    let dir = std::env::temp_dir().join("rustshot_existing_dir_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("file.png");
    assert!(ensure_parent_directory(&path).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}
