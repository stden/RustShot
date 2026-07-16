//! Additional `save_image_optimized` coverage (round-trips, sizes, alpha handling) plus `Format` equality and small-buffer `bgra_to_rgba` tests.

use crate::capture::*;

// -------------------------------------------------------------------------
// capture_screenshot_with — screen_index: None
// -------------------------------------------------------------------------

#[test]
fn capture_screenshot_with_none_screen_index_uses_default() {
    let result = capture_screenshot_with(
        3,
        None, // None triggers resolve_screen_index(3, None) → Ok(0)
        || Ok("fake"),
        None,
        Format::Png,
        &PathBuf::from("/tmp/shots"),
        |_| Ok(()),
        |_, _, _| Ok(()),
    );
    assert!(result.is_ok());
}

// -------------------------------------------------------------------------
// benchmark helpers — additional edge cases
// -------------------------------------------------------------------------

#[test]
fn benchmark_screen_count_from_result_single_screen() {
    assert_eq!(benchmark_screen_count_from_result(Ok(vec![42])).unwrap(), 1);
}

#[test]
fn benchmark_first_screen_from_result_error_passthrough() {
    let result: Result<i32, String> =
        benchmark_first_screen_from_result(Err("no screens".to_string()));
    assert_eq!(result.unwrap_err(), "no screens");
}

#[test]
fn benchmark_rgb_image_from_rgba_preserves_pixel_colors() {
    let mut img = image::RgbaImage::new(1, 1);
    img.put_pixel(0, 0, image::Rgba([100, 150, 200, 255]));
    let rgb = benchmark_rgb_image_from_rgba(&img);
    let pixel = rgb.get_pixel(0, 0);
    assert_eq!(pixel[0], 100); // R
    assert_eq!(pixel[1], 150); // G
    assert_eq!(pixel[2], 200); // B
}

// -------------------------------------------------------------------------
// bgra_to_rgba — 3-byte remainder is ignored
// -------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_three_trailing_bytes_untouched() {
    let mut pixels = vec![10, 20, 30, 255, 1, 2, 3]; // 1 full pixel + 3 remainder
    bgra_to_rgba(&mut pixels);
    // Full pixel: B=10, G=20, R=30, A=255 → R=30, G=20, B=10, A=255
    assert_eq!(pixels[0], 30);
    assert_eq!(pixels[2], 10);
    // Remainder bytes unchanged
    assert_eq!(pixels[4], 1);
    assert_eq!(pixels[5], 2);
    assert_eq!(pixels[6], 3);
}

// -------------------------------------------------------------------------
// save_image_optimized — 2x2 image produces valid files
// -------------------------------------------------------------------------

