//! Hidden adapters for benchmarks and executable specifications.

use super::*;

/// Validate and resolve recording output dimensions without starting a worker.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_recording_dimensions(
    width: u32,
    height: u32,
) -> Result<(u32, u32, usize), String> {
    let dimensions = RecordingDimensions::try_from(Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: width,
        physical_height: height,
    })?;
    Ok((
        dimensions.width().get(),
        dimensions.height().get(),
        dimensions.rgba_len(),
    ))
}

/// Compute exact Media Foundation timing through validated domain types.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_frame_timing_hns(frame_index: u64, fps: u32) -> Result<(i64, i64), String> {
    let timing = frame_timing_hns(
        FrameIndex::saturating_from_u128(u128::from(frame_index)),
        Fps::new(fps)?,
    );
    Ok((timing.timestamp().get(), timing.duration().get()))
}

/// Derive the bounded recording bitrate through validated domain types.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_recording_bitrate(width: u32, height: u32, fps: u32) -> Result<u32, String> {
    let dimensions = RecordingDimensions::try_from(Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: width,
        physical_height: height,
    })?;
    Ok(recording_bitrate(dimensions, Fps::new(fps)?).get())
}
