//! Tests for `Format`, screen-index resolution, output-path resolution, and directory helpers.

use crate::capture::*;

#[test]
fn format_extension_is_correct() {
    assert_eq!(Format::Png.extension(), "png");
    assert_eq!(Format::Jpg.extension(), "jpg");
    assert_eq!(Format::Bmp.extension(), "bmp");
}

#[test]
fn format_to_image_format_is_correct() {
    assert_eq!(Format::Png.to_image_format(), ImageFormat::Png);
    assert_eq!(Format::Jpg.to_image_format(), ImageFormat::Jpeg);
    assert_eq!(Format::Bmp.to_image_format(), ImageFormat::Bmp);
}

#[test]
fn format_default_is_png() {
    assert!(matches!(Format::default(), Format::Png));
}

#[test]
fn format_clone_and_copy_work() {
    let f = Format::Jpg;
    let g = f; // Copy
    #[allow(clippy::clone_on_copy)]
    let h = f.clone(); // Clone
    assert!(matches!(g, Format::Jpg));
    assert!(matches!(h, Format::Jpg));
}

#[test]
fn screen_index_selection_handles_default_and_bounds() {
    assert_eq!(resolve_screen_index(2, None).unwrap(), 0);
    assert_eq!(resolve_screen_index(2, Some(1)).unwrap(), 1);
    assert!(resolve_screen_index(2, Some(3)).is_err());
    assert_eq!(
        resolve_screen_index(2, Some(3)).unwrap_err(),
        "Screen index 3 is out of range (0-1)"
    );
    assert!(resolve_screen_index(0, None).is_err());
    assert_eq!(
        resolve_screen_index(0, None).unwrap_err(),
        "No screens detected"
    );
}

#[test]
fn default_screen_index_prefers_primary_screen() {
    let index = default_screen_index([false, true, false]).unwrap();
    assert_eq!(index, 1);
}

#[test]
fn default_screen_index_falls_back_to_first_screen() {
    let index = default_screen_index([false, false, false]).unwrap();
    assert_eq!(index, 0);
}

#[test]
fn default_screen_index_returns_none_for_empty_input() {
    assert_eq!(default_screen_index(Vec::<bool>::new()), None);
}

#[test]
fn resolve_screen_index_one_screen_index_one_is_err() {
    // screen_count=1, requested=Some(1): 1 >= 1, out of range
    assert!(resolve_screen_index(1, Some(1)).is_err());
    assert_eq!(
        resolve_screen_index(1, Some(1)).unwrap_err(),
        "Screen index 1 is out of range (0-0)"
    );
}

#[test]
fn resolve_screen_index_none_always_returns_zero() {
    // With any positive screen count and None, always returns 0
    assert_eq!(resolve_screen_index(1, None).unwrap(), 0);
    assert_eq!(resolve_screen_index(10, None).unwrap(), 0);
    assert_eq!(resolve_screen_index(100, None).unwrap(), 0);
}

#[test]
fn resolve_screen_index_error_message_contains_range() {
    let err = resolve_screen_index(3, Some(5)).unwrap_err();
    assert!(err.contains("5"));
    assert!(err.contains("0-2"));
}

#[test]
fn output_path_uses_default_format_extension() {
    let default_path = resolve_output_path(None, Format::Bmp);
    assert_eq!(default_path.parent().unwrap(), &get_screenshots_dir());
    assert_eq!(
        default_path.extension().and_then(|ext| ext.to_str()),
        Some("bmp")
    );
    assert!(default_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .starts_with("screenshot_"));

    let explicit = resolve_output_path(Some(PathBuf::from("custom.png")), Format::Png);
    assert_eq!(explicit, PathBuf::from("custom.png"));
}

#[test]
fn resolve_output_path_jpg_has_jpg_extension() {
    let path = resolve_output_path(None, Format::Jpg);
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("jpg"));
}

#[test]
fn resolve_output_path_explicit_overrides_format() {
    // When an explicit path is given, it is returned verbatim regardless of format
    let explicit = PathBuf::from("my_shot.bmp");
    let result = resolve_output_path(Some(explicit.clone()), Format::Png);
    assert_eq!(result, explicit);
}

#[test]
fn resolve_output_path_in_dir_uses_provided_directory() {
    let dir = PathBuf::from("/tmp/custom-shots");
    let path = resolve_output_path_in_dir(None, Format::Bmp, &dir);
    assert_eq!(path.parent(), Some(dir.as_path()));
    assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("bmp"));
}

