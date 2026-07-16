//! Tests for the `benchmark_*` shims, `save_image_optimized` file output, and screen/path helper coverage.

use crate::capture::*;

// -------------------------------------------------------------------------
// bgra_to_rgba edge cases
// -------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_single_pixel_swaps_r_and_b() {
    // BGRA [10, 20, 30, 255] -> RGBA [30, 20, 10, 255]
    let mut pixels = vec![10u8, 20, 30, 255];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![30, 20, 10, 255]);
}

#[test]
fn bgra_to_rgba_preserves_alpha_channel() {
    let mut pixels = vec![0u8, 0, 0, 128];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels[3], 128);
}

#[test]
fn bgra_to_rgba_white_pixel_unchanged() {
    // White: [255,255,255,255] - swapping R and B when both are 255 is identity
    let mut pixels = vec![255u8, 255, 255, 255];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![255, 255, 255, 255]);
}

#[test]
fn bgra_to_rgba_empty_slice_is_noop() {
    let mut pixels: Vec<u8> = vec![];
    bgra_to_rgba(&mut pixels);
    assert!(pixels.is_empty());
}

#[test]
fn bgra_to_rgba_two_pixels_verify_both_swapped() {
    let mut pixels = vec![
        10, 20, 30, 255, // pixel 1: B=10, G=20, R=30, A=255
        40, 50, 60, 128, // pixel 2: B=40, G=50, R=60, A=128
    ];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels[0], 30); // R was at index 2
    assert_eq!(pixels[2], 10); // B was at index 0
    assert_eq!(pixels[4], 60); // R was at index 6
    assert_eq!(pixels[6], 40); // B was at index 4
}

// -------------------------------------------------------------------------
// benchmark_screen_count_from_result edge cases
// -------------------------------------------------------------------------

#[test]
fn benchmark_screen_count_from_result_error_passthrough() {
    let result: Result<Vec<i32>, String> = Err("enumeration failed".to_string());
    let count = benchmark_screen_count_from_result(result);
    assert_eq!(count, Err("enumeration failed".to_string()));
}

#[test]
fn benchmark_screen_count_from_result_empty_vec_is_error() {
    let result: Result<Vec<i32>, String> = Ok(vec![]);
    let count = benchmark_screen_count_from_result(result);
    assert_eq!(count, Err("No screens detected".to_string()));
}

#[test]
fn benchmark_screen_count_from_result_three_screens() {
    let result: Result<Vec<i32>, String> = Ok(vec![1, 2, 3]);
    assert_eq!(benchmark_screen_count_from_result(result), Ok(3));
}

// -------------------------------------------------------------------------
// benchmark_first_screen_from_result edge cases
// -------------------------------------------------------------------------

#[test]
fn benchmark_first_screen_from_result_returns_first_element() {
    let result: Result<Vec<&str>, String> = Ok(vec!["primary", "secondary"]);
    assert_eq!(benchmark_first_screen_from_result(result), Ok("primary"));
}

#[test]
fn benchmark_first_screen_from_result_empty_vec_is_error() {
    let result: Result<Vec<i32>, String> = Ok(vec![]);
    assert_eq!(
        benchmark_first_screen_from_result(result),
        Err("No screens detected".to_string())
    );
}

// -------------------------------------------------------------------------
// save_image_optimized to real files
// -------------------------------------------------------------------------

#[test]
fn save_image_optimized_png_creates_file() {
    let dir = std::env::temp_dir().join("rustshot_save_png_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.png");
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    save_image_optimized(&img, &path, Format::Png).unwrap();
    assert!(path.exists());
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_jpg_creates_file() {
    let dir = std::env::temp_dir().join("rustshot_save_jpg_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.jpg");
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 255, 0, 255]));
    save_image_optimized(&img, &path, Format::Jpg).unwrap();
    assert!(path.exists());
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_bmp_creates_file() {
    let dir = std::env::temp_dir().join("rustshot_save_bmp_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.bmp");
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 255, 255]));
    save_image_optimized(&img, &path, Format::Bmp).unwrap();
    assert!(path.exists());
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------
// benchmark_rgb_image_from_rgba
// -------------------------------------------------------------------------

