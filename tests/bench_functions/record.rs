#[test]
fn frame_duration_hns_zero_returns_zero() {
    assert_eq!(rustshot::record::frame_duration_hns(0), 0);
}

#[test]
fn rgba_to_bgra_bottom_up_swaps_channels_and_rows() {
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2).unwrap();
    assert_eq!(
        result,
        vec![11, 10, 9, 12, 15, 14, 13, 16, 3, 2, 1, 4, 7, 6, 5, 8]
    );
}

#[test]
fn rgba_to_bgra_bottom_up_double_conversion_is_identity() {
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let once = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2).unwrap();
    let twice = rustshot::record::rgba_to_bgra_bottom_up(&once, 2, 2).unwrap();
    assert_eq!(twice, rgba);
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_all_alpha_values() {
    let width = 4;
    let height = 3;
    let mut rgba = vec![0; (width * height * 4) as usize];
    for (index, alpha) in rgba.iter_mut().skip(3).step_by(4).enumerate() {
        *alpha = (index * 17) as u8;
    }

    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, width, height).unwrap();
    let mut source: Vec<_> = rgba.iter().skip(3).step_by(4).copied().collect();
    let mut converted: Vec<_> = result.iter().skip(3).step_by(4).copied().collect();
    source.sort_unstable();
    converted.sort_unstable();
    assert_eq!(converted, source);
}

#[cfg(windows)]
#[test]
fn validate_recording_start_accepts_maximum_fps() {
    assert!(rustshot::record::validate_recording_start(240).is_ok());
}

#[cfg(not(windows))]
#[test]
fn validate_recording_start_reports_platform_error() {
    assert!(rustshot::record::validate_recording_start(30).is_err());
}

#[test]
fn get_recording_path_in_dir_generates_mp4_inside_given_dir() {
    let dir = std::path::PathBuf::from("/custom/recordings");
    let path = rustshot::record::get_recording_path_in_dir(&dir);
    assert_eq!(path.parent(), Some(dir.as_path()));
    assert_eq!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("mp4")
    );
    assert!(path
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("recording_")));
}

#[test]
fn benchmark_copy_bgra_frame_exact_fit_copies_bottom_up() {
    // Two rows of one BGRA pixel each. The production copy flips the row order,
    // so the benchmark shim must hand back the bottom row first — otherwise the
    // benchmark would be measuring a different function than the recorder uses.
    let source = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let mut output = [0u8; 8];

    rustshot::record::benchmark_copy_bgra_frame_exact_fit(&source, 1, 2, &mut output)
        .expect("an exact-fit frame must copy");

    assert_eq!(output, [5, 6, 7, 8, 1, 2, 3, 4]);
}

#[test]
fn benchmark_copy_bgra_frame_exact_fit_rejects_an_oversized_row() {
    let mut output = [0u8; 8];
    let error =
        rustshot::record::benchmark_copy_bgra_frame_exact_fit(&[], u32::MAX, 1, &mut output)
            .expect_err("a row pitch that overflows must be reported");
    assert_eq!(error, "Captured frame row is too large");
}

#[test]
fn benchmark_copy_bgra_region_crops_pitched_source_and_flips_rows() {
    // Two 3-pixel rows with one padding pixel after each row. Selecting the
    // rightmost two pixels must ignore that pitch padding and emit bottom-up
    // rows, proving the benchmark adapter reaches the recorder's crop path.
    let source = [
        1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 201, 202, 203, 204, 13, 14, 15, 16, 17, 18, 19,
        20, 21, 22, 23, 24, 205, 206, 207, 208,
    ];
    let mut output = [0u8; 16];

    rustshot::record::benchmark_copy_bgra_region(
        &source,
        16,
        (3, 2),
        (1, 0, 2, 2),
        (2, 2),
        &mut output,
    )
    .expect("a valid pitched region must copy");

    assert_eq!(
        output,
        [17, 18, 19, 20, 21, 22, 23, 24, 5, 6, 7, 8, 9, 10, 11, 12]
    );
}

#[test]
fn benchmark_copy_bgra_region_preserves_source_pitch_validation() {
    let source = [0u8; 24];
    let mut output = [0u8; 4];

    let error = rustshot::record::benchmark_copy_bgra_region(
        &source,
        8,
        (3, 1),
        (0, 0, 1, 1),
        (1, 1),
        &mut output,
    )
    .expect_err("a pitch shorter than the captured source row must be rejected");

    assert_eq!(
        error,
        "Captured row pitch 8 is smaller than the 12-byte row"
    );
}
