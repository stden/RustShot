use std::path::PathBuf;

// ---------------------------------------------------------------------------
// capture module
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_swaps_channels_correctly() {
    // BGRA: B=1, G=2, R=3, A=4
    let mut pixels = vec![1, 2, 3, 4];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    // After swap: R=3, G=2, B=1, A=4
    assert_eq!(pixels, vec![3, 2, 1, 4]);
}

#[test]
fn bgra_to_rgba_empty_input() {
    let mut pixels: Vec<u8> = vec![];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert!(pixels.is_empty());
}

#[test]
fn bgra_to_rgba_multiple_pixels() {
    let mut pixels = vec![10, 20, 30, 255, 50, 60, 70, 128];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![30, 20, 10, 255, 70, 60, 50, 128]);
}

#[test]
fn bgra_to_rgba_double_swap_is_identity() {
    let original = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut pixels = original.clone();
    rustshot::capture::bgra_to_rgba(&mut pixels);
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, original);
}

#[test]
fn resolve_screen_index_none_returns_zero() {
    assert_eq!(rustshot::capture::resolve_screen_index(3, None).unwrap(), 0);
}

#[test]
fn resolve_screen_index_valid_returns_index() {
    assert_eq!(
        rustshot::capture::resolve_screen_index(3, Some(2)).unwrap(),
        2
    );
}

#[test]
fn resolve_screen_index_out_of_range_is_error() {
    assert!(rustshot::capture::resolve_screen_index(3, Some(5)).is_err());
}

#[test]
fn resolve_screen_index_zero_screens_is_error() {
    assert!(rustshot::capture::resolve_screen_index(0, None).is_err());
}

#[test]
fn resolve_screen_index_boundary_last_valid() {
    assert_eq!(
        rustshot::capture::resolve_screen_index(5, Some(4)).unwrap(),
        4
    );
}

#[test]
fn resolve_screen_index_boundary_first_invalid() {
    assert!(rustshot::capture::resolve_screen_index(5, Some(5)).is_err());
}

#[test]
fn resolve_screen_index_out_of_range_error_mentions_valid_range() {
    let err = rustshot::capture::resolve_screen_index(3, Some(5)).unwrap_err();
    assert!(err.contains("5"));
    assert!(err.contains("0-2"));
}

#[test]
fn default_screen_index_primary_first() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![true, false, false]),
        Some(0)
    );
}

#[test]
fn default_screen_index_primary_last() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false, false, true]),
        Some(2)
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
fn default_screen_index_single_primary() {
    assert_eq!(rustshot::capture::default_screen_index(vec![true]), Some(0));
}

#[test]
fn default_screen_index_single_non_primary_returns_zero() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false]),
        Some(0)
    );
}

#[test]
fn default_screen_index_multiple_primaries_returns_first_primary() {
    assert_eq!(
        rustshot::capture::default_screen_index(vec![false, true, true, false]),
        Some(1)
    );
}

#[test]
fn resolve_output_path_explicit_returns_given_path() {
    let path = PathBuf::from("/tmp/shot.png");
    let result =
        rustshot::capture::resolve_output_path(Some(path.clone()), rustshot::capture::Format::Png);
    assert_eq!(result, path);
}

#[test]
fn resolve_output_path_none_generates_path_with_extension() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Jpg);
    assert!(result.extension().and_then(|e| e.to_str()) == Some("jpg"));
}

#[test]
fn resolve_output_path_none_png_has_png_extension() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("png"));
}

#[test]
fn resolve_output_path_none_bmp_has_bmp_extension() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Bmp);
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("bmp"));
}

#[test]
fn resolve_output_path_none_is_under_screenshots_dir() {
    let result = rustshot::capture::resolve_output_path(None, rustshot::capture::Format::Png);
    assert_eq!(
        result.parent(),
        Some(rustshot::capture::get_screenshots_dir().as_path())
    );
}

#[test]
fn resolve_output_path_explicit_without_extension_is_preserved() {
    let path = PathBuf::from("custom_output");
    let result =
        rustshot::capture::resolve_output_path(Some(path.clone()), rustshot::capture::Format::Bmp);
    assert_eq!(result, path);
}

#[test]
fn resolve_output_path_explicit_extension_is_not_rewritten() {
    let path = PathBuf::from("custom.jpg");
    let result =
        rustshot::capture::resolve_output_path(Some(path.clone()), rustshot::capture::Format::Png);
    assert_eq!(result, path);
}

// ---------------------------------------------------------------------------
// capture::resolve_output_path_in_dir (consumed by benches/benchmarks/capture.rs)
// ---------------------------------------------------------------------------

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
    let name = result.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(
        name.starts_with("screenshot_"),
        "generated name should start with 'screenshot_': {name}"
    );
    assert_eq!(result.extension().and_then(|e| e.to_str()), Some("jpg"));
}

// ---------------------------------------------------------------------------
// capture::Format API
// ---------------------------------------------------------------------------

#[test]
fn format_extension_png() {
    assert_eq!(rustshot::capture::Format::Png.extension(), "png");
}

#[test]
fn format_extension_jpg() {
    assert_eq!(rustshot::capture::Format::Jpg.extension(), "jpg");
}

#[test]
fn format_extension_bmp() {
    assert_eq!(rustshot::capture::Format::Bmp.extension(), "bmp");
}

