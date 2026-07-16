//! Edge-case tests: `bgra_to_rgba` corner inputs, pipeline ordering, and review-sweep coverage of format/path/screen helpers.

use crate::capture::*;
use std::cell::{Cell, RefCell};

// --- bgra_to_rgba edge cases ---

#[test]
fn bgra_to_rgba_trailing_bytes_ignored() {
    // chunks_exact_mut skips trailing bytes that don't form a full 4-byte chunk
    let mut pixels = vec![255, 0, 0, 255, 10, 20]; // 4 + 2 trailing
    bgra_to_rgba(&mut pixels);
    // First chunk swapped
    assert_eq!(pixels[0], 0);
    assert_eq!(pixels[2], 255);
    // Trailing bytes untouched
    assert_eq!(pixels[4], 10);
    assert_eq!(pixels[5], 20);
}

#[test]
fn bgra_to_rgba_three_bytes_is_noop() {
    // Less than 4 bytes: no chunks to process
    let mut pixels = vec![1, 2, 3];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![1, 2, 3]);
}

#[test]
fn bgra_to_rgba_large_image_all_swapped() {
    // 100 pixels, all with B=10, G=20, R=30, A=40
    let mut pixels = [10, 20, 30, 40].repeat(100);
    bgra_to_rgba(&mut pixels);
    for chunk in pixels.chunks(4) {
        assert_eq!(chunk[0], 30); // was R, now in B position
        assert_eq!(chunk[1], 20); // G unchanged
        assert_eq!(chunk[2], 10); // was B, now in R position
        assert_eq!(chunk[3], 40); // A unchanged
    }
}

// --- capture_screenshot_with additional edge cases ---

#[test]
fn capture_screenshot_with_multiple_screens_last_index() {
    let result = capture_screenshot_with(
        5,
        Some(4),
        || Ok("img"),
        Some(PathBuf::from("shot.png")),
        Format::Png,
        &PathBuf::from("."),
        |_| Ok(()),
        |_, _, _| Ok(()),
    )
    .unwrap();
    assert_eq!(result, PathBuf::from("shot.png"));
}

#[test]
fn capture_screenshot_with_capture_error_prevents_save() {
    let save_called = Cell::new(false);
    let _ = capture_screenshot_with(
        1,
        None,
        || Err::<(), _>("capture boom".to_string()),
        Some(PathBuf::from("out.png")),
        Format::Png,
        &PathBuf::from("."),
        |_| Ok(()),
        |_: &(), _, _| {
            save_called.set(true);
            Ok(())
        },
    );
    assert!(
        !save_called.get(),
        "save should not be called after capture error"
    );
}

// --- resolve_output_path with different formats ---

#[test]
fn resolve_output_path_two_different_formats_have_different_extensions() {
    let png = resolve_output_path(None, Format::Png);
    let jpg = resolve_output_path(None, Format::Jpg);
    assert_ne!(
        png.extension().and_then(|e| e.to_str()),
        jpg.extension().and_then(|e| e.to_str())
    );
}

// --- ensure_parent_directory error path ---

#[test]
fn ensure_parent_directory_with_only_filename_is_ok() {
    // "file.png" has parent "" which is empty, so no mkdir
    assert!(ensure_parent_directory(Path::new("file.png")).is_ok());
}

#[test]
fn ensure_parent_directory_root_path_is_ok() {
    // "/file.png" parent is "/" which exists
    assert!(ensure_parent_directory(Path::new("/file.png")).is_ok());
}

// --- Format trait coverage ---

#[test]
fn format_debug_contains_variant_name() {
    assert!(format!("{:?}", Format::Png).contains("Png"));
    assert!(format!("{:?}", Format::Jpg).contains("Jpg"));
    assert!(format!("{:?}", Format::Bmp).contains("Bmp"));
}

// --- default_screen_index additional tests ---

#[test]
fn default_screen_index_single_primary_returns_zero() {
    assert_eq!(default_screen_index([true]), Some(0));
}

#[test]
fn default_screen_index_single_non_primary_returns_zero() {
    assert_eq!(default_screen_index([false]), Some(0));
}

#[test]
fn default_screen_index_multiple_primaries_returns_first() {
    // Rare but possible: two screens both marked primary
    assert_eq!(default_screen_index([true, true, false]), Some(0));
}

#[test]
fn default_screen_index_primary_in_middle_of_five() {
    assert_eq!(
        default_screen_index([false, false, true, false, false]),
        Some(2)
    );
}

// --- bgra_to_rgba additional tests ---

