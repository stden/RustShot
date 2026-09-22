//! Tests for the recording domain value types.

use crate::record::*;

fn region(width: u32, height: u32) -> Region {
    Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: width,
        physical_height: height,
    }
}

#[test]
fn recording_width_and_height_are_distinct_validated_types() {
    let width = Width::try_from(1921).unwrap();
    let height = Height::try_from(1079).unwrap();

    assert_eq!(width.get(), 1922);
    assert_eq!(height.get(), 1080);
    assert_eq!(
        Width::try_from(0),
        Err("Recording width must be greater than 0".to_string())
    );
    assert_eq!(
        Height::try_from(0),
        Err("Recording height must be greater than 0".to_string())
    );
}

#[test]
fn recording_dimensions_validate_round_and_precompute_buffer_length() {
    let dimensions = RecordingDimensions::try_from(region(1921, 1079)).unwrap();

    assert_eq!(dimensions.width().get(), 1922);
    assert_eq!(dimensions.height().get(), 1080);
    assert_eq!(dimensions.rgba_len(), 1922 * 1080 * 4);
}

#[test]
fn recording_dimensions_reject_zero_sized_regions() {
    assert_eq!(
        RecordingDimensions::try_from(region(0, 1080)),
        Err("Recording region must have non-zero size".to_string())
    );
}

#[test]
fn recording_dimensions_reject_values_that_cannot_be_encoded_or_allocated() {
    assert_eq!(
        RecordingDimensions::try_from(region(u32::MAX, 2)),
        Err("Recording dimension is too large to round to an even value".to_string())
    );
    assert_eq!(
        RecordingDimensions::try_from(region(u32::MAX - 1, u32::MAX - 1)),
        Err("Pixel buffer dimensions are too large".to_string())
    );
}

#[test]
fn frame_timing_keeps_index_timestamp_and_duration_distinct() {
    let fps = Fps::new(30).unwrap();
    let timing = frame_timing_hns(FrameIndex::new(2), fps);

    assert_eq!(timing.timestamp().get(), 666_666);
    assert_eq!(timing.duration().get(), 333_334);
}

#[test]
fn frame_index_deadline_multiplier_saturates_instead_of_wrapping() {
    // The encoder multiplies the frame interval by this value to find the next
    // sleep deadline. A plain `as u32` would wrap the first index past u32::MAX
    // back to 0, collapsing every later deadline into the past and turning the
    // paced loop into a busy spin. Saturating keeps the deadline in the future.
    assert_eq!(FrameIndex::new(7).deadline_multiplier(), 7);
    assert_eq!(
        FrameIndex::new(u64::from(u32::MAX)).deadline_multiplier(),
        u32::MAX
    );
    assert_eq!(
        FrameIndex::new(u64::from(u32::MAX) + 1).deadline_multiplier(),
        u32::MAX
    );
    assert_eq!(
        FrameIndex::saturating_from_u128(u128::MAX).deadline_multiplier(),
        u32::MAX
    );
}

#[test]
fn frame_index_displays_as_a_plain_integer() {
    // The index is interpolated into "Failed to write frame {frame_index}", so
    // its Display must be the bare number rather than the newtype's debug shape.
    assert_eq!(FrameIndex::ZERO.to_string(), "0");
    assert_eq!(FrameIndex::new(1234).to_string(), "1234");
}

#[test]
fn bitrate_is_derived_from_validated_dimensions_and_fps() {
    let dimensions = RecordingDimensions::try_from(region(1920, 1080)).unwrap();
    let bitrate = recording_bitrate(dimensions, Fps::new(30).unwrap());

    assert_eq!(bitrate.get(), 7_464_960);
}