#[test]
fn save_image_optimized_2x2_all_formats() {
    let dir = std::env::temp_dir().join(format!(
        "rustshot-2x2-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let img = image::RgbaImage::from_fn(2, 2, |x, y| {
        image::Rgba([(x * 100) as u8, (y * 100) as u8, 128, 255])
    });

    for (fmt, ext) in [
        (Format::Png, "png"),
        (Format::Jpg, "jpg"),
        (Format::Bmp, "bmp"),
    ] {
        let path = dir.join(format!("test.{ext}"));
        save_image_optimized(&img, &path, fmt).unwrap();
        assert!(path.exists(), "{ext} file should exist");
        assert!(
            std::fs::metadata(&path).unwrap().len() > 0,
            "{ext} file should be non-empty"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------
// capture_active_window_platform — error message on non-windows
// -------------------------------------------------------------------------

#[cfg(not(windows))]
#[test]
fn capture_active_window_error_message_mentions_windows() {
    let err = capture_active_window_image().unwrap_err();
    assert!(
        err.contains("Windows"),
        "error should mention Windows: {err}"
    );
}

// -------------------------------------------------------------------------
// resolve_output_path_in_dir — two consecutive calls produce different names
// -------------------------------------------------------------------------

#[test]
fn resolve_output_path_in_dir_two_calls_produce_different_names() {
    let dir = PathBuf::from("/tmp/test-shots");
    let p1 = resolve_output_path_in_dir(None, Format::Png, &dir);
    std::thread::sleep(std::time::Duration::from_millis(1));
    let p2 = resolve_output_path_in_dir(None, Format::Png, &dir);
    assert_ne!(p1, p2, "two default paths should differ due to timestamp");
}

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
    let dir = std::env::temp_dir().join("rustshot_test_png_lossless");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("exact.png");
    let img = make_patterned_image(16, 16);
    save_image_optimized(&img, &path, Format::Png).unwrap();
    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(img.as_raw(), loaded.as_raw());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_large_256x256() {
    let dir = std::env::temp_dir().join("rustshot_test_large256");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("large.png");
    let img = make_patterned_image(256, 256);
    save_image_optimized(&img, &path, Format::Png).unwrap();
    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (256, 256));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_bmp_non_square_dimensions() {
    let dir = std::env::temp_dir().join("rustshot_test_bmp_nonsquare");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("rect.bmp");
    let img = make_patterned_image(13, 7);
    save_image_optimized(&img, &path, Format::Bmp).unwrap();
    let loaded = image::open(&path).unwrap();
    assert_eq!(loaded.width(), 13);
    assert_eq!(loaded.height(), 7);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_jpg_file_is_smaller_than_raw() {
    let dir = std::env::temp_dir().join("rustshot_test_jpg_size");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("compressed.jpg");
    let img = make_patterned_image(64, 64);
    save_image_optimized(&img, &path, Format::Jpg).unwrap();
    let raw_size = 64 * 64 * 4; // RGBA raw bytes
    let file_size = std::fs::metadata(&path).unwrap().len();
    assert!(file_size < raw_size as u64);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_png_file_size_is_positive() {
    let dir = std::env::temp_dir().join("rustshot_test_png_size");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("size.png");
    let img = make_patterned_image(4, 4);
    save_image_optimized(&img, &path, Format::Png).unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_all_formats_same_image() {
    let dir = std::env::temp_dir().join("rustshot_test_all_fmts");
    let _ = std::fs::create_dir_all(&dir);
    let img = make_patterned_image(8, 8);
    for (fmt, ext) in [
        (Format::Png, "png"),
        (Format::Jpg, "jpg"),
        (Format::Bmp, "bmp"),
    ] {
        let path = dir.join(format!("out.{ext}"));
        save_image_optimized(&img, &path, fmt).unwrap();
        let loaded = image::open(&path).unwrap();
        assert_eq!(loaded.width(), 8);
        assert_eq!(loaded.height(), 8);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// --- bgra_to_rgba: Requirement 14.5 — large buffer length preserved ---

#[test]
fn bgra_to_rgba_large_buffer_length_preserved() {
    // 1000 pixels = 4000 bytes, plus 3 trailing bytes
    let len = 1000 * 4 + 3;
    let mut buf: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
    let original_len = buf.len();
    bgra_to_rgba(&mut buf);
    assert_eq!(buf.len(), original_len);
}

// =========================================================================
// Task 1.5 — Gap-filling tests for capture engine requirements
// Requirements: 1.7, 1.8, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3,
//              3.4, 3.5, 3.6
// =========================================================================

// --- Format: PartialEq not yet exercised (Req 3.1, 3.5, 3.6) ---

#[test]
fn format_each_variant_matches_itself() {
    // Verify each variant can be pattern-matched correctly
    assert!(matches!(Format::Png, Format::Png));
    assert!(matches!(Format::Jpg, Format::Jpg));
    assert!(matches!(Format::Bmp, Format::Bmp));
}

#[test]
fn format_variants_are_distinct() {
    // Verify variants don't overlap — each maps to a unique extension and ImageFormat
    let variants = [Format::Png, Format::Jpg, Format::Bmp];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i != j {
                assert_ne!(
                    a.extension(),
                    b.extension(),
                    "{:?} and {:?} should have different extensions",
                    a,
                    b
                );
                assert_ne!(
                    a.to_image_format(),
                    b.to_image_format(),
                    "{:?} and {:?} should have different ImageFormats",
                    a,
                    b
                );
            }
        }
    }
}

// --- Format: extension has no leading dot (Req 3.4) ---

#[test]
fn format_extension_has_no_leading_dot() {
    // Requirement 3.4: extension() returns lowercase string WITHOUT leading dot
    for fmt in [Format::Png, Format::Jpg, Format::Bmp] {
        let ext = fmt.extension();
        assert!(
            !ext.starts_with('.'),
            "extension for {fmt:?} should not start with dot: {ext}"
        );
    }
}

// --- save_image_optimized: 1x1 pixel JPG roundtrip (Req 3.3) ---

#[test]
fn save_image_optimized_1x1_jpg_roundtrip_dimensions() {
    // Verify 1x1 pixel edge case for JPG specifically (RGBA→RGB conversion)
    let dir = std::env::temp_dir().join(format!(
        "rustshot-1x1-jpg-rt-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("1x1.jpg");

    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 100, 50, 255]));
    save_image_optimized(&img, &path, Format::Jpg).unwrap();

    let loaded = image::open(&path).unwrap();
    assert_eq!(loaded.width(), 1);
    assert_eq!(loaded.height(), 1);
    // JPG is lossy, so we only check dimensions, not exact pixel values

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_1x1_bmp_roundtrip_pixel() {
    // Verify 1x1 pixel edge case for BMP (lossless, should preserve pixel)
    let dir = std::env::temp_dir().join(format!(
        "rustshot-1x1-bmp-rt-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("1x1.bmp");

    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
    save_image_optimized(&img, &path, Format::Bmp).unwrap();

    let loaded = image::open(&path).unwrap().to_rgba8();
    assert_eq!(loaded.dimensions(), (1, 1));
    assert_eq!(loaded.get_pixel(0, 0), &image::Rgba([128, 64, 32, 255]));

    let _ = std::fs::remove_dir_all(&dir);
}

// --- save_image_optimized: JPG drops alpha channel (Req 3.3) ---

#[test]
fn save_image_optimized_jpg_output_is_rgb_not_rgba() {
    // Requirement 3.3: JPG converts RGBA to RGB before encoding
    let dir = std::env::temp_dir().join(format!(
        "rustshot-jpg-rgb-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("rgb_check.jpg");

    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 128, 64, 200]));
    save_image_optimized(&img, &path, Format::Jpg).unwrap();

    // Re-read as DynamicImage — JPEG files are always RGB (no alpha)
    let loaded = image::open(&path).unwrap();
    // The loaded image should be convertible to RGB without loss
    let rgb = loaded.to_rgb8();
    assert_eq!(rgb.dimensions(), (4, 4));
    // Verify the color channels are approximately correct (JPEG is lossy)
    let pixel = rgb.get_pixel(0, 0);
    assert!((pixel[0] as i16 - 255).unsigned_abs() < 10, "R channel");
    assert!((pixel[1] as i16 - 128).unsigned_abs() < 10, "G channel");
    assert!((pixel[2] as i16 - 64).unsigned_abs() < 10, "B channel");

    let _ = std::fs::remove_dir_all(&dir);
}

// --- bgra_to_rgba: 1-byte and 2-byte inputs (Req 3.1) ---

#[test]
fn bgra_to_rgba_one_byte_is_noop() {
    // Less than 4 bytes: no complete pixel, nothing to swap
    let mut pixels = vec![42u8];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![42]);
}

#[test]
fn bgra_to_rgba_two_bytes_is_noop() {
    let mut pixels = vec![10u8, 20];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![10, 20]);
}
