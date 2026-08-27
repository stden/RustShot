use std::path::PathBuf;

#[test]
fn bgra_to_rgba_multiple_pixels() {
    let mut pixels = vec![10, 20, 30, 255, 50, 60, 70, 128];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![30, 20, 10, 255, 70, 60, 50, 128]);
}

#[test]
fn bgra_to_rgba_trailing_bytes_ignored() {
    let mut pixels = vec![1, 2, 3, 4, 5, 6];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![3, 2, 1, 4, 5, 6]);
}

#[test]
fn resolve_screen_index_none_with_various_counts_always_zero() {
    for count in [1, 2, 5, 10, 100] {
        assert_eq!(
            rustshot::capture::resolve_screen_index(count, None).unwrap(),
            0,
            "None with {count} screens should return 0"
        );
    }
}

#[test]
fn resolve_screen_index_boundary_last_valid() {
    assert_eq!(
        rustshot::capture::resolve_screen_index(5, Some(4)).unwrap(),
        4
    );
}

#[test]
fn resolve_screen_index_zero_screens_is_error() {
    assert!(rustshot::capture::resolve_screen_index(0, None).is_err());
}

#[test]
fn resolve_screen_index_out_of_range_error_mentions_valid_range() {
    let err = rustshot::capture::resolve_screen_index(3, Some(5)).unwrap_err();
    assert!(err.contains('5'));
    assert!(err.contains("0-2"));
}

#[test]
fn default_screen_index_multiple_primaries_returns_first_primary() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false, true, true, false]),
        Some(1)
    );
}

#[test]
fn default_screen_index_no_primary_falls_back_to_first() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false, false, false]),
        Some(0)
    );
}

#[test]
fn default_screen_index_empty_returns_none() {
    assert_eq!(
        rustshot::capture::default_screen_index(Vec::<bool>::new()),
        None
    );
}

#[test]
fn resolve_output_path_explicit_extension_is_not_rewritten() {
    let path = PathBuf::from("custom.jpg");
    let result =
        rustshot::capture::resolve_output_path(Some(path.clone()), rustshot::capture::Format::Png);
    assert_eq!(result, path);
}

#[test]
fn resolve_output_path_in_dir_explicit_path_wins_over_dir() {
    let dir = PathBuf::from("/custom/dir");
    let explicit = PathBuf::from("/elsewhere/shot.png");
    let result = rustshot::capture::resolve_output_path_in_dir(
        Some(explicit.clone()),
        rustshot::capture::Format::Png,
        &dir,
    );
    assert_eq!(result, explicit);
}

#[test]
fn resolve_output_path_in_dir_none_generates_file_inside_dir() {
    let dir = PathBuf::from("/custom/dir");
    let result =
        rustshot::capture::resolve_output_path_in_dir(None, rustshot::capture::Format::Jpg, &dir);
    assert_eq!(result.parent(), Some(dir.as_path()));
    let name = result
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    assert!(name.starts_with("screenshot_"));
    assert_eq!(
        result.extension().and_then(|extension| extension.to_str()),
        Some("jpg")
    );
}

#[test]
fn format_to_image_format_all_variants() {
    use rustshot::capture::Format;

    for (format, expected) in [
        (Format::Png, image::ImageFormat::Png),
        (Format::Jpg, image::ImageFormat::Jpeg),
        (Format::Bmp, image::ImageFormat::Bmp),
    ] {
        assert_eq!(format.to_image_format(), expected);
    }
}