#[test]
fn resolve_output_path_in_dir_explicit_path_ignores_directory_and_format() {
    let dir = PathBuf::from("/tmp/custom-shots");
    let explicit = PathBuf::from("nested/final-name.jpg");
    let path = resolve_output_path_in_dir(Some(explicit.clone()), Format::Bmp, &dir);
    assert_eq!(path, explicit);
}

#[test]
fn resolve_output_path_in_dir_default_filename_starts_with_screenshot() {
    let dir = PathBuf::from("custom-shots");
    let path = resolve_output_path_in_dir(None, Format::Png, &dir);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with("screenshot_"));
}

#[test]
fn ensure_parent_directory_handles_empty_parent() {
    let path = PathBuf::from("screenshot.png");
    assert!(ensure_parent_directory(&path).is_ok());
}

#[test]
fn ensure_parent_directory_creates_deeply_nested_path() {
    // TestDir only creates the root; the nested a/b/c/d parents stay missing
    // so ensure_parent_directory still exercises deep creation.
    let temp_dir = crate::test_support::TestDir::new("rustshot-deep-test");
    let deep_path = temp_dir
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("d")
        .join("shot.png");
    ensure_parent_directory(&deep_path).expect("deep parent dirs should be created");
    assert!(deep_path.parent().unwrap().exists());
}

#[test]
fn screenshots_dir_is_named_rustshot() {
    let dir = get_screenshots_dir();
    assert_eq!(
        dir.file_name().and_then(|name| name.to_str()),
        Some("RustShot")
    );
}

#[test]
fn get_screenshots_dir_is_under_pictures_or_dot() {
    let dir = get_screenshots_dir();
    // The parent must be either the system pictures dir or "." (current dir)
    let parent = dir.parent().expect("screenshots dir should have a parent");
    let expected = dirs::picture_dir().unwrap_or_else(|| PathBuf::from("."));
    assert_eq!(parent, expected.as_path());
}

#[test]
fn get_screenshots_dir_is_absolute_or_relative_dot() {
    let dir = get_screenshots_dir();
    // Either absolute path (from pictures_dir) or starts with "." (fallback)
    let display = dir.display().to_string();
    let is_absolute = dir.is_absolute();
    let is_relative_dot = display.starts_with('.');
    assert!(
        is_absolute || is_relative_dot,
        "dir should be absolute or relative-to-dot: {}",
        display
    );
}

#[test]
fn resolve_screen_index_five_screens_index_five_is_err() {
    // index == screen_count is out of range
    let err = resolve_screen_index(5, Some(5)).unwrap_err();
    assert!(err.contains("5"), "error should mention index: {err}");
    assert!(err.contains("0-4"), "error should show valid range: {err}");
}

#[test]
fn resolve_screen_index_usize_max_is_out_of_range() {
    let err = resolve_screen_index(5, Some(usize::MAX)).unwrap_err();
    assert!(err.contains("out of range"));
}

#[test]
fn resolve_screen_index_called_twice_same_result() {
    assert_eq!(resolve_screen_index(3, Some(0)).unwrap(), 0);
    assert_eq!(resolve_screen_index(3, Some(0)).unwrap(), 0);
}

#[test]
fn resolve_output_path_two_calls_same_format_same_prefix() {
    let p1 = resolve_output_path(None, Format::Png);
    let p2 = resolve_output_path(None, Format::Png);
    let name1 = p1.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let name2 = p2.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name1.starts_with("screenshot_"));
    assert!(name2.starts_with("screenshot_"));
    assert!(name1.ends_with(".png"));
    assert!(name2.ends_with(".png"));
}

#[test]
fn get_screenshots_dir_consistent_across_calls() {
    // Two successive calls must return the same path
    let first = get_screenshots_dir();
    let second = get_screenshots_dir();
    assert_eq!(first, second);
}

#[test]
fn resolve_screen_index_large_count_last_index() {
    assert_eq!(resolve_screen_index(1000, Some(999)).unwrap(), 999);
}

#[test]
fn resolve_output_path_filename_contains_digits() {
    // Default filename contains the timestamp, which consists of digits and underscores
    let path = resolve_output_path(None, Format::Png);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(
        name.chars().any(|c| c.is_ascii_digit()),
        "filename should contain timestamp digits: {name}"
    );
}