#[test]
fn benchmark_rgb_image_from_rgba_drops_alpha() {
    let rgba = image::RgbaImage::from_pixel(2, 2, image::Rgba([100, 150, 200, 128]));
    let rgb = benchmark_rgb_image_from_rgba(&rgba);
    assert_eq!(rgb.width(), 2);
    assert_eq!(rgb.height(), 2);
    // RGB image has 3 bytes per pixel
    assert_eq!(rgb.as_raw().len(), 2 * 2 * 3);
    // First pixel should be [100, 150, 200]
    assert_eq!(rgb.as_raw()[0], 100);
    assert_eq!(rgb.as_raw()[1], 150);
    assert_eq!(rgb.as_raw()[2], 200);
}

#[test]
fn benchmark_rgb_image_from_rgba_preserves_dimensions() {
    let rgba = image::RgbaImage::new(10, 20);
    let rgb = benchmark_rgb_image_from_rgba(&rgba);
    assert_eq!(rgb.width(), 10);
    assert_eq!(rgb.height(), 20);
}

// -------------------------------------------------------------------------
// default_screen_index edge cases
// -------------------------------------------------------------------------

#[test]
fn default_screen_index_single_primary() {
    assert_eq!(default_screen_index([true]), Some(0));
}

#[test]
fn default_screen_index_single_non_primary() {
    assert_eq!(default_screen_index([false]), Some(0));
}

#[test]
fn default_screen_index_first_is_primary() {
    assert_eq!(default_screen_index([true, false, false]), Some(0));
}

// -------------------------------------------------------------------------
// Format edge cases
// -------------------------------------------------------------------------

// -------------------------------------------------------------------------
// resolve_screen_index additional edge cases
// -------------------------------------------------------------------------

#[test]
fn resolve_screen_index_zero_screens_none_is_err() {
    assert!(resolve_screen_index(0, None).is_err());
}

#[test]
fn resolve_screen_index_zero_screens_some_is_err() {
    assert!(resolve_screen_index(0, Some(0)).is_err());
}

#[test]
fn resolve_screen_index_zero_screens_mentions_no_screens() {
    let err = resolve_screen_index(0, None).unwrap_err();
    assert!(err.contains("No screens"));
}

#[test]
fn resolve_screen_index_last_valid() {
    assert_eq!(resolve_screen_index(3, Some(2)), Ok(2));
}

#[test]
fn resolve_screen_index_first_invalid() {
    assert!(resolve_screen_index(3, Some(3)).is_err());
}

#[test]
fn resolve_screen_index_error_shows_valid_range() {
    let err = resolve_screen_index(3, Some(5)).unwrap_err();
    assert!(
        err.contains("0-2"),
        "error should mention valid range: {err}"
    );
}

#[test]
fn resolve_screen_index_one_screen_default_is_zero() {
    assert_eq!(resolve_screen_index(1, None), Ok(0));
}

#[test]
fn resolve_screen_index_one_screen_explicit_zero_ok() {
    assert_eq!(resolve_screen_index(1, Some(0)), Ok(0));
}

#[test]
fn resolve_screen_index_one_screen_explicit_one_fails() {
    assert!(resolve_screen_index(1, Some(1)).is_err());
}

// -------------------------------------------------------------------------
// resolve_output_path_in_dir tests
// -------------------------------------------------------------------------

#[test]
fn resolve_output_path_in_dir_explicit_path_returned() {
    let p = resolve_output_path_in_dir(
        Some(PathBuf::from("/tmp/my.png")),
        Format::Png,
        Path::new("/screenshots"),
    );
    assert_eq!(p, PathBuf::from("/tmp/my.png"));
}

#[test]
fn resolve_output_path_in_dir_generated_has_correct_extension() {
    let p = resolve_output_path_in_dir(None, Format::Jpg, Path::new("/screenshots"));
    assert!(
        p.extension().and_then(|e| e.to_str()) == Some("jpg"),
        "expected .jpg extension, got: {p:?}"
    );
}

#[test]
fn resolve_output_path_in_dir_generated_starts_with_dir() {
    let dir = Path::new("/my/screenshots");
    let p = resolve_output_path_in_dir(None, Format::Png, dir);
    assert!(
        p.starts_with(dir),
        "path should start with {dir:?}, got: {p:?}"
    );
}

#[test]
fn resolve_output_path_in_dir_generated_contains_screenshot() {
    let p = resolve_output_path_in_dir(None, Format::Bmp, Path::new("/tmp"));
    let name = p.file_name().unwrap().to_str().unwrap();
    assert!(
        name.starts_with("screenshot_"),
        "name should start with screenshot_, got: {name}"
    );
}

