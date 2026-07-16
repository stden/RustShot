// ---------------------------------------------------------------------------
// record module
// ---------------------------------------------------------------------------

#[test]
fn frame_duration_hns_30fps() {
    assert_eq!(rustshot::record::frame_duration_hns(30), 333333);
}

#[test]
fn frame_duration_hns_60fps() {
    assert_eq!(rustshot::record::frame_duration_hns(60), 166666);
}

#[test]
fn frame_duration_hns_zero_returns_zero() {
    assert_eq!(rustshot::record::frame_duration_hns(0), 0);
}

#[test]
fn frame_duration_hns_1fps() {
    assert_eq!(rustshot::record::frame_duration_hns(1), 10_000_000);
}

#[test]
fn round_to_even_even_unchanged() {
    assert_eq!(rustshot::record::round_to_even(1920), 1920);
}

#[test]
fn round_to_even_odd_rounds_up() {
    assert_eq!(rustshot::record::round_to_even(1921), 1922);
}

#[test]
fn round_to_even_zero() {
    assert_eq!(rustshot::record::round_to_even(0), 0);
}

#[test]
fn round_to_even_one() {
    assert_eq!(rustshot::record::round_to_even(1), 2);
}

#[test]
fn round_to_even_two() {
    assert_eq!(rustshot::record::round_to_even(2), 2);
}

#[test]
fn rgba_to_bgra_bottom_up_2x2() {
    // 2x2 RGBA image, top-down
    // Row 0: pixel(0,0)=[R,G,B,A]=[1,2,3,4], pixel(1,0)=[5,6,7,8]
    // Row 1: pixel(0,1)=[9,10,11,12], pixel(1,1)=[13,14,15,16]
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2).unwrap();
    // Bottom-up: row 1 first, then row 0
    // BGRA: swap R and B channels
    // Row 1 (dst row 0): [11,10,9,12, 15,14,13,16]
    // Row 0 (dst row 1): [3,2,1,4, 7,6,5,8]
    assert_eq!(
        result,
        vec![11, 10, 9, 12, 15, 14, 13, 16, 3, 2, 1, 4, 7, 6, 5, 8]
    );
}

#[test]
fn rgba_to_bgra_bottom_up_1x1() {
    let rgba = vec![100, 150, 200, 255];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 1).unwrap();
    assert_eq!(result, vec![200, 150, 100, 255]);
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_alpha() {
    let rgba = vec![10, 20, 30, 128];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 1).unwrap();
    assert_eq!(result[3], 128);
}

#[test]
fn pack_u32_pair_basic() {
    let packed = rustshot::record::pack_u32_pair(1920, 1080);
    assert_eq!(packed >> 32, 1920);
    assert_eq!(packed & 0xFFFF_FFFF, 1080);
}

#[test]
fn pack_u32_pair_zeros() {
    assert_eq!(rustshot::record::pack_u32_pair(0, 0), 0);
}

#[test]
fn pack_u32_pair_max_values() {
    let packed = rustshot::record::pack_u32_pair(u32::MAX, u32::MAX);
    assert_eq!(packed, u64::MAX);
}

#[test]
fn pack_u32_pair_hi_only() {
    let packed = rustshot::record::pack_u32_pair(1, 0);
    assert_eq!(packed, 1u64 << 32);
}

#[test]
fn pack_u32_pair_lo_only() {
    let packed = rustshot::record::pack_u32_pair(0, 42);
    assert_eq!(packed, 42);
}

#[test]
fn validate_recording_start_zero_fps_is_error() {
    assert!(rustshot::record::validate_recording_start(0).is_err());
}

#[test]
fn validate_recording_start_zero_fps_error_message() {
    let err = rustshot::record::validate_recording_start(0).unwrap_err();
    assert!(err.contains("FPS"));
}

#[cfg(windows)]
#[test]
fn validate_recording_start_valid_fps_ok() {
    assert!(rustshot::record::validate_recording_start(30).is_ok());
}

#[cfg(not(windows))]
#[test]
fn validate_recording_start_non_windows_is_error() {
    assert!(rustshot::record::validate_recording_start(30).is_err());
}

// ---------------------------------------------------------------------------
// record::get_recording_dir / get_recording_path
// ---------------------------------------------------------------------------

#[test]
fn get_recording_dir_ends_with_recordings() {
    let dir = rustshot::record::get_recording_dir();
    assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("recordings"));
}

#[test]
fn get_recording_dir_parent_is_rustshot() {
    let dir = rustshot::record::get_recording_dir();
    let parent = dir.parent().unwrap();
    assert_eq!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("RustShot")
    );
}

#[test]
fn get_recording_dir_consistent() {
    let a = rustshot::record::get_recording_dir();
    let b = rustshot::record::get_recording_dir();
    assert_eq!(a, b);
}

#[test]
fn get_recording_path_has_mp4_extension() {
    let path = rustshot::record::get_recording_path();
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
}

#[test]
fn get_recording_path_filename_starts_with_recording() {
    let path = rustshot::record::get_recording_path();
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
    assert!(
        name.starts_with("recording_"),
        "filename should start with 'recording_': {name}"
    );
}

