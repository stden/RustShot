#[test]
fn benchmark_screen_count_from_result_returns_count_for_non_empty_screens() {
    let count = rustshot::capture::benchmark_screen_count_from_result::<u8>(Ok(vec![1, 2, 3]))
        .expect("non-empty screen list should produce a count");
    assert_eq!(count, 3);
}

#[test]
fn benchmark_first_screen_from_result_returns_first_screen() {
    let first = rustshot::capture::benchmark_first_screen_from_result(Ok(vec![10, 20, 30]))
        .expect("first screen should be returned");
    assert_eq!(first, 10);
}

#[test]
fn benchmark_rgb_image_from_rgba_produces_jpeg_compatible_image() {
    let rgba = image::RgbaImage::from_pixel(2, 2, image::Rgba([100, 150, 200, 255]));
    let rgb = rustshot::capture::benchmark_rgb_image_from_rgba(&rgba);

    let mut encoded = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 75);
    rgb.write_with_encoder(encoder)
        .expect("RGB benchmark images should encode as JPEG");

    assert!(encoded.len() > 2);
    assert_eq!(&encoded[..2], &[0xFF, 0xD8]);
}

// ---------------------------------------------------------------------------
// benchmark helper exports
// ---------------------------------------------------------------------------

#[test]
fn benchmark_ensure_parent_directory_creates_nested_parent() {
    let root = std::env::temp_dir().join(format!(
        "rustshot_benchmark_ensure_parent_{}",
        rustshot::settings::generate_timestamp()
    ));
    let path = root.join("nested/deeper/shot.png");

    rustshot::capture::benchmark_ensure_parent_directory(&path).unwrap();

    assert!(path.parent().unwrap().is_dir());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn benchmark_save_image_optimized_writes_png_file() {
    let root = std::env::temp_dir().join(format!(
        "rustshot_benchmark_save_image_{}",
        rustshot::settings::generate_timestamp()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("shot.png");
    let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));

    rustshot::capture::benchmark_save_image_optimized(
        &image,
        &path,
        rustshot::capture::Format::Png,
    )
    .unwrap();

    assert!(path.is_file());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn benchmark_build_default_icon_pixels_returns_full_rgba_buffer() {
    let pixels = rustshot::tray::benchmark_build_default_icon_pixels();
    assert_eq!(pixels.len(), 32 * 32 * 4);
}
