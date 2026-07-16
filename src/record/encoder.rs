//! Windows Media Foundation plumbing: COM/MF lifetime guards, the WMF
//! recording loop, and per-frame sample writing. Everything here is
//! `#[cfg(windows)]`; the module is empty on other platforms.

use super::*;

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
    use screenshots::Screen;
    use windows::{core::PCWSTR, Win32::Media::MediaFoundation::*};

    let _co = CoGuard::init()?;
    let _mf = MfGuard::startup()?;

    // Release all Media Foundation COM objects before shutting down MF/COM.
    let result = (|| -> Result<PathBuf, String> {
        // Ensure even dimensions (H264 requirement)
        let width = round_to_even(region.physical_width);
        let height = round_to_even(region.physical_height);

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
            .SetUINT32(&MF_MT_AVG_BITRATE, 1_500_000)
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

        sink_writer
            .SetInputMediaType(video_idx, &in_video, None)
            .map_err(|e| format!("SetInputMediaType: {e}"))?;

        sink_writer
            .BeginWriting()
            .map_err(|e| format!("BeginWriting: {e}"))?;

        let frame_duration_hns = frame_duration_hns(fps);
        let frame_interval = Duration::from_secs(1) / fps;

        // Find the screen by its origin
        let screens = Screen::all().map_err(|e| format!("Failed to get screens: {e}"))?;
        let screen = screens
            .into_iter()
            .find(|s| s.display_info.x == screen_x && s.display_info.y == screen_y)
            .ok_or_else(|| format!("Screen at ({screen_x},{screen_y}) not found"))?;

        let mut frame_index = 0i64;
        // Preallocated once and reused for every frame. The hot loop below
        // captures ONLY the selected region and converts straight into this
        // buffer. An earlier version captured the whole screen and allocated a
        // fresh cropped-and-padded `RgbaImage` per frame; that per-frame
        // allocation was the dominant source of recording CPU load and lag.
        // Keep the buffer preallocated and keep the capture region-scoped.
        let mut bgra_buffer = vec![0u8; (width * height * 4) as usize];

        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            let t0 = std::time::Instant::now();

            // Capture only the requested region.
            // In screenshots 0.8.10, capture_area takes (x, y) relative to screen origin.
            let captured = match screen.capture_area(
                region.physical_x,
                region.physical_y,
                region.physical_width,
                region.physical_height,
            ) {
                Ok(img) => img,
                Err(e) => {
                    log::warn!("Failed to capture area: {e}");
                    thread::sleep(frame_interval);
                    continue;
                }
            };

            // If the captured area doesn't match the required even dimensions (H264),
            // we use our zero-copy into-buffer converter which handles the mismatch.
            rgba_to_bgra_bottom_up_into(captured.as_raw(), &mut bgra_buffer, width, height);

            let timestamp_hns = frame_index * frame_duration_hns;

            if let Err(e) = write_video_sample(
                &sink_writer,
                video_idx,
                &bgra_buffer,
                timestamp_hns,
                frame_duration_hns,
            ) {
                log::warn!("Failed to write frame {frame_index}: {e}");
            }

            frame_index += 1;

            let elapsed = t0.elapsed();
            if elapsed < frame_interval {
                thread::sleep(frame_interval - elapsed);
            }
        }

        sink_writer
            .Finalize()
            .map_err(|e| format!("Finalize: {e}"))?;

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