#[test]
fn bgra_to_rgba_single_pixel_specific_values() {
    // BGRA [100, 150, 200, 128] → RGBA [200, 150, 100, 128]
    let mut pixels = vec![100, 150, 200, 128];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![200, 150, 100, 128]);
}

#[test]
fn bgra_to_rgba_two_pixels() {
    let mut pixels = vec![
        100, 200, 50, 255, // pixel 0: B=100, G=200, R=50, A=255
        10, 20, 30, 128, // pixel 1: B=10, G=20, R=30, A=128
    ];
    bgra_to_rgba(&mut pixels);
    // pixel 0: R=50, G=200, B=100, A=255
    assert_eq!(&pixels[0..4], &[50, 200, 100, 255]);
    // pixel 1: R=30, G=20, B=10, A=128
    assert_eq!(&pixels[4..8], &[30, 20, 10, 128]);
}

// --- capture_screenshot_with: ensure closure order ---

#[test]
fn capture_screenshot_with_calls_closures_in_order() {
    let order = RefCell::new(Vec::<&str>::new());

    let _ = capture_screenshot_with(
        1,
        None,
        || {
            order.borrow_mut().push("capture");
            Ok(())
        },
        Some(PathBuf::from("test.png")),
        Format::Png,
        &PathBuf::from("."),
        |_| {
            order.borrow_mut().push("ensure_dir");
            Ok(())
        },
        |_, _, _| {
            order.borrow_mut().push("save");
            Ok(())
        },
    );

    assert_eq!(*order.borrow(), vec!["capture", "ensure_dir", "save"]);
}

// --- resolve_output_path consistency ---

#[test]
fn resolve_output_path_default_filenames_all_start_with_screenshot() {
    for fmt in [Format::Png, Format::Jpg, Format::Bmp] {
        let path = resolve_output_path(None, fmt);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(
            name.starts_with("screenshot_"),
            "expected 'screenshot_' prefix for {fmt:?}, got: {name}"
        );
    }
}

#[test]
fn resolve_output_path_explicit_with_no_extension() {
    let explicit = PathBuf::from("no_ext");
    let result = resolve_output_path(Some(explicit.clone()), Format::Png);
    assert_eq!(result, explicit);
    assert_eq!(result.extension(), None);
}

// --- ensure_parent_directory with temp dir ---

#[test]
fn ensure_parent_directory_succeeds_for_existing_temp_dir() {
    let path = std::env::temp_dir().join("test_file.png");
    assert!(ensure_parent_directory(&path).is_ok());
}

// --- resolve_screen_index property tests ---

#[test]
fn resolve_screen_index_always_returns_requested_when_in_range() {
    for count in 1..=10usize {
        for idx in 0..count {
            assert_eq!(
                resolve_screen_index(count, Some(idx)).unwrap(),
                idx,
                "count={count}, idx={idx}"
            );
        }
    }
}

#[test]
fn resolve_screen_index_always_errors_when_out_of_range() {
    for count in 1..=5usize {
        for idx in count..count + 3 {
            assert!(
                resolve_screen_index(count, Some(idx)).is_err(),
                "count={count}, idx={idx} should be out of range"
            );
        }
    }
}

// =========================================================================
// Format — additional coverage
// =========================================================================

#[test]
fn format_default_variant_extension_is_png() {
    let f = Format::default();
    assert_eq!(f.extension(), "png");
}

#[test]
fn format_to_image_format_png_roundtrip() {
    assert_eq!(Format::Png.to_image_format(), ImageFormat::Png);
}

#[test]
fn format_to_image_format_jpg_is_jpeg() {
    assert_eq!(Format::Jpg.to_image_format(), ImageFormat::Jpeg);
}

#[test]
fn format_to_image_format_bmp() {
    assert_eq!(Format::Bmp.to_image_format(), ImageFormat::Bmp);
}

#[test]
fn format_clone_preserves_variant() {
    let f = Format::Jpg;
    let f2 = f;
    assert_eq!(f.extension(), f2.extension());
}

#[test]
fn format_debug_is_non_empty() {
    assert!(!format!("{:?}", Format::Png).is_empty());
    assert!(!format!("{:?}", Format::Jpg).is_empty());
    assert!(!format!("{:?}", Format::Bmp).is_empty());
}

// =========================================================================
// ensure_parent_directory — additional coverage
// =========================================================================

#[test]
fn ensure_parent_directory_root_path_succeeds() {
    // Path with no parent (just filename) should succeed
    assert!(ensure_parent_directory(std::path::Path::new("file.png")).is_ok());
}

