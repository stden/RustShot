//! Windows Media Foundation plumbing: COM/MF lifetime guards, the WMF
//! recording loop, and per-frame sample writing. Everything here is
//! `#[cfg(windows)]`; the module is empty on other platforms.

use super::*;

#[cfg(windows)]
use super::capture::RecordingFrameSource;
#[cfg(windows)]
use std::time::Duration;

/// RAII guard for COM initialisation.  Calls `CoInitializeEx` on creation
/// and `CoUninitialize` on drop, ensuring COM is always properly shut down.
#[cfg(windows)]
struct CoGuard;

#[cfg(windows)]
impl CoGuard {
    // SAFETY: `CoInitializeEx` is safe to call once per thread. The guard
    // ensures `CoUninitialize` is called exactly once (in `Drop`).
    unsafe fn init() -> Result<Self, String> {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| format!("CoInitializeEx failed: {e}"))?;
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for CoGuard {
    fn drop(&mut self) {
        // SAFETY: Paired with the `CoInitializeEx` in `init()`. Called
        // exactly once because `CoGuard` is not `Clone`.
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}

/// RAII guard for Media Foundation.  Calls `MFStartup` on creation and
/// `MFShutdown` on drop.
#[cfg(windows)]
struct MfGuard;

#[cfg(windows)]
impl MfGuard {
    // SAFETY: `MFStartup` must be called before any MF API. The guard
    // ensures `MFShutdown` is called exactly once (in `Drop`).
    unsafe fn startup() -> Result<Self, String> {
        use windows::Win32::Media::MediaFoundation::{MFStartup, MFSTARTUP_NOSOCKET, MF_VERSION};

        MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET).map_err(|e| format!("MFStartup failed: {e}"))?;
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for MfGuard {
    fn drop(&mut self) {
        // SAFETY: Paired with `MFStartup` in `startup()`. Called exactly
        // once because `MfGuard` is not `Clone`.
        unsafe {
            let _ = windows::Win32::Media::MediaFoundation::MFShutdown();
        }
    }
}

/// Perform the actual Media Foundation recording loop.
///
/// # Safety
///
/// Caller must ensure COM has **not** been initialised on this thread yet
/// (this function does it via [`CoGuard`]).  All COM pointers are released
/// inside the inner closure before `CoGuard`/`MfGuard` drop.
#[cfg(windows)]
pub(super) unsafe fn wmf_record(
    output_path: &PathBuf,
    screen_x: i32,
    screen_y: i32,
    region: Region,
    fps: u32,
    stop_rx: mpsc::Receiver<()>,
) -> Result<PathBuf, String> {
    use windows::{core::PCWSTR, Win32::Media::MediaFoundation::*};

    let _co = CoGuard::init()?;
    let _mf = MfGuard::startup()?;

    // Release all Media Foundation COM objects before shutting down MF/COM.
    let result = (|| -> Result<PathBuf, String> {
        // Ensure even dimensions (H264 requirement)
        let width = checked_round_to_even(region.physical_width)?;
        let height = checked_round_to_even(region.physical_height)?;
        let mut frame_source = RecordingFrameSource::new(screen_x, screen_y, region)?;

        let path_wide = crate::settings::wide(&output_path.to_string_lossy());
        let pcwstr = PCWSTR(path_wide.as_ptr());

        let sink_writer: IMFSinkWriter = MFCreateSinkWriterFromURL(pcwstr, None, None)
            .map_err(|e| format!("MFCreateSinkWriterFromURL failed: {e}"))?;

        // Output type: H264
        let out_video: IMFMediaType =
            MFCreateMediaType().map_err(|e| format!("MFCreateMediaType: {e}"))?;
        out_video
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(|e| format!("out SetGUID major: {e}"))?;
        out_video
            .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)
            .map_err(|e| format!("out SetGUID H264: {e}"))?;
        out_video
            .SetUINT32(&MF_MT_AVG_BITRATE, recording_bitrate(width, height, fps))
            .map_err(|e| format!("out SetUINT32 bitrate: {e}"))?;
        out_video
            .SetUINT64(&MF_MT_FRAME_SIZE, pack_u32_pair(width, height))
            .map_err(|e| format!("out SetUINT64 frame size: {e}"))?;
        out_video
            .SetUINT64(&MF_MT_FRAME_RATE, pack_u32_pair(fps, 1))
            .map_err(|e| format!("out SetUINT64 frame rate: {e}"))?;
        out_video
            .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .map_err(|e| format!("out SetUINT32 interlace: {e}"))?;
        out_video
            .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack_u32_pair(1, 1))
            .map_err(|e| format!("out SetUINT64 aspect ratio: {e}"))?;
        out_video
            .SetUINT32(&MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709.0 as u32)
            .map_err(|e| format!("out SetUINT32 primaries: {e}"))?;
        out_video
            .SetUINT32(&MF_MT_TRANSFER_FUNCTION, MFVideoTransFunc_709.0 as u32)
            .map_err(|e| format!("out SetUINT32 transfer function: {e}"))?;
        out_video
            .SetUINT32(&MF_MT_YUV_MATRIX, MFVideoTransferMatrix_BT709.0 as u32)
            .map_err(|e| format!("out SetUINT32 YUV matrix: {e}"))?;
        out_video
            .SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32)
            .map_err(|e| format!("out SetUINT32 nominal range: {e}"))?;

        let video_idx = sink_writer
            .AddStream(&out_video)
            .map_err(|e| format!("AddStream: {e}"))?;

        // Input type: RGB32 (BGRA bottom-up)
        let in_video: IMFMediaType =
            MFCreateMediaType().map_err(|e| format!("MFCreateMediaType in: {e}"))?;
        in_video
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(|e| format!("in SetGUID major: {e}"))?;
        in_video
            .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
            .map_err(|e| format!("in SetGUID RGB32: {e}"))?;
        in_video
            .SetUINT64(&MF_MT_FRAME_SIZE, pack_u32_pair(width, height))
            .map_err(|e| format!("in SetUINT64 frame size: {e}"))?;
        in_video
            .SetUINT64(&MF_MT_FRAME_RATE, pack_u32_pair(fps, 1))
            .map_err(|e| format!("in SetUINT64 frame rate: {e}"))?;
        in_video
            .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .map_err(|e| format!("in SetUINT32 interlace: {e}"))?;
        in_video
            .SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack_u32_pair(1, 1))
            .map_err(|e| format!("in SetUINT64 aspect ratio: {e}"))?;
        in_video
            .SetUINT32(&MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709.0 as u32)
            .map_err(|e| format!("in SetUINT32 primaries: {e}"))?;
        in_video
            .SetUINT32(&MF_MT_TRANSFER_FUNCTION, MFVideoTransFunc_sRGB.0 as u32)
            .map_err(|e| format!("in SetUINT32 transfer function: {e}"))?;
        in_video
            .SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_0_255.0 as u32)
            .map_err(|e| format!("in SetUINT32 nominal range: {e}"))?;

