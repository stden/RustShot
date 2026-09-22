use super::mapped_bgrx_length;

#[test]
fn mapped_rows_accept_padding_and_smaller_frame_extent() {
    assert_eq!(mapped_bgrx_length((3, 2), (4, 3), 16), Some(32));
}

#[test]
fn mapped_rows_reject_invalid_extents_and_short_pitch() {
    for (frame, texture, pitch) in [
        ((0, 2), (4, 3), 16),
        ((3, 0), (4, 3), 16),
        ((5, 2), (4, 3), 20),
        ((3, 4), (4, 3), 16),
        ((3, 2), (4, 3), 11),
    ] {
        assert_eq!(mapped_bgrx_length(frame, texture, pitch), None);
    }
}

#[test]
fn mapped_rows_reject_lengths_beyond_rust_slice_limits() {
    assert_eq!(mapped_bgrx_length((1, 2), (1, 2), usize::MAX), None);
    assert_eq!(
        mapped_bgrx_length((1, 1), (1, 1), isize::MAX as usize + 1),
        None
    );
}