#[test]
fn ensure_parent_directory_nested_tmp_path() {
    let path = std::path::Path::new("/tmp/rustshot_test_ensure/nested/deep/file.png");
    let result = ensure_parent_directory(path);
    assert!(result.is_ok());
    // Cleanup
    let _ = std::fs::remove_dir_all("/tmp/rustshot_test_ensure");
}

// =========================================================================
// resolve_output_path_in_dir — additional coverage
// =========================================================================

#[test]
fn resolve_output_path_in_dir_explicit_path_ignores_dir() {
    let dir = PathBuf::from("/should/be/ignored");
    let explicit = PathBuf::from("/custom/shot.png");
    let result = resolve_output_path_in_dir(Some(explicit.clone()), Format::Png, &dir);
    assert_eq!(result, explicit);
}

#[test]
fn resolve_output_path_in_dir_none_uses_dir() {
    let dir = PathBuf::from("/my/screenshots");
    let result = resolve_output_path_in_dir(None, Format::Png, &dir);
    assert_eq!(result.parent().unwrap(), dir.as_path());
}

#[test]
fn resolve_output_path_in_dir_jpg_extension() {
    let dir = PathBuf::from("/tmp");
    let result = resolve_output_path_in_dir(None, Format::Jpg, &dir);
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("jpg"));
}

#[test]
fn resolve_output_path_in_dir_bmp_extension() {
    let dir = PathBuf::from("/tmp");
    let result = resolve_output_path_in_dir(None, Format::Bmp, &dir);
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("bmp"));
}

#[test]
fn resolve_output_path_in_dir_png_extension() {
    let dir = PathBuf::from("/tmp");
    let result = resolve_output_path_in_dir(None, Format::Png, &dir);
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("png"));
}

#[test]
fn resolve_output_path_in_dir_filename_starts_with_screenshot() {
    let dir = PathBuf::from("/tmp");
    let result = resolve_output_path_in_dir(None, Format::Png, &dir);
    let stem = result.file_stem().and_then(|n| n.to_str()).unwrap();
    assert!(stem.starts_with("screenshot_"));
}

#[test]
fn resolve_output_path_in_dir_timestamp_is_25_chars() {
    let dir = PathBuf::from("/tmp");
    let result = resolve_output_path_in_dir(None, Format::Png, &dir);
    let stem = result.file_stem().and_then(|n| n.to_str()).unwrap();
    let ts = &stem["screenshot_".len()..];
    assert_eq!(ts.len(), 25); // YYYYMMDD_HHMMSS_NNNNNNNNN
}

// =========================================================================
// default_screen_index — additional coverage
// =========================================================================

#[test]
fn default_screen_index_all_primary_returns_first() {
    assert_eq!(default_screen_index(vec![true, true, true]), Some(0));
}

#[test]
fn default_screen_index_single_non_primary_returns_some_0() {
    assert_eq!(default_screen_index(vec![false]), Some(0));
}

#[test]
fn default_screen_index_empty_returns_none() {
    assert_eq!(default_screen_index(Vec::<bool>::new()), None);
}

#[test]
fn default_screen_index_primary_at_index_2() {
    assert_eq!(
        default_screen_index(vec![false, false, true, false]),
        Some(2)
    );
}

// =========================================================================
// bgra_to_rgba — additional coverage
// =========================================================================

#[test]
fn bgra_to_rgba_single_pixel() {
    let mut data = vec![30u8, 20, 10, 255]; // BGRA
    bgra_to_rgba(&mut data);
    assert_eq!(data, vec![10, 20, 30, 255]); // RGBA
}

#[test]
fn bgra_to_rgba_two_swaps_restore_original() {
    let original = vec![100u8, 150, 200, 255, 10, 20, 30, 128];
    let mut data = original.clone();
    bgra_to_rgba(&mut data);
    bgra_to_rgba(&mut data);
    assert_eq!(data, original);
}

#[test]
fn bgra_to_rgba_preserves_alpha() {
    let mut data = vec![0u8, 0, 0, 42];
    bgra_to_rgba(&mut data);
    assert_eq!(data[3], 42);
}

#[test]
fn bgra_to_rgba_empty_buffer_is_noop() {
    let mut data: Vec<u8> = vec![];
    bgra_to_rgba(&mut data);
    assert!(data.is_empty());
}

#[test]
fn bgra_to_rgba_preserves_green() {
    let mut data = vec![0u8, 128, 0, 255];
    bgra_to_rgba(&mut data);
    assert_eq!(data[1], 128); // green unchanged
}
