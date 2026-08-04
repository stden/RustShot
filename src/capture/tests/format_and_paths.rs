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
fn resolve_screen_index_usize_max_is_out_of_range() {
    let err = resolve_screen_index(5, Some(usize::MAX)).unwrap_err();
    assert!(err.contains("out of range"));
}
