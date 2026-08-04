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
fn bitrate_is_derived_from_validated_dimensions_and_fps() {
    let dimensions = RecordingDimensions::try_from(region(1920, 1080)).unwrap();
    let bitrate = recording_bitrate(dimensions, Fps::new(30).unwrap());

    assert_eq!(bitrate.get(), 7_464_960);
}
