//! Tests for the `benchmark_*` shims, `save_image_optimized` file output, and screen/path helper coverage.

use crate::capture::*;

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

// -------------------------------------------------------------------------
// resolve_output_path_in_dir tests
// -------------------------------------------------------------------------

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
// save_image_optimized additional tests
// -------------------------------------------------------------------------

#[test]
fn save_image_optimized_transparent_pixel_png() {
    let dir = crate::test_support::TestDir::new("rustshot_test_transparent");
    let path = dir.file("transparent.png");

    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 0]));
    save_image_optimized(&img, &path, Format::Png).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);
}