#[test]
fn get_recording_path_parent_equals_recording_dir() {
    let path = rustshot::record::get_recording_path();
    let dir = rustshot::record::get_recording_dir();
    assert_eq!(path.parent().unwrap(), dir.as_path());
}

#[test]
fn get_recording_path_timestamp_segment_has_expected_length() {
    let path = rustshot::record::get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let inner = &name["recording_".len()..name.len() - ".mp4".len()];
    assert_eq!(inner.len(), 25);
}

#[test]
fn get_recording_path_filename_contains_only_valid_chars() {
    let path = rustshot::record::get_recording_path();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.'));
}

// ---------------------------------------------------------------------------
// record::get_recording_path_in_dir (consumed by benches/benchmarks/recording.rs)
// ---------------------------------------------------------------------------

#[test]
fn get_recording_path_in_dir_generates_mp4_inside_given_dir() {
    let dir = std::path::PathBuf::from("/custom/recordings");
    let path = rustshot::record::get_recording_path_in_dir(&dir);
    assert_eq!(path.parent(), Some(dir.as_path()));
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
    assert!(
        name.starts_with("recording_"),
        "filename should start with 'recording_': {name}"
    );
}

// ---------------------------------------------------------------------------
// record::rgba_to_bgra_bottom_up edge cases
// ---------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_output_length_equals_input() {
    let rgba = vec![0u8; 64];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 4, 4).unwrap();
    assert_eq!(result.len(), rgba.len());
}

#[test]
fn rgba_to_bgra_bottom_up_all_zeros() {
    let rgba = vec![0u8; 16];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2).unwrap();
    assert_eq!(result, vec![0u8; 16]);
}

#[test]
fn rgba_to_bgra_bottom_up_3x1_single_row_not_flipped() {
    // Single row: bottom-up flip is a no-op, only channel swap
    let rgba = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 3, 1).unwrap();
    // RGBA→BGRA: swap R↔B
    assert_eq!(
        result,
        vec![30, 20, 10, 40, 70, 60, 50, 80, 110, 100, 90, 120]
    );
}

#[test]
fn rgba_to_bgra_bottom_up_1x3_tall_image_flips() {
    // 3 rows, each 1 pixel wide
    // Row 0: [1,2,3,4], Row 1: [5,6,7,8], Row 2: [9,10,11,12]
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 1, 3).unwrap();
    // Bottom-up: Row2→dst0, Row1→dst1, Row0→dst2
    // + BGRA swap
    assert_eq!(result, vec![11, 10, 9, 12, 7, 6, 5, 8, 3, 2, 1, 4]);
}

#[test]
fn rgba_to_bgra_bottom_up_double_conversion_restores_row_order() {
    // Applying twice reverses the flip twice (identity for row order),
    // but the channel swap happens twice too (R↔B twice = identity)
    let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let once = rustshot::record::rgba_to_bgra_bottom_up(&rgba, 2, 2).unwrap();
    let twice = rustshot::record::rgba_to_bgra_bottom_up(&once, 2, 2).unwrap();
    assert_eq!(twice, rgba);
}

// ---------------------------------------------------------------------------
// rgba_to_bgra_bottom_up — larger images
// ---------------------------------------------------------------------------

#[test]
fn rgba_to_bgra_bottom_up_4x4_corners() {
    // Verify specific corner pixels in a 4x4 image
    let w = 4u32;
    let h = 4u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    // Top-left pixel: R=10
    rgba[0] = 10;
    // Bottom-right pixel: R=20 (last row, last pixel)
    let br = ((h - 1) * w + (w - 1)) as usize * 4;
    rgba[br] = 20;

    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, w, h).unwrap();

    // After flip: top-left of output = bottom-left of input
    // Bottom-right of input becomes top-right of output
    let tr_out = (w - 1) as usize * 4; // top-right of output = last pixel of first row
    assert_eq!(result[tr_out + 2], 20); // R moved to B channel pos 2 (BGRA)

    // Top-left of input becomes bottom-left of output
    let bl_out = ((h - 1) * w) as usize * 4;
    assert_eq!(result[bl_out + 2], 10); // R moved to B channel pos 2
}

#[test]
fn rgba_to_bgra_bottom_up_preserves_all_alpha_values() {
    let w = 4u32;
    let h = 3u32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    // Set distinct alpha values
    for i in 0..(w * h) as usize {
        rgba[i * 4 + 3] = (i * 17 % 256) as u8;
    }
    let result = rustshot::record::rgba_to_bgra_bottom_up(&rgba, w, h).unwrap();

    // All alpha values should still exist (just reordered by row flip)
    let mut src_alphas: Vec<u8> = (0..(w * h) as usize).map(|i| rgba[i * 4 + 3]).collect();
    let mut dst_alphas: Vec<u8> = (0..(w * h) as usize).map(|i| result[i * 4 + 3]).collect();
    src_alphas.sort();
    dst_alphas.sort();
    assert_eq!(src_alphas, dst_alphas);
}