#[test]
fn resolve_output_path_in_dir_each_format_extension() {
    for (fmt, ext) in [
        (Format::Png, "png"),
        (Format::Jpg, "jpg"),
        (Format::Bmp, "bmp"),
    ] {
        let p = resolve_output_path_in_dir(None, fmt, Path::new("/tmp"));
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some(ext));
    }
}

// -------------------------------------------------------------------------
// ensure_parent_directory tests
// -------------------------------------------------------------------------

#[test]
fn ensure_parent_directory_creates_missing_dir() {
    let dir = std::env::temp_dir().join("rustshot_test_ensure_parent");
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("subdir").join("file.txt");
    ensure_parent_directory(&file).unwrap();
    assert!(file.parent().unwrap().exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ensure_parent_directory_existing_dir_ok() {
    let dir = std::env::temp_dir();
    let file = dir.join("existing_file.txt");
    assert!(ensure_parent_directory(&file).is_ok());
}

#[test]
fn ensure_parent_directory_root_path_ok() {
    // A path like "file.txt" with no parent should succeed
    assert!(ensure_parent_directory(Path::new("file.txt")).is_ok());
}

// -------------------------------------------------------------------------
// default_screen_index additional tests
// -------------------------------------------------------------------------

#[test]
fn default_screen_index_empty() {
    let empty: Vec<bool> = vec![];
    assert_eq!(default_screen_index(empty), None);
}

#[test]
fn default_screen_index_primary_in_middle() {
    assert_eq!(default_screen_index([false, true, false]), Some(1));
}

#[test]
fn default_screen_index_primary_last() {
    assert_eq!(default_screen_index([false, false, true]), Some(2));
}

#[test]
fn default_screen_index_two_primaries_picks_earlier() {
    assert_eq!(default_screen_index([false, true, true]), Some(1));
}

#[test]
fn default_screen_index_no_primary_returns_first() {
    assert_eq!(default_screen_index([false, false, false]), Some(0));
}

// -------------------------------------------------------------------------
// save_image_optimized additional tests
// -------------------------------------------------------------------------

#[test]
fn save_image_optimized_1x1_pixel_all_formats() {
    for (fmt, ext) in [
        (Format::Png, "png"),
        (Format::Jpg, "jpg"),
        (Format::Bmp, "bmp"),
    ] {
        let dir = std::env::temp_dir().join(format!("rustshot_test_1x1_{ext}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("pixel.{ext}"));

        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 64, 32, 255]));
        save_image_optimized(&img, &path, fmt).unwrap();
        assert!(path.exists(), "file should exist for format {ext}");
        assert!(std::fs::metadata(&path).unwrap().len() > 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn save_image_optimized_transparent_pixel_png() {
    let dir = std::env::temp_dir().join("rustshot_test_transparent");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("transparent.png");

    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 0]));
    save_image_optimized(&img, &path, Format::Png).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_image_optimized_invalid_path_returns_error() {
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
    let result = save_image_optimized(&img, Path::new("/nonexistent/dir/file.png"), Format::Png);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to create file"));
}

// -------------------------------------------------------------------------
// save_image_optimized — JPEG with transparent pixel (alpha channel drop)
// -------------------------------------------------------------------------

#[test]
fn save_image_optimized_jpg_drops_alpha_channel() {
    let dir = std::env::temp_dir().join(format!(
        "rustshot-jpg-alpha-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("alpha.jpg");
    // Image with 50% transparent red pixel
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 128]));
    save_image_optimized(&img, &path, Format::Jpg).unwrap();
    // JPEG file should exist and be non-empty
    let size = std::fs::metadata(&path).unwrap().len();
    assert!(size > 0, "JPEG file should be non-empty");
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------
// default_screen_index — multiple primaries picks first primary
// -------------------------------------------------------------------------

#[test]
fn default_screen_index_multiple_primaries_picks_first() {
    let index = default_screen_index([false, true, true, false]).unwrap();
    assert_eq!(index, 1, "should pick the first primary screen");
}

#[test]
fn default_screen_index_single_screen_primary_returns_zero() {
    let index = default_screen_index([true]).unwrap();
    assert_eq!(index, 0);
}

#[test]
fn default_screen_index_single_screen_not_primary_returns_zero() {
    // Fallback to first screen when no primary flag
    let index = default_screen_index([false]).unwrap();
    assert_eq!(index, 0);
}
