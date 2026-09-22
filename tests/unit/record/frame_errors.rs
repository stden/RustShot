//! Guard paths in the frame cropping and tone-mapping helpers.
//!
//! Each branch below reports a distinct human-readable reason, and the
//! assertions pin the message text: that string is what reaches the user when a
//! recording refuses a frame, so a reworded or misordered check is a visible
//! regression rather than an internal detail.

use crate::domain::PhysicalSize;
use crate::record::*;

fn full_region(width: u32, height: u32) -> Region {
    Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: width,
        physical_height: height,
    }
}

#[test]
fn native_frame_copy_rejects_an_output_smaller_than_the_region() {
    // A 2x2 region cannot be written into a 1-column, 2-row encoder frame.
    let source = vec![0u8; 2 * 2 * 4];
    let mut output = vec![0u8; 2 * 4];

    let error = copy_bgra_region_bottom_up(
        FrameView::new(&source, PhysicalSize::new(2, 2), 8),
        full_region(2, 2),
        BgraFrameMut::new(&mut output, PhysicalSize::new(1, 2)),
    )
    .expect_err("an output frame narrower than the region must be rejected");

    assert_eq!(error, "Encoded frame is smaller than the recording region");
}

#[test]
fn native_frame_copy_reports_a_truncated_source_buffer() {
    // Two 8-byte rows at an 8-byte pitch need 16 bytes; only 12 are supplied.
    let mut output = vec![0u8; 2 * 2 * 4];

    let error = copy_bgra_region_bottom_up(
        FrameView::new(&[0; 12], PhysicalSize::new(2, 2), 8),
        full_region(2, 2),
        BgraFrameMut::new(&mut output, PhysicalSize::new(2, 2)),
    )
    .expect_err("a source shorter than the captured frame must be rejected");

    assert_eq!(error, "Captured frame has 12 bytes but 16 are required");
}

#[test]
fn scrgb_copy_rejects_undersized_output_and_bad_pitch() {
    let source = vec![0u8; 2 * 2 * 8];
    let mut output = vec![0u8; 2 * 2 * 4];

    let error = copy_scrgb_region_bottom_up(
        FrameView::new(&source, PhysicalSize::new(2, 2), 16),
        full_region(2, 2),
        1_000.0,
        BgraFrameMut::new(&mut output, PhysicalSize::new(1, 2)),
    )
    .expect_err("an output frame narrower than the region must be rejected");
    assert_eq!(error, "Encoded frame is smaller than the recording region");

    // FP16 rows are 8 bytes per pixel, so a 2-pixel row cannot fit in 8 bytes.
    let error = copy_scrgb_region_bottom_up(
        FrameView::new(&source, PhysicalSize::new(2, 2), 8),
        full_region(2, 2),
        1_000.0,
        BgraFrameMut::new(&mut output, PhysicalSize::new(2, 2)),
    )
    .expect_err("a pitch below one FP16 row must be rejected");
    assert_eq!(
        error,
        "Captured FP16 row pitch 8 is smaller than the 16-byte row"
    );
}

#[test]
fn scrgb_copy_reports_truncated_source_and_output_buffers() {
    let mut output = vec![0u8; 2 * 2 * 4];

    // Two FP16 rows at a 16-byte pitch need 32 bytes; only 20 are supplied.
    let error = copy_scrgb_region_bottom_up(
        FrameView::new(&[0; 20], PhysicalSize::new(2, 2), 16),
        full_region(2, 2),
        1_000.0,
        BgraFrameMut::new(&mut output, PhysicalSize::new(2, 2)),
    )
    .expect_err("a source shorter than the captured FP16 frame must be rejected");
    assert_eq!(
        error,
        "Captured FP16 frame has 20 bytes but 32 are required"
    );

    // A 2x2 RGB32 frame needs 16 bytes; only 12 are supplied.
    let source = vec![0u8; 2 * 2 * 8];
    let mut short_output = vec![0u8; 12];
    let error = copy_scrgb_region_bottom_up(
        FrameView::new(&source, PhysicalSize::new(2, 2), 16),
        full_region(2, 2),
        1_000.0,
        BgraFrameMut::new(&mut short_output, PhysicalSize::new(2, 2)),
    )
    .expect_err("an output buffer shorter than the encoded frame must be rejected");
    assert_eq!(error, "Encoded frame has 12 bytes but 16 are required");
}