        sink_writer
            .SetInputMediaType(video_idx, &in_video, None)
            .map_err(|e| format!("SetInputMediaType: {e}"))?;

        sink_writer
            .BeginWriting()
            .map_err(|e| format!("BeginWriting: {e}"))?;

        let frame_interval = Duration::from_secs(1) / fps;

        let mut frame_index = 0i64;
        let mut has_frame = false;
        let mut timeline_start = None;
        let mut terminal_error = None;
        // Preallocated once and reused for every Media Foundation sample.
        let mut bgra_buffer = vec![0u8; checked_rgba_len(width, height)?];

        loop {
            if recording_stop_requested(&stop_rx) {
                break;
            }

            let t0 = std::time::Instant::now();

            // Block briefly only until the first compositor frame. Afterwards
            // capture is non-blocking: when the desktop has not produced a new
            // frame, reuse the previous buffer to keep constant-rate timestamps.
            let wait = if has_frame {
                Duration::ZERO
            } else {
                Duration::from_millis(100)
            };
            match frame_source.capture_into(width, height, &mut bgra_buffer, wait) {
                Ok(updated) => has_frame |= updated,
                Err(error) => {
                    terminal_error = Some(format!("Screen capture stopped: {error}"));
                    break;
                }
            }
            if !has_frame {
                continue;
            }

            let (timestamp_hns, frame_duration_hns) = frame_timing_hns(frame_index, fps);

            if let Err(e) = write_video_sample(
                &sink_writer,
                video_idx,
                &bgra_buffer,
                timestamp_hns,
                frame_duration_hns,
            ) {
                terminal_error = Some(format!("Failed to write frame {frame_index}: {e}"));
                break;
            }

            let timeline_start = *timeline_start.get_or_insert(t0);
            frame_index += 1;

            let next_deadline = timeline_start
                + frame_interval
                    .checked_mul(frame_index as u32)
                    .unwrap_or(Duration::MAX);
            if let Some(remaining) = next_deadline.checked_duration_since(std::time::Instant::now())
            {
                thread::sleep(remaining);
            }
        }

        let finalize_result = sink_writer.Finalize().map_err(|e| format!("Finalize: {e}"));
        match (terminal_error, finalize_result) {
            (Some(error), Ok(())) => return Err(error),
            (Some(error), Err(finalize_error)) => return Err(format!("{error}; {finalize_error}")),
            (None, Err(finalize_error)) => return Err(finalize_error),
            (None, Ok(())) => {}
        }

        Ok(output_path.to_owned())
    })();

    result
}

/// Write one video frame to the sink writer.
///
/// # Safety
///
/// Requires a valid, initialised `IMFSinkWriter` obtained from an active
/// Media Foundation session. `data` must be a valid BGRA bottom-up frame
/// buffer. The raw pointer from `MFCreateMemoryBuffer::Lock` is valid
/// only between `Lock` and `Unlock`.
#[cfg(windows)]
unsafe fn write_video_sample(
    sink_writer: &windows::Win32::Media::MediaFoundation::IMFSinkWriter,
    stream_index: u32,
    data: &[u8],
    timestamp_hns: i64,
    duration_hns: i64,
) -> Result<(), String> {
    use windows::Win32::Media::MediaFoundation::*;

    let buffer = MFCreateMemoryBuffer(data.len() as u32)
        .map_err(|e| format!("MFCreateMemoryBuffer: {e}"))?;

    let mut ptr: *mut u8 = std::ptr::null_mut();
    buffer
        .Lock(&mut ptr, None, None)
        .map_err(|e| format!("Lock: {e}"))?;
    std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
    buffer.Unlock().map_err(|e| format!("Unlock: {e}"))?;
    buffer
        .SetCurrentLength(data.len() as u32)
        .map_err(|e| format!("SetCurrentLength: {e}"))?;

    let sample: IMFSample = MFCreateSample().map_err(|e| format!("MFCreateSample: {e}"))?;
    sample
        .AddBuffer(&buffer)
        .map_err(|e| format!("AddBuffer: {e}"))?;
    sample
        .SetSampleTime(timestamp_hns)
        .map_err(|e| format!("SetSampleTime: {e}"))?;
    sample
        .SetSampleDuration(duration_hns)
        .map_err(|e| format!("SetSampleDuration: {e}"))?;
    sink_writer
        .WriteSample(stream_index, &sample)
        .map_err(|e| format!("WriteSample: {e}"))?;

    Ok(())
}