#[test]
fn format_default_is_png() {
    assert!(matches!(
        rustshot::capture::Format::default(),
        rustshot::capture::Format::Png
    ));
}

#[test]
fn format_to_image_format_png() {
    assert_eq!(
        rustshot::capture::Format::Png.to_image_format(),
        image::ImageFormat::Png
    );
}

#[test]
fn format_to_image_format_jpg() {
    assert_eq!(
        rustshot::capture::Format::Jpg.to_image_format(),
        image::ImageFormat::Jpeg
    );
}

#[test]
fn format_to_image_format_bmp() {
    assert_eq!(
        rustshot::capture::Format::Bmp.to_image_format(),
        image::ImageFormat::Bmp
    );
}

#[test]
fn format_to_image_format_all_distinct() {
    let png = rustshot::capture::Format::Png.to_image_format();
    let jpg = rustshot::capture::Format::Jpg.to_image_format();
    let bmp = rustshot::capture::Format::Bmp.to_image_format();
    assert_ne!(png, jpg);
    assert_ne!(png, bmp);
    assert_ne!(jpg, bmp);
}

#[test]
fn format_clone_produces_equal_variant() {
    let f = rustshot::capture::Format::Jpg;
    #[allow(clippy::clone_on_copy)]
    let g = f.clone();
    assert_eq!(f.extension(), g.extension());
}

#[test]
fn format_debug_is_non_empty() {
    assert!(!format!("{:?}", rustshot::capture::Format::Png).is_empty());
    assert!(!format!("{:?}", rustshot::capture::Format::Jpg).is_empty());
    assert!(!format!("{:?}", rustshot::capture::Format::Bmp).is_empty());
}

// ---------------------------------------------------------------------------
// capture::get_screenshots_dir
// ---------------------------------------------------------------------------

#[test]
fn get_screenshots_dir_ends_with_rustshot() {
    let dir = rustshot::capture::get_screenshots_dir();
    assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("RustShot"));
}

#[test]
fn get_screenshots_dir_consistent() {
    let a = rustshot::capture::get_screenshots_dir();
    let b = rustshot::capture::get_screenshots_dir();
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// capture::list_screens
// ---------------------------------------------------------------------------

#[test]
fn list_screens_returns_vec() {
    let screens = rustshot::capture::list_screens();
    // Should return a Vec (possibly empty in CI)
    let _ = screens.len();
}

#[test]
fn list_screens_entries_contain_screen_prefix() {
    let screens = rustshot::capture::list_screens();
    for entry in &screens {
        assert!(
            entry.starts_with("Screen "),
            "entry should start with 'Screen ': {entry}"
        );
    }
}

// ---------------------------------------------------------------------------
// capture::bgra_to_rgba edge cases
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_all_zeros() {
    let mut pixels = vec![0u8; 16];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![0u8; 16]);
}

#[test]
fn bgra_to_rgba_all_max() {
    let mut pixels = vec![255u8; 8];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![255u8; 8]);
}

#[test]
fn bgra_to_rgba_preserves_green_and_alpha() {
    // Only R and B should swap; G and A stay in place
    let mut pixels = vec![100, 200, 50, 128];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels[1], 200); // G unchanged
    assert_eq!(pixels[3], 128); // A unchanged
}

#[test]
fn bgra_to_rgba_trailing_bytes_ignored() {
    // chunks_exact_mut ignores trailing bytes not aligned to 4
    let mut pixels = vec![1, 2, 3, 4, 5, 6]; // 4+2 trailing bytes
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels[0], 3); // B→R swap happened on first chunk
    assert_eq!(pixels[2], 1);
    assert_eq!(pixels[4], 5); // trailing bytes untouched
    assert_eq!(pixels[5], 6);
}

#[test]
fn bgra_to_rgba_large_buffer_length_preserved() {
    let len = 4096;
    let mut pixels: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
    let original_len = pixels.len();
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels.len(), original_len);
}

// ---------------------------------------------------------------------------
// resolve_screen_index — additional edge cases
// ---------------------------------------------------------------------------

#[test]
fn resolve_screen_index_single_screen_index_zero_ok() {
    assert_eq!(
        rustshot::capture::resolve_screen_index(1, Some(0)).unwrap(),
        0
    );
}

#[test]
fn resolve_screen_index_single_screen_index_one_is_error() {
    let err = rustshot::capture::resolve_screen_index(1, Some(1)).unwrap_err();
    assert!(err.contains("out of range"));
    assert!(err.contains("0-0"));
}

#[test]
fn resolve_screen_index_usize_max_is_out_of_range() {
    assert!(rustshot::capture::resolve_screen_index(5, Some(usize::MAX)).is_err());
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

// ---------------------------------------------------------------------------
// bgra_to_rgba — pixel-level verification
// ---------------------------------------------------------------------------

#[test]
fn bgra_to_rgba_sequential_values() {
    // 3 pixels: [0,1,2,3, 4,5,6,7, 8,9,10,11]
    let mut pixels = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    // Each chunk: swap [0]↔[2]
    assert_eq!(pixels, vec![2, 1, 0, 3, 6, 5, 4, 7, 10, 9, 8, 11]);
}

#[test]
fn bgra_to_rgba_only_r_and_b_differ() {
    // When R==B, swap is invisible
    let mut pixels = vec![42, 100, 42, 200];
    rustshot::capture::bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![42, 100, 42, 200]);
}