#[test]
fn scrgb_copy_zero_pads_when_the_output_is_larger_than_the_region() {
    // One opaque white FP16 pixel cropped out of a 2x2 source into a 2x2 frame:
    // the three pixels the region does not cover must be cleared, not left as
    // whatever the reused encoder buffer held.
    let white = [0x3c00u16, 0x3c00, 0x3c00, 0x3c00];
    let mut source = Vec::new();
    for _ in 0..4 {
        source.extend(white.iter().flat_map(|half| half.to_le_bytes()));
    }
    let mut output = vec![0xcc; 2 * 2 * 4];

    copy_scrgb_region_bottom_up(
        FrameView::new(&source, PhysicalSize::new(2, 2), 16),
        full_region(1, 1),
        1_000.0,
        BgraFrameMut::new(&mut output, PhysicalSize::new(2, 2)),
    )
    .expect("a 1x1 region padded into a 2x2 frame is valid");

    // Bottom-up output: source row 0 is written to output row 1, so the single
    // region pixel lands at byte 8 and everything around it must be cleared.
    assert_eq!(&output[..8], &[0; 8], "rows above the pixel are cleared");
    assert_eq!(&output[12..], &[0; 4], "the rest of the row is cleared");
    assert_eq!(output[11], u8::MAX, "the copied pixel stays opaque");
    assert!(output[8] > 0, "the copied pixel keeps its luminance");
}

#[test]
fn scrgb_tone_mapping_treats_non_finite_channels_as_black() {
    for pixel in [
        tone_map_scrgb_pixel(f32::NAN, 0.0, 0.0, 1_000.0),
        tone_map_scrgb_pixel(0.0, f32::NAN, 0.0, 1_000.0),
        tone_map_scrgb_pixel(0.0, 0.0, f32::INFINITY, 1_000.0),
        // Negative scRGB values clamp to zero instead of wrapping.
        tone_map_scrgb_pixel(-1.0, -2.0, -3.0, 1_000.0),
    ] {
        assert_eq!(pixel, [0, 0, 0]);
    }

    // A non-finite channel must not drag down a channel that is finite and lit.
    let red_only = tone_map_scrgb_pixel(1.0, f32::NAN, f32::NEG_INFINITY, 1_000.0);
    assert!(red_only[0] > 0);
    assert_eq!(red_only[1], 0);
    assert_eq!(red_only[2], 0);
}

#[test]
fn rgba_to_bgra_bottom_up_into_reports_which_buffer_is_too_short() {
    let mut output = vec![0u8; 16];
    let error = rgba_to_bgra_bottom_up_into(&[0; 8], &mut output, 2, 2)
        .expect_err("a short input buffer must be rejected");
    assert_eq!(
        error,
        "RGBA input buffer is too short: expected 16 bytes, got 8"
    );

    let mut short_output = vec![0u8; 8];
    let error = rgba_to_bgra_bottom_up_into(&[0; 16], &mut short_output, 2, 2)
        .expect_err("a short output buffer must be rejected");
    assert_eq!(
        error,
        "BGRA output buffer is too short: expected 16 bytes, got 8"
    );
}

#[test]
fn rgba_to_bgra_bottom_up_in_place_reports_a_buffer_that_is_too_short() {
    // One buffer, so the message names no direction: it is the caller's own frame
    // that is short. Validation runs before the first swap, so a rejected frame is
    // also still intact rather than half-converted — the in-place form is the one
    // that could hand the caller back a corrupted screenshot along with an error.
    let mut pixels: Vec<u8> = (1..=12).collect();

    let error = rgba_to_bgra_bottom_up_in_place(&mut pixels, 2, 2)
        .expect_err("a buffer shorter than width * height * 4 must be rejected");

    assert_eq!(
        error,
        "Pixel buffer is too short: expected 16 bytes, got 12"
    );
    assert_eq!(
        pixels,
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        "a rejected frame must not be left half-converted"
    );
}
