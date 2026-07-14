//! Screen recording to MP4 (Windows only, H.264 via Media Foundation).
//!
//! # Key concepts
//!
//! - **HNS (hundred-nanosecond units):** Media Foundation timestamps are in
//!   100 ns ticks. 1 second = 10 000 000 HNS. See [`frame_duration_hns`].
//! - **`IMFSinkWriter`:** The MF object that muxes encoded H.264 frames into
//!   an MP4 container. Created by `MFCreateSinkWriterFromURL`.
//! - **RGB32 (BGRA bottom-up):** The input pixel format expected by the MF
//!   encoder. Frames are vertically flipped (bottom row first) with
//!   Blue-Green-Red-Alpha channel order. See [`rgba_to_bgra_bottom_up`].
//!
//! # Architecture
//!
//! - [`RecordingSession::start`] validates parameters, spawns a background
//!   thread, and returns immediately.
//! - The background thread runs [`record_loop`] → [`wmf_record`], which:
//!   1. Initialises COM + Media Foundation.
//!   2. Creates an `IMFSinkWriter` pointed at an MP4 file.
//!   3. Captures frames via the `screenshots` crate, converts RGBA→BGRA
//!      bottom-up ([`rgba_to_bgra_bottom_up`]), and writes them as samples.
//!   4. Stops when a message arrives on `stop_rx`.
//! - [`RecordingSession::stop`] sends the stop signal and joins the thread.
//!
//! Helper functions extracted for testability:
//! - [`frame_duration_hns`] — frame duration in 100ns units.
//! - [`round_to_even`] — H.264 requires even dimensions.
//! - [`get_recording_dir`] / [`get_recording_path`] — output path logic.

use crate::selection::Region;
use crate::settings::Fps;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
#[cfg(windows)]
use std::time::Duration;

/// An active screen recording session.
///
/// Created with [`RecordingSession::start`] and stopped with [`RecordingSession::stop`].
/// Recording runs on a background thread using Windows Media Foundation (H.264 MP4).
pub struct RecordingSession {
    stop_tx: mpsc::SyncSender<()>,
    join: Option<thread::JoinHandle<Result<PathBuf, String>>>,
    output_path: PathBuf,
}

impl RecordingSession {
    /// Start recording the given screen region at the specified frame rate.
    ///
    /// `screen_x`/`screen_y` identify the monitor by its display origin.
    /// `region` is the capture area within that monitor (screen-local coords).
    /// Returns immediately; encoding runs on a background thread.
    #[allow(dead_code)]
    pub fn start(screen_x: i32, screen_y: i32, region: Region, fps: u32) -> Result<Self, String> {
        Self::start_in_dir(screen_x, screen_y, region, fps, get_recording_dir())
    }

    /// Start recording and save the MP4 under the provided recordings directory.
    pub fn start_in_dir(
        screen_x: i32,
        screen_y: i32,
        region: Region,
        fps: u32,
        recordings_dir: PathBuf,
    ) -> Result<Self, String> {
        validate_recording_start(fps)?;
        let output_path = get_recording_path_in_dir(&recordings_dir);
        let path_clone = output_path.clone();
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let join = thread::spawn(move || {
            record_loop(path_clone, screen_x, screen_y, region, fps, stop_rx)
        });
        Ok(Self {
            stop_tx,
            join: Some(join),
            output_path,
        })
    }

    /// Signal the recording thread to stop and wait for the MP4 to be finalized.
    ///
    /// Blocks until the background thread exits. Returns the path to the saved file.
    pub fn stop(mut self) -> Result<PathBuf, String> {
        let _ = self.stop_tx.send(());
        self.join
            .take()
            .map(|j| {
                j.join()
                    .map_err(|_| "Recording thread panicked".to_string())?
            })
            .unwrap_or(Ok(self.output_path.clone()))
    }

    /// Returns a reference to the output MP4 file path.
    #[allow(dead_code)]
    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }
}

/// Returns the directory where recordings are saved: `Pictures/RustShot/recordings`.
#[allow(dead_code)]
pub fn get_recording_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustShot")
        .join("recordings")
}

/// Returns a timestamped MP4 path inside [`get_recording_dir`].
#[allow(dead_code)]
pub fn get_recording_path() -> PathBuf {
    let recordings_dir = get_recording_dir();
    get_recording_path_in_dir(&recordings_dir)
}

/// Returns a timestamped MP4 path inside the provided recordings directory.
pub fn get_recording_path_in_dir(recordings_dir: &Path) -> PathBuf {
    let timestamp = crate::settings::generate_timestamp();
    recordings_dir.join(format!("recording_{}.mp4", timestamp))
}

/// Check that a requested FPS can actually start a recording.
///
/// Two distinct concerns, in order:
/// 1. **Is the value a valid rate at all?** Delegated to [`Fps::new`], which owns
///    the `1..=Fps::MAX` rule. Range checking is not duplicated here — the
///    newtype is the single place that rule lives.
/// 2. **Can this platform record?** Recording needs Windows Media Foundation, so
///    non-Windows builds fail here.
///
/// The ordering keeps error reporting deterministic: invalid input is reported
/// as invalid input even on platforms that cannot record at all.
///
/// This takes a raw `u32` on purpose — it is the boundary that turns untrusted
/// input into a validated [`Fps`]. Code that already holds an `Fps` (e.g. from
/// [`crate::settings::Settings`]) has no range check left to perform.
pub fn validate_recording_start(fps: u32) -> Result<(), String> {
    Fps::new(fps)?;

    #[cfg(not(windows))]
    {
        Err("Recording is only supported on Windows".to_string())
    }

    #[cfg(windows)]
    {
        Ok(())
    }
}

/// Compute the frame duration in 100-nanosecond units (HNS) for a given FPS.
///
/// WMF timestamps use 100ns ticks. A 30 fps video has 333 333 HNS per frame.
/// Returns 0 if `fps` is 0 to avoid division by zero.
///
/// Algorithm:
/// 1. Treat one second as `10_000_000` HNS.
/// 2. Divide that constant by the requested FPS.
/// 3. Use integer division because WMF timestamps are integer-based anyway.
///
/// The result is intentionally truncated rather than rounded. That keeps the
/// math fast and deterministic, and the small remainder is acceptable for the
/// current fixed-rate recording loop.
///
/// Possible improvements:
/// - Carry the remainder between frames to reduce long-recording drift.
/// - Represent frame timing as a rational pair if variable-rate timing or
///   exact NTSC-style fractions are ever required.
///
/// # Examples
///
/// ```
/// use rustshot::record::frame_duration_hns;
///
/// assert_eq!(frame_duration_hns(30), 333_333);
/// assert_eq!(frame_duration_hns(60), 166_666);
/// assert_eq!(frame_duration_hns(0), 0);
/// ```
#[allow(dead_code)]
pub fn frame_duration_hns(fps: u32) -> i64 {
    if fps == 0 {
        return 0;
    }
    10_000_000i64 / fps as i64
}

/// Round a dimension up to the nearest even number (H.264 requirement).
///
/// H.264 encoders require even width and height. This rounds odd values up by 1.
///
/// Algorithm:
/// 1. Add 1 to the input.
/// 2. Clear the least-significant bit.
/// 3. Even values remain unchanged; odd values round up to the next even.
///
/// This bit trick is branch-free and cheap, which is useful in sizing code
/// that may run repeatedly during capture setup.
///
/// Possible improvements:
/// - Switch to `saturating_add(1)` if release-mode overflow behavior on
///   extremely large dimensions ever becomes a concern.
/// - Return a checked error instead of wrapping semantics if invalid geometry
///   should be surfaced earlier in the pipeline.
///
/// # Examples
///
/// ```
/// use rustshot::record::round_to_even;
///
/// assert_eq!(round_to_even(1920), 1920); // already even
/// assert_eq!(round_to_even(1921), 1922); // rounded up
/// assert_eq!(round_to_even(0), 0);
/// ```
#[allow(dead_code)]
pub fn round_to_even(value: u32) -> u32 {
    (value + 1) & !1
}

#[cfg(test)]
fn validate_capture_region(
    region: Region,
    frame_width: u32,
    frame_height: u32,
) -> Result<(), String> {
    // Validate in three stages: origin, non-zero size, then far edge. Splitting
    // the checks this way keeps the error messages simple and predictable.
    if region.physical_x < 0 || region.physical_y < 0 {
        return Err("Recording region must stay within the selected monitor".to_string());
    }

    if region.physical_width == 0 || region.physical_height == 0 {
        return Err("Recording region must have non-zero size".to_string());
    }

    let region_right = i64::from(region.physical_x) + i64::from(region.physical_width);
    let region_bottom = i64::from(region.physical_y) + i64::from(region.physical_height);

    if region_right > i64::from(frame_width) || region_bottom > i64::from(frame_height) {
        return Err("Recording region must stay within the selected monitor".to_string());
    }

    Ok(())
}

/// Convert RGBA (top-down) to BGRA bottom-up (WMF RGB32 format).
///
/// Two-pass: memcpy rows in reverse order, then swap R↔B in place.
///
/// Algorithm:
/// 1. Allocate an output buffer of identical size.
/// 2. Copy each source row into the mirrored destination row so the image
///    becomes bottom-up.
/// 3. Walk the output buffer in 4-byte pixels and swap red with blue.
/// 4. Preserve green and alpha unchanged.
///
/// The two-pass approach keeps the code simple and avoids having to reason
/// about overlapping writes while rows are being reversed.
///
/// Possible improvements:
/// - Fuse the row flip and channel swap into one pass if profiling shows the
///   extra memory traversal to be significant.
/// - Reuse a caller-provided scratch buffer to reduce allocations during long
///   recording sessions.
///
/// # Examples
///
/// ```
/// use rustshot::record::rgba_to_bgra_bottom_up;
///
/// // 2x1 image: pixel₀ = (R=10, G=20, B=30, A=255)
/// let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255];
/// let bgra = rgba_to_bgra_bottom_up(&rgba, 2, 1);
/// // Single row, so bottom-up is the same order; R↔B swapped:
/// assert_eq!(bgra, vec![30, 20, 10, 255, 60, 50, 40, 255]);
/// ```
#[allow(dead_code)]
pub fn rgba_to_bgra_bottom_up(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = vec![0u8; (width * height * 4) as usize];
    rgba_to_bgra_bottom_up_into(rgba, &mut out, width, height);
    out
}

/// Optimized RGBA to BGRA bottom-up conversion.
///
/// Writes the converted pixels into the provided `out` buffer.
/// The `out` buffer must have size at least `width * height * 4`.
pub fn rgba_to_bgra_bottom_up_into(rgba: &[u8], out: &mut [u8], width: u32, height: u32) {
    let row_bytes = (width * 4) as usize;
    let height = height as usize;

    for y in 0..height {
        let src_start = y * row_bytes;
        let dst_start = (height - 1 - y) * row_bytes;
        let src_row = &rgba[src_start..src_start + row_bytes];
        let dst_row = &mut out[dst_start..dst_start + row_bytes];

        for (src_px, dst_px) in src_row
            .as_chunks::<4>()
            .0
            .iter()
            .zip(dst_row.as_chunks_mut::<4>().0.iter_mut())
        {
            dst_px[0] = src_px[2]; // B <- R
            dst_px[1] = src_px[1]; // G <- G
            dst_px[2] = src_px[0]; // R <- B
            dst_px[3] = src_px[3]; // A <- A
        }
    }
}

#[cfg(windows)]
fn record_loop(
    output_path: PathBuf,
    screen_x: i32,
    screen_y: i32,
    region: Region,
    fps: u32,
    stop_rx: mpsc::Receiver<()>,
) -> Result<PathBuf, String> {
    use std::fs;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create recording dir: {}", e))?;
    }
    // SAFETY: `wmf_record` calls COM and Media Foundation APIs that require
    // the current thread to have COM initialised (done inside `wmf_record`
    // via `CoGuard`). The output path, region, and stop channel are valid.
    unsafe { wmf_record(&output_path, screen_x, screen_y, region, fps, stop_rx) }
}

#[cfg(not(windows))]
fn record_loop(
    _output_path: PathBuf,
    _screen_x: i32,
    _screen_y: i32,
    _region: Region,
    _fps: u32,
    _stop_rx: mpsc::Receiver<()>,
) -> Result<PathBuf, String> {
    Err("Recording is only supported on Windows".to_string())
}

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
unsafe fn wmf_record(
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

/// Pack two u32 values into a single u64 (hi in upper 32 bits, lo in lower 32).
///
/// Used to encode frame size (`width | height`) and frame rate (`fps | 1`)
/// for Media Foundation attribute APIs that take a single u64.
///
/// # Examples
///
/// ```
/// use rustshot::record::pack_u32_pair;
///
/// assert_eq!(pack_u32_pair(1920, 1080), (1920u64 << 32) | 1080);
/// assert_eq!(pack_u32_pair(30, 1), (30u64 << 32) | 1);
/// ```
#[allow(dead_code)]
pub fn pack_u32_pair(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | (lo as u64)
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

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // get_recording_dir tests
    // -------------------------------------------------------------------------

    #[test]
    fn get_recording_dir_is_named_recordings_under_rustshot() {
        let dir = get_recording_dir();
        assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some("recordings"));
        let parent = dir.parent().unwrap();
        assert_eq!(
            parent.file_name().and_then(|n| n.to_str()),
            Some("RustShot")
        );
    }

    #[test]
    fn get_recording_dir_parent_is_rustshot() {
        let dir = get_recording_dir();
        let parent = dir.parent().expect("recordings dir must have a parent");
        assert_eq!(
            parent.file_name().and_then(|n| n.to_str()),
            Some("RustShot"),
            "parent of recordings dir should be named RustShot"
        );
    }

    #[test]
    fn get_recording_dir_grandparent_is_pictures_or_dot() {
        let dir = get_recording_dir();
        let rustshot = dir.parent().expect("recordings must have parent");
        let grandparent = rustshot.parent().expect("RustShot must have parent");
        let name = grandparent
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(".");
        // Grandparent is either the OS pictures dir (some name) or "." as fallback
        // We cannot predict the exact name, but it must be non-empty
        assert!(
            !name.is_empty(),
            "grandparent directory name should not be empty, got: {name}"
        );
    }

    #[test]
    fn get_recording_dir_is_absolute_or_dot_relative() {
        let dir = get_recording_dir();
        let is_absolute = dir.is_absolute();
        let starts_with_dot = dir.to_string_lossy().starts_with('.');
        assert!(
            is_absolute || starts_with_dot,
            "path should be absolute or start with '.', got: {}",
            dir.display()
        );
    }

    #[test]
    fn get_recording_dir_consistent_across_calls() {
        let dir1 = get_recording_dir();
        let dir2 = get_recording_dir();
        assert_eq!(
            dir1, dir2,
            "get_recording_dir must return the same path on repeated calls"
        );
    }

    #[test]
    fn get_recording_dir_has_at_least_three_components() {
        let dir = get_recording_dir();
        let component_count = dir.components().count();
        assert!(
            component_count >= 3,
            "path should have at least 3 components (root/pictures/RustShot/recordings), got: {}",
            component_count
        );
    }

    #[test]
    fn get_recording_dir_ends_with_recordings() {
        let dir = get_recording_dir();
        assert_eq!(
            dir.file_name().and_then(|n| n.to_str()),
            Some("recordings"),
            "last component of dir must be 'recordings'"
        );
    }

    // -------------------------------------------------------------------------
    // get_recording_path tests
    // -------------------------------------------------------------------------

    #[test]
    fn recording_output_path_has_mp4_extension() {
        let path = get_recording_path();
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
    }

    #[test]
    fn recording_path_filename_starts_with_recording() {
        let path = get_recording_path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(name.starts_with("recording_"), "got: {name}");
    }

    #[test]
    fn recording_path_is_under_recording_dir() {
        let path = get_recording_path();
        let parent = path.parent().expect("recording path must have a parent");
        let expected_dir = get_recording_dir();
        assert_eq!(
            parent, expected_dir,
            "parent of recording path should equal get_recording_dir()"
        );
    }

    #[test]
    fn get_recording_path_in_dir_uses_provided_directory() {
        let dir = PathBuf::from("/tmp/custom-recordings");
        let path = get_recording_path_in_dir(&dir);
        assert_eq!(path.parent(), Some(dir.as_path()));
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("mp4"));
    }

    #[test]
    fn get_recording_path_in_dir_filename_starts_with_recording() {
        let dir = PathBuf::from("/tmp/custom-recordings");
        let path = get_recording_path_in_dir(&dir);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(name.starts_with("recording_"));
        assert!(name.ends_with(".mp4"));
    }

    #[test]
    fn recording_path_filename_contains_digits() {
        let path = get_recording_path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let has_digits = name.chars().any(|c| c.is_ascii_digit());
        assert!(
            has_digits,
            "filename should contain timestamp digits, got: {name}"
        );
    }

    #[test]
    fn recording_path_two_calls_same_prefix_and_suffix() {
        let p1 = get_recording_path();
        let p2 = get_recording_path();
        let n1 = p1.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let n2 = p2.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(
            n1.starts_with("recording_") && n1.ends_with(".mp4"),
            "first path should start with 'recording_' and end with '.mp4', got: {n1}"
        );
        assert!(
            n2.starts_with("recording_") && n2.ends_with(".mp4"),
            "second path should start with 'recording_' and end with '.mp4', got: {n2}"
        );
    }

    #[test]
    fn recording_path_filename_only_valid_chars() {
        let path = get_recording_path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        for ch in name.chars() {
            assert!(
                ch.is_alphanumeric() || ch == '_' || ch == '.',
                "filename contains unexpected character '{ch}' in: {name}"
            );
        }
    }

    #[test]
    fn validate_recording_start_rejects_zero_fps() {
        assert_eq!(
            validate_recording_start(0),
            Err("Recording FPS must be greater than 0".to_string())
        );
    }

    #[cfg(windows)]
    #[test]
    fn validate_recording_start_accepts_positive_fps_on_windows() {
        assert!(validate_recording_start(30).is_ok());
    }

    #[cfg(not(windows))]
    #[test]
    fn validate_recording_start_rejects_recording_on_non_windows() {
        assert_eq!(
            validate_recording_start(30),
            Err("Recording is only supported on Windows".to_string())
        );
    }

    #[test]
    fn validate_capture_region_accepts_exact_screen_bounds() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_rejects_negative_origin() {
        let region = Region {
            physical_x: -1,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_rejects_region_outside_frame() {
        let region = Region {
            physical_x: 1800,
            physical_y: 900,
            physical_width: 200,
            physical_height: 200,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_rejects_zero_width() {
        let region = Region {
            physical_x: 10,
            physical_y: 10,
            physical_width: 0,
            physical_height: 100,
        };
        assert_eq!(
            validate_capture_region(region, 1920, 1080),
            Err("Recording region must have non-zero size".to_string())
        );
    }

    #[test]
    fn validate_capture_region_rejects_zero_height() {
        let region = Region {
            physical_x: 10,
            physical_y: 10,
            physical_width: 100,
            physical_height: 0,
        };
        assert_eq!(
            validate_capture_region(region, 1920, 1080),
            Err("Recording region must have non-zero size".to_string())
        );
    }

    #[test]
    fn validate_capture_region_accepts_offset_region_touching_frame_edge() {
        let region = Region {
            physical_x: 1820,
            physical_y: 980,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn recording_session_start_rejects_zero_fps() {
        let result = RecordingSession::start(
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            0,
        );

        assert!(matches!(
            result,
            Err(ref error) if error == "Recording FPS must be greater than 0"
        ));
    }

    #[test]
    fn recording_session_start_in_dir_rejects_zero_fps() {
        let result = RecordingSession::start_in_dir(
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            0,
            PathBuf::from("/tmp/custom-recordings"),
        );

        assert!(matches!(
            result,
            Err(ref error) if error == "Recording FPS must be greater than 0"
        ));
    }

    // -------------------------------------------------------------------------
    // rgba_to_bgra_bottom_up tests (original 3 kept + new tests)
    // -------------------------------------------------------------------------

    #[test]
    fn rgba_to_bgra_bottom_up_swaps_rb_and_flips() {
        // 1x2 image: top row = [R=255,G=0,B=0,A=255], bottom row = [R=0,G=0,B=255,A=255]
        let rgba = vec![
            255, 0, 0, 255, // top row: red
            0, 0, 255, 255, // bottom row: blue
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 2);
        // Bottom-up: output row 0 = src row 1 (blue), output row 1 = src row 0 (red)
        // Blue pixel in BGRA = [B=255, G=0, R=0, A=255]
        assert_eq!(
            &out[0..4],
            &[255, 0, 0, 255],
            "bottom row (blue) should be first in output"
        );
        // Red pixel in BGRA = [B=0, G=0, R=255, A=255]
        assert_eq!(
            &out[4..8],
            &[0, 0, 255, 255],
            "top row (red) should be second in output"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_single_black_pixel() {
        let rgba = vec![0u8, 0, 0, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(out, vec![0, 0, 0, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_output_length_matches_input() {
        let rgba = vec![1u8; 4 * 10 * 10];
        let out = rgba_to_bgra_bottom_up(&rgba, 10, 10);
        assert_eq!(out.len(), rgba.len());
    }

    #[test]
    fn rgba_to_bgra_bottom_up_single_white_pixel() {
        // White pixel: R=255, G=255, B=255, A=255 — R/B swap is symmetric for white
        let rgba = vec![255u8, 255, 255, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(out, vec![255, 255, 255, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_green_pixel() {
        // Green pixel: R=0, G=255, B=0, A=255 — G channel stays at position 1
        // RGBA [0,255,0,255] → BGRA [B=0,G=255,R=0,A=255]
        let rgba = vec![0u8, 255, 0, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(
            out,
            vec![0, 255, 0, 255],
            "green pixel BGRA should be [0,255,0,255]"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_red_pixel_becomes_blue_channel() {
        // Pure red RGBA [255,0,0,255] → BGRA: out[0]=B=0, out[2]=R=255
        let rgba = vec![255u8, 0, 0, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(
            out[0], 0,
            "blue channel (index 0) should be 0 for pure red input"
        );
        assert_eq!(
            out[2], 255,
            "red channel (index 2) should be 255 for pure red input"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_alpha_preserved() {
        // Arbitrary alpha value — must survive conversion unchanged
        let rgba = vec![10u8, 20, 30, 128];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(
            out[3], 128,
            "alpha channel must be unchanged after conversion"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_1x1_arbitrary_color() {
        // RGBA [100, 150, 200, 128] → BGRA [200, 150, 100, 128]
        let rgba = vec![100u8, 150, 200, 128];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(
            out,
            vec![200, 150, 100, 128],
            "expected BGRA [200,150,100,128] for RGBA [100,150,200,128]"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_2x2_row_order_reversed() {
        // 2x2 image with two distinct rows
        // Row 0 (top):    [R=255, G=0,   B=0,   A=255] [R=255, G=0,   B=0,   A=255]  (red)
        // Row 1 (bottom): [R=0,   G=0,   B=255, A=255] [R=0,   G=0,   B=255, A=255]  (blue)
        let rgba = vec![
            255, 0, 0, 255, 255, 0, 0, 255, // row 0: red, red
            0, 0, 255, 255, 0, 0, 255, 255, // row 1: blue, blue
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 2, 2);
        // Bottom-up output: row 0 of output = row 1 of input (blue → BGRA [255,0,0,255])
        assert_eq!(
            &out[0..4],
            &[255, 0, 0, 255],
            "output row 0 pixel 0 should be blue→BGRA"
        );
        assert_eq!(
            &out[4..8],
            &[255, 0, 0, 255],
            "output row 0 pixel 1 should be blue→BGRA"
        );
        // Row 1 of output = row 0 of input (red → BGRA [0,0,255,255])
        assert_eq!(
            &out[8..12],
            &[0, 0, 255, 255],
            "output row 1 pixel 0 should be red→BGRA"
        );
        assert_eq!(
            &out[12..16],
            &[0, 0, 255, 255],
            "output row 1 pixel 1 should be red→BGRA"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_1x1_is_identity_for_bgra_symmetric() {
        // Black pixel [0,0,0,255]: after R/B swap it remains the same (both are 0)
        let rgba = vec![0u8, 0, 0, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(out, vec![0, 0, 0, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_output_is_all_zero_for_zero_input() {
        // All-zero input (transparent black) must produce all-zero output
        let rgba = vec![0u8; 4 * 5 * 5];
        let out = rgba_to_bgra_bottom_up(&rgba, 5, 5);
        assert!(
            out.iter().all(|&b| b == 0),
            "all-zero input should produce all-zero output"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_large_image_correct_length() {
        // 1920x1080 input — output must be exactly 1920*1080*4 bytes
        let width: u32 = 1920;
        let height: u32 = 1080;
        let rgba = vec![0u8; (width * height * 4) as usize];
        let out = rgba_to_bgra_bottom_up(&rgba, width, height);
        assert_eq!(
            out.len(),
            (width * height * 4) as usize,
            "output length must equal width*height*4 for 1920x1080"
        );
    }

    #[test]
    fn rgba_to_bgra_bottom_up_3_row_image_middle_row_is_in_middle_output() {
        // 1x3 image: rows 0,1,2 — after bottom-up flip: output row 0 = src row 2, output row 2 = src row 0
        // Middle row (src row 1) must appear as output row 1 (middle stays middle in odd-height image)
        let red = [255u8, 0, 0, 255]; // row 0
        let green = [0u8, 255, 0, 255]; // row 1 (middle)
        let blue = [0u8, 0, 255, 255]; // row 2
        let rgba: Vec<u8> = [red, green, blue].concat();
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 3);

        // output row 0 = src row 2 (blue) → BGRA [255, 0, 0, 255]
        assert_eq!(
            &out[0..4],
            &[255, 0, 0, 255],
            "output row 0 should be blue (src row 2)"
        );
        // output row 1 = src row 1 (green) → BGRA [0, 255, 0, 255]
        assert_eq!(
            &out[4..8],
            &[0, 255, 0, 255],
            "output row 1 should be green (src row 1, middle)"
        );
        // output row 2 = src row 0 (red) → BGRA [0, 0, 255, 255]
        assert_eq!(
            &out[8..12],
            &[0, 0, 255, 255],
            "output row 2 should be red (src row 0)"
        );
    }

    // -------------------------------------------------------------------------
    // pack_u32_pair tests
    // -------------------------------------------------------------------------

    #[test]
    fn pack_u32_pair_30fps_encodes_correctly() {
        // pack_u32_pair(30, 1) should encode 30 fps as ((30 << 32) | 1)
        let result = pack_u32_pair(30, 1);
        let expected: u64 = ((30u64) << 32) | 1u64;
        assert_eq!(
            result, expected,
            "pack_u32_pair(30, 1) should equal 0x{:016X}",
            expected
        );
    }

    #[test]
    fn pack_u32_pair_zero_both() {
        assert_eq!(pack_u32_pair(0, 0), 0u64);
    }

    #[test]
    fn pack_u32_pair_max_hi() {
        let result = pack_u32_pair(u32::MAX, 0);
        let expected: u64 = (u32::MAX as u64) << 32;
        assert_eq!(result, expected);
    }

    #[test]
    fn pack_u32_pair_max_lo() {
        let result = pack_u32_pair(0, u32::MAX);
        let expected: u64 = u32::MAX as u64;
        assert_eq!(result, expected);
    }

    #[test]
    fn pack_u32_pair_hi_and_lo_independent() {
        // hi=1, lo=1: only bit 32 and bit 0 set
        let result = pack_u32_pair(1, 1);
        assert_eq!(result, (1u64 << 32) | 1u64);
    }

    // -------------------------------------------------------------------------
    // RecordingSession::output_path tests
    // (RecordingSession::start requires real hardware; test pure path logic only)
    // -------------------------------------------------------------------------

    #[test]
    fn recording_session_output_path_accessor_returns_internal_path() {
        let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
        let expected = PathBuf::from("recordings/test_output.mp4");
        let session = RecordingSession {
            stop_tx,
            join: None,
            output_path: expected.clone(),
        };

        assert_eq!(session.output_path(), &expected);
    }

    #[test]
    fn recording_session_output_path_accessor_returns_mp4() {
        // We cannot call RecordingSession::start in unit tests (needs WMF/screen),
        // but we can verify that the path produced by get_recording_path()
        // — which is exactly what start() uses for output_path — has .mp4 extension.
        // This exercises the same code path as output_path() would return.
        let path = get_recording_path();
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("mp4"),
            "output_path() value must have .mp4 extension"
        );
    }

    #[test]
    fn recording_session_output_path_is_under_recording_dir() {
        // Mirrors what output_path() returns: it is set to get_recording_path() in start()
        let path = get_recording_path();
        let parent = path.parent().expect("output path must have a parent");
        assert_eq!(
            parent,
            get_recording_dir(),
            "output_path() must be a child of get_recording_dir()"
        );
    }

    #[test]
    fn recording_session_stop_without_join_returns_stored_output_path() {
        let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
        let expected = PathBuf::from("recordings/fallback.mp4");
        let session = RecordingSession {
            stop_tx,
            join: None,
            output_path: expected.clone(),
        };

        assert_eq!(session.stop(), Ok(expected));
    }

    #[test]
    fn recording_session_stop_returns_path_from_recording_thread() {
        let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
        let expected = PathBuf::from("recordings/thread_result.mp4");
        let join = thread::spawn({
            let expected = expected.clone();
            move || Ok(expected)
        });
        let session = RecordingSession {
            stop_tx,
            join: Some(join),
            output_path: PathBuf::from("recordings/original.mp4"),
        };

        assert_eq!(session.stop(), Ok(expected));
    }

    #[test]
    fn recording_session_stop_propagates_recording_thread_error() {
        let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
        let join = thread::spawn(|| Err("encoder failed".to_string()));
        let session = RecordingSession {
            stop_tx,
            join: Some(join),
            output_path: PathBuf::from("recordings/original.mp4"),
        };

        assert_eq!(session.stop(), Err("encoder failed".to_string()));
    }

    #[test]
    fn recording_session_stop_reports_panicked_recording_thread() {
        let (stop_tx, _stop_rx) = mpsc::sync_channel(1);
        let join = thread::spawn(|| -> Result<PathBuf, String> {
            panic!("simulated recorder panic");
        });
        let session = RecordingSession {
            stop_tx,
            join: Some(join),
            output_path: PathBuf::from("recordings/original.mp4"),
        };

        assert_eq!(session.stop(), Err("Recording thread panicked".to_string()));
    }

    #[test]
    fn rgba_to_bgra_bottom_up_preserves_all_alpha_values() {
        // 2x1 image with different alpha values
        let rgba = vec![100, 150, 200, 0, 10, 20, 30, 128];
        let out = rgba_to_bgra_bottom_up(&rgba, 2, 1);
        assert_eq!(out[3], 0, "first pixel alpha should be 0");
        assert_eq!(out[7], 128, "second pixel alpha should be 128");
    }

    #[test]
    fn rgba_to_bgra_bottom_up_single_row_not_flipped() {
        // 3x1 image: only one row, so bottom-up flip is identity
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 3, 1);
        // Red RGBA→BGRA: [0, 0, 255, 255]
        assert_eq!(&out[0..4], &[0, 0, 255, 255]);
        // Green stays same in BGRA
        assert_eq!(&out[4..8], &[0, 255, 0, 255]);
        // Blue RGBA→BGRA: [255, 0, 0, 255]
        assert_eq!(&out[8..12], &[255, 0, 0, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_double_conversion_restores_original_row_order() {
        // Applying bottom-up flip twice should restore original row order
        // (R/B swap applied twice also restores channels)
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 128];
        let first = rgba_to_bgra_bottom_up(&rgba, 1, 2);
        let second = rgba_to_bgra_bottom_up(&first, 1, 2);
        assert_eq!(second, rgba);
    }

    #[test]
    fn pack_u32_pair_frame_size_1920x1080() {
        let result = pack_u32_pair(1920, 1080);
        let expected = ((1920u64) << 32) | 1080u64;
        assert_eq!(result, expected);
    }

    #[test]
    fn pack_u32_pair_is_reversible() {
        let packed = pack_u32_pair(1234, 5678);
        let hi = (packed >> 32) as u32;
        let lo = packed as u32;
        assert_eq!(hi, 1234);
        assert_eq!(lo, 5678);
    }

    // -------------------------------------------------------------------------
    // frame_duration_hns tests (TC-FPS-001 through TC-FPS-004)
    // -------------------------------------------------------------------------

    #[test]
    fn frame_duration_hns_30fps() {
        // 10_000_000 / 30 = 333333
        assert_eq!(frame_duration_hns(30), 333333);
    }

    #[test]
    fn frame_duration_hns_60fps() {
        // 10_000_000 / 60 = 166666
        assert_eq!(frame_duration_hns(60), 166666);
    }

    #[test]
    fn frame_duration_hns_1fps() {
        // 10_000_000 / 1 = 10_000_000 (one full second)
        assert_eq!(frame_duration_hns(1), 10_000_000);
    }

    #[test]
    fn frame_duration_hns_0fps_returns_zero() {
        // Edge case: 0 fps should not panic, returns 0
        assert_eq!(frame_duration_hns(0), 0);
    }

    #[test]
    fn frame_duration_hns_24fps() {
        // 10_000_000 / 24 = 416666
        assert_eq!(frame_duration_hns(24), 416666);
    }

    #[test]
    fn frame_duration_hns_is_positive_for_any_nonzero_fps() {
        for fps in [1, 2, 10, 15, 24, 25, 30, 50, 60, 120, 144, 240] {
            assert!(
                frame_duration_hns(fps) > 0,
                "frame_duration_hns({fps}) should be positive"
            );
        }
    }

    #[test]
    fn frame_duration_hns_decreases_with_higher_fps() {
        let d30 = frame_duration_hns(30);
        let d60 = frame_duration_hns(60);
        assert!(
            d30 > d60,
            "30fps duration ({d30}) should be greater than 60fps duration ({d60})"
        );
    }

    // -------------------------------------------------------------------------
    // round_to_even tests (TC-H264-001 through TC-H264-003)
    // -------------------------------------------------------------------------

    #[test]
    fn round_to_even_even_stays_even() {
        assert_eq!(round_to_even(1920), 1920);
        assert_eq!(round_to_even(1080), 1080);
        assert_eq!(round_to_even(100), 100);
    }

    #[test]
    fn round_to_even_odd_rounds_up() {
        assert_eq!(round_to_even(1921), 1922);
        assert_eq!(round_to_even(1079), 1080);
        assert_eq!(round_to_even(101), 102);
    }

    #[test]
    fn round_to_even_zero_stays_zero() {
        assert_eq!(round_to_even(0), 0);
    }

    #[test]
    fn round_to_even_one_becomes_two() {
        assert_eq!(round_to_even(1), 2);
    }

    #[test]
    fn round_to_even_two_stays_two() {
        assert_eq!(round_to_even(2), 2);
    }

    #[test]
    fn round_to_even_result_is_always_even() {
        for v in 0..200 {
            let result = round_to_even(v);
            assert_eq!(
                result % 2,
                0,
                "round_to_even({v}) = {result}, which is not even"
            );
        }
    }

    #[test]
    fn round_to_even_result_is_gte_input() {
        for v in 0..200 {
            assert!(round_to_even(v) >= v, "round_to_even({v}) should be >= {v}");
        }
    }

    #[test]
    fn round_to_even_result_differs_by_at_most_one() {
        for v in 0..200 {
            let diff = round_to_even(v) - v;
            assert!(
                diff <= 1,
                "round_to_even({v}) differs from input by {diff}, expected at most 1"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Recording path naming convention tests (TC-RECERR-006 area)
    // -------------------------------------------------------------------------

    #[test]
    fn recording_path_filename_matches_pattern() {
        let path = get_recording_path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Pattern: recording_YYYYMMDD_HHMMSS.mp4
        let re_like = name.starts_with("recording_") && name.ends_with(".mp4");
        assert!(
            re_like,
            "filename should match recording_*.mp4, got: {name}"
        );
        // Strip prefix and suffix, check remaining is digits and underscore
        let inner = &name["recording_".len()..name.len() - ".mp4".len()];
        assert!(
            inner.chars().all(|c| c.is_ascii_digit() || c == '_'),
            "timestamp portion should be digits and underscores, got: {inner}"
        );
    }

    #[test]
    fn recording_path_timestamp_has_expected_length() {
        let path = get_recording_path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // "recording_YYYYMMDD_HHMMSS_NNNNNNNNN.mp4" → inner is 25 chars
        let inner = &name["recording_".len()..name.len() - ".mp4".len()];
        assert_eq!(
            inner.len(),
            25,
            "timestamp should be 25 chars (YYYYMMDD_HHMMSS_NNNNNNNNN), got {} chars: {inner}",
            inner.len()
        );
    }

    // --- validate_recording_start edge cases ---

    #[test]
    fn validate_recording_start_rejects_zero_fps_error_message() {
        let err = validate_recording_start(0).unwrap_err();
        assert!(err.contains("FPS"), "error should mention FPS: {err}");
        assert!(
            err.contains("greater than 0"),
            "error should say > 0: {err}"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn validate_recording_start_rejects_1_fps_on_non_windows() {
        assert!(validate_recording_start(1).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn validate_recording_start_rejects_high_fps_on_non_windows() {
        assert!(validate_recording_start(120).is_err());
    }

    // --- frame_duration_hns additional tests ---

    #[test]
    fn frame_duration_hns_120fps() {
        assert_eq!(frame_duration_hns(120), 10_000_000 / 120);
    }

    #[test]
    fn frame_duration_hns_1_is_one_second() {
        assert_eq!(frame_duration_hns(1), 10_000_000);
    }

    #[test]
    fn frame_duration_hns_max_fps_no_panic() {
        // Should not panic even for very high fps
        let _ = frame_duration_hns(u32::MAX);
    }

    #[test]
    fn frame_duration_hns_10fps() {
        assert_eq!(frame_duration_hns(10), 1_000_000);
    }

    // --- round_to_even additional tests ---

    #[test]
    fn round_to_even_large_odd() {
        assert_eq!(round_to_even(99999), 100000);
    }

    #[test]
    fn round_to_even_large_even() {
        assert_eq!(round_to_even(100000), 100000);
    }

    #[test]
    fn round_to_even_3_becomes_4() {
        assert_eq!(round_to_even(3), 4);
    }

    // --- rgba_to_bgra_bottom_up additional tests ---

    #[test]
    fn rgba_to_bgra_bottom_up_wide_single_row() {
        // 4x1 image: single row, no flip needed
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            128, 128, 128, 255, // gray
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 4, 1);
        // Red → BGRA: [0, 0, 255, 255]
        assert_eq!(&out[0..4], &[0, 0, 255, 255]);
        // Green stays
        assert_eq!(&out[4..8], &[0, 255, 0, 255]);
        // Blue → BGRA: [255, 0, 0, 255]
        assert_eq!(&out[8..12], &[255, 0, 0, 255]);
        // Gray → symmetric
        assert_eq!(&out[12..16], &[128, 128, 128, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_4x4_first_and_last_row_swapped() {
        // 1x4 image
        let rgba = vec![
            10, 20, 30, 255, // row 0
            40, 50, 60, 255, // row 1
            70, 80, 90, 255, // row 2
            100, 110, 120, 255, // row 3
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 4);
        // Output row 0 = src row 3: RGBA[100,110,120,255] → BGRA[120,110,100,255]
        assert_eq!(&out[0..4], &[120, 110, 100, 255]);
        // Output row 3 = src row 0: RGBA[10,20,30,255] → BGRA[30,20,10,255]
        assert_eq!(&out[12..16], &[30, 20, 10, 255]);
    }

    // --- RecordingSession::start with region edge cases ---

    #[test]
    fn recording_session_start_zero_fps_with_large_region() {
        let result = RecordingSession::start(
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 1920,
                physical_height: 1080,
            },
            0,
        );
        assert!(result.is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn recording_session_start_nonzero_fps_fails_on_non_windows() {
        let result = RecordingSession::start(
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            30,
        );
        assert!(result.is_err());
        // Can't use unwrap_err (no Debug on RecordingSession), so use match
        match result {
            Err(e) => assert!(e.contains("only supported on Windows")),
            Ok(_) => panic!("expected error on non-windows"),
        }
    }

    // -------------------------------------------------------------------------
    // Additional pack_u32_pair mutation-killing tests
    // -------------------------------------------------------------------------

    #[test]
    fn pack_u32_pair_hi_only_uses_upper_bits() {
        // If << were replaced with >>, hi=1 would give 0 instead of 1<<32
        let result = pack_u32_pair(1, 0);
        assert_eq!(result, 1u64 << 32);
        assert!(result > u32::MAX as u64, "hi must be in upper 32 bits");
    }

    #[test]
    fn pack_u32_pair_lo_only_uses_lower_bits() {
        // Ensure lo doesn't bleed into upper bits
        let result = pack_u32_pair(0, 42);
        assert_eq!(result, 42u64);
    }

    #[test]
    fn pack_u32_pair_or_not_and() {
        // If | were replaced with &, pack(1, 2) would be 0 instead of (1<<32)|2
        let result = pack_u32_pair(1, 2);
        assert_ne!(result, 0, "| must not be replaced with &");
        assert_eq!(result, (1u64 << 32) | 2u64);
    }

    #[test]
    fn pack_u32_pair_or_not_xor() {
        // For non-overlapping bits, ^ gives same result as |, but test with overlapping
        // hi=0, lo=0 -> result should be 0 (both & and ^ give 0 too, so test non-zero)
        let result = pack_u32_pair(0xFF, 0xFF);
        assert_eq!(result, (0xFFu64 << 32) | 0xFFu64);
    }

    #[test]
    fn pack_u32_pair_both_max() {
        let result = pack_u32_pair(u32::MAX, u32::MAX);
        assert_eq!(result, u64::MAX);
    }

    #[test]
    fn pack_u32_pair_extracts_hi_correctly() {
        let packed = pack_u32_pair(0xDEAD, 0xBEEF);
        assert_eq!((packed >> 32) as u32, 0xDEAD);
    }

    #[test]
    fn pack_u32_pair_extracts_lo_correctly() {
        let packed = pack_u32_pair(0xDEAD, 0xBEEF);
        assert_eq!(packed as u32, 0xBEEF);
    }

    // -------------------------------------------------------------------------
    // record_loop non-windows tests
    // -------------------------------------------------------------------------

    #[cfg(not(windows))]
    #[test]
    fn record_loop_returns_error_on_non_windows() {
        let (_, rx) = mpsc::sync_channel(1);
        let result = record_loop(
            PathBuf::from("/tmp/test.mp4"),
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            30,
            rx,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("only supported on Windows"));
    }

    #[cfg(not(windows))]
    #[test]
    fn record_loop_error_message_is_not_empty() {
        let (_, rx) = mpsc::sync_channel(1);
        let result = record_loop(
            PathBuf::from("/tmp/test.mp4"),
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 640,
                physical_height: 480,
            },
            60,
            rx,
        );
        assert!(!result.unwrap_err().is_empty());
    }

    // -------------------------------------------------------------------------
    // RecordingSession start/stop on non-windows
    // -------------------------------------------------------------------------

    #[cfg(not(windows))]
    #[test]
    fn recording_session_stop_returns_error_on_non_windows() {
        let session = RecordingSession::start(
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            30,
        );
        // start itself should fail on non-windows because the thread returns Err
        // but the session is created before the thread finishes
        match session {
            Ok(s) => {
                let result = s.stop();
                assert!(result.is_err());
            }
            Err(e) => assert!(e.contains("only supported on Windows")),
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn recording_session_output_path_is_mp4_even_on_non_windows() {
        let session = RecordingSession::start(
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            30,
        );
        if let Ok(s) = session {
            assert_eq!(
                s.output_path().extension().and_then(|e| e.to_str()),
                Some("mp4")
            );
            let _ = s.stop();
        }
    }

    // --- frame_duration_hns high fps tests ---

    #[test]
    fn frame_duration_hns_240fps() {
        assert_eq!(frame_duration_hns(240), 10_000_000 / 240);
    }

    #[test]
    fn frame_duration_hns_144fps() {
        assert_eq!(frame_duration_hns(144), 10_000_000 / 144);
    }

    #[test]
    fn frame_duration_hns_25fps_pal() {
        assert_eq!(frame_duration_hns(25), 400_000);
    }

    #[test]
    fn frame_duration_hns_15fps() {
        assert_eq!(frame_duration_hns(15), 10_000_000 / 15);
    }

    #[test]
    fn frame_duration_hns_2fps() {
        assert_eq!(frame_duration_hns(2), 5_000_000);
    }

    // --- round_to_even edge cases ---

    #[test]
    fn round_to_even_u32_max_minus_1_stays() {
        // u32::MAX - 1 = 4294967294, which is even
        assert_eq!(round_to_even(u32::MAX - 1), u32::MAX - 1);
    }

    #[test]
    fn round_to_even_4_stays_4() {
        assert_eq!(round_to_even(4), 4);
    }

    #[test]
    fn round_to_even_5_becomes_6() {
        assert_eq!(round_to_even(5), 6);
    }

    #[test]
    fn round_to_even_common_resolutions() {
        assert_eq!(round_to_even(1920), 1920);
        assert_eq!(round_to_even(1080), 1080);
        assert_eq!(round_to_even(2560), 2560);
        assert_eq!(round_to_even(1440), 1440);
        assert_eq!(round_to_even(3840), 3840);
        assert_eq!(round_to_even(2160), 2160);
    }

    // --- rgba_to_bgra_bottom_up asymmetric dimensions ---

    #[test]
    fn rgba_to_bgra_bottom_up_3x1_wide_image() {
        // 3 pixels wide, 1 pixel tall
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 3, 1);
        assert_eq!(out.len(), 12);
        // Red → BGRA [0, 0, 255, 255]
        assert_eq!(&out[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_1x3_tall_image_flips_rows() {
        let rgba = vec![
            10, 20, 30, 255, // row 0
            40, 50, 60, 255, // row 1
            70, 80, 90, 255, // row 2
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 3);
        // Output row 0 = src row 2: RGBA[70,80,90,255] → BGRA[90,80,70,255]
        assert_eq!(&out[0..4], &[90, 80, 70, 255]);
        // Output row 2 = src row 0: RGBA[10,20,30,255] → BGRA[30,20,10,255]
        assert_eq!(&out[8..12], &[30, 20, 10, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_asymmetric_5x2() {
        // 5 pixels wide, 2 rows tall
        let mut rgba = vec![0u8; 5 * 2 * 4];
        // Row 0: all red (255, 0, 0, 255)
        for x in 0..5 {
            rgba[x * 4] = 255;
            rgba[x * 4 + 3] = 255;
        }
        // Row 1: all blue (0, 0, 255, 255)
        for x in 0..5 {
            let offset = 5 * 4 + x * 4;
            rgba[offset + 2] = 255;
            rgba[offset + 3] = 255;
        }
        let out = rgba_to_bgra_bottom_up(&rgba, 5, 2);
        // Output row 0 = src row 1 (blue). Blue RGBA→BGRA: [255, 0, 0, 255]
        assert_eq!(&out[0..4], &[255, 0, 0, 255]);
        // Output row 1 = src row 0 (red). Red RGBA→BGRA: [0, 0, 255, 255]
        assert_eq!(&out[5 * 4..5 * 4 + 4], &[0, 0, 255, 255]);
    }

    // --- validate_recording_start boundary ---

    #[cfg(windows)]
    #[test]
    fn validate_recording_start_accepts_1_fps_on_windows() {
        assert!(validate_recording_start(1).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn validate_recording_start_accepts_240_fps_on_windows() {
        assert!(validate_recording_start(240).is_ok());
    }

    // --- recording path consistency ---

    #[test]
    fn get_recording_path_parent_equals_get_recording_dir() {
        let path = get_recording_path();
        let parent = path.parent().unwrap();
        assert_eq!(parent, get_recording_dir().as_path());
    }

    #[test]
    fn get_recording_path_filename_timestamp_is_25_chars() {
        let path = get_recording_path();
        let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
        // "recording_YYYYMMDD_HHMMSS_NNNNNNNNN" → stem after "recording_" is 25 chars
        let ts = &name["recording_".len()..];
        assert_eq!(ts.len(), 25);
        assert!(ts.chars().all(|c| c.is_ascii_digit() || c == '_'));
    }

    // =========================================================================
    // pack_u32_pair — additional coverage
    // =========================================================================

    #[test]
    fn pack_u32_pair_both_zero() {
        assert_eq!(pack_u32_pair(0, 0), 0u64);
    }

    #[test]
    fn pack_u32_pair_hi_only() {
        assert_eq!(pack_u32_pair(1, 0), 1u64 << 32);
    }

    #[test]
    fn pack_u32_pair_lo_only() {
        assert_eq!(pack_u32_pair(0, 42), 42u64);
    }

    #[test]
    fn pack_u32_pair_max_hi_zero_lo() {
        assert_eq!(pack_u32_pair(u32::MAX, 0), (u32::MAX as u64) << 32);
    }

    #[test]
    fn pack_u32_pair_4k_frame_size() {
        let packed = pack_u32_pair(3840, 2160);
        assert_eq!((packed >> 32) as u32, 3840);
        assert_eq!(packed as u32, 2160);
    }

    #[test]
    fn pack_u32_pair_roundtrip_arbitrary() {
        let packed = pack_u32_pair(12345, 67890);
        assert_eq!((packed >> 32) as u32, 12345);
        assert_eq!(packed as u32, 67890);
    }

    // =========================================================================
    // frame_duration_hns — additional coverage
    // =========================================================================

    #[test]
    fn frame_duration_hns_1_fps() {
        assert_eq!(frame_duration_hns(1), 10_000_000);
    }

    #[test]
    fn frame_duration_hns_10_fps() {
        assert_eq!(frame_duration_hns(10), 1_000_000);
    }

    #[test]
    fn frame_duration_hns_24_fps() {
        // 10_000_000 / 24 = 416666
        assert_eq!(frame_duration_hns(24), 416666);
    }

    #[test]
    fn frame_duration_hns_25_fps() {
        assert_eq!(frame_duration_hns(25), 400000);
    }

    #[test]
    fn frame_duration_hns_120_fps() {
        // 10_000_000 / 120 = 83333
        assert_eq!(frame_duration_hns(120), 83333);
    }

    // =========================================================================
    // round_to_even — additional coverage
    // =========================================================================

    #[test]
    fn round_to_even_2_stays_2() {
        assert_eq!(round_to_even(2), 2);
    }

    #[test]
    fn round_to_even_1080_is_unchanged() {
        assert_eq!(round_to_even(1080), 1080);
    }

    #[test]
    fn round_to_even_1079_rounds_up_to_1080() {
        assert_eq!(round_to_even(1079), 1080);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn round_to_even_u32_max_panics_in_debug() {
        round_to_even(u32::MAX);
    }

    #[test]
    fn round_to_even_u32_max_minus_1_unchanged() {
        assert_eq!(round_to_even(u32::MAX - 1), u32::MAX - 1);
    }

    // =========================================================================
    // rgba_to_bgra_bottom_up — additional coverage
    // =========================================================================

    #[test]
    fn rgba_to_bgra_bottom_up_2x2_correct_flip_and_swap() {
        // 2x2 image:
        // Row 0: (255, 0, 0, 255), (0, 255, 0, 255)
        // Row 1: (0, 0, 255, 255), (128, 128, 128, 255)
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, // row 0
            0, 0, 255, 255, 128, 128, 128, 255, // row 1
        ];
        let result = rgba_to_bgra_bottom_up(&rgba, 2, 2);
        // After bottom-up flip: row 1 becomes row 0, row 0 becomes row 1
        // After RGBA→BGRA swap (R↔B):
        // New row 0 (was row 1): (255, 0, 0, 255), (128, 128, 128, 255)
        // New row 1 (was row 0): (0, 0, 255, 255), (0, 255, 0, 255)
        assert_eq!(result[0], 255); // B channel of pixel (0,0,255) = 255
        assert_eq!(result[1], 0); // G
        assert_eq!(result[2], 0); // R→B swap: was R=0
        assert_eq!(result[3], 255); // A

        // Second pixel of new row 0: (128,128,128,255) → BGRA swap: still (128,128,128,255)
        assert_eq!(result[4], 128);
        assert_eq!(result[5], 128);
        assert_eq!(result[6], 128);
        assert_eq!(result[7], 255);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_preserves_alpha() {
        let rgba = vec![100, 150, 200, 50]; // 1x1 with alpha=50
        let result = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(result[3], 50); // alpha preserved
    }

    #[test]
    fn rgba_to_bgra_bottom_up_3x1_no_flip_only_swap() {
        // Single row: no vertical flip needed
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255];
        let result = rgba_to_bgra_bottom_up(&rgba, 3, 1);
        // Pixel 0: R=10, G=20, B=30 → BGRA: B=30, G=20, R=10
        assert_eq!(result[0], 30);
        assert_eq!(result[1], 20);
        assert_eq!(result[2], 10);
    }

    // =========================================================================
    // validate_capture_region — additional coverage
    // =========================================================================

    #[test]
    fn validate_capture_region_region_at_origin_full_frame() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_one_pixel_past_right() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1921,
            physical_height: 1080,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_one_pixel_past_bottom() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1081,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_offset_exactly_touching_edge() {
        let region = Region {
            physical_x: 100,
            physical_y: 200,
            physical_width: 1820,
            physical_height: 880,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_offset_one_past_edge() {
        let region = Region {
            physical_x: 100,
            physical_y: 200,
            physical_width: 1821,
            physical_height: 880,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_1x1_at_last_pixel() {
        let region = Region {
            physical_x: 1919,
            physical_y: 1079,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_negative_x_rejected() {
        let region = Region {
            physical_x: -1,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_negative_y_rejected() {
        let region = Region {
            physical_x: 0,
            physical_y: -1,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_zero_width_rejected() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_zero_height_rejected() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 0,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    // =========================================================================
    // get_recording_path_in_dir — additional coverage
    // =========================================================================

    #[test]
    fn get_recording_path_in_dir_respects_custom_dir() {
        let dir = PathBuf::from("/my/custom/dir");
        let path = get_recording_path_in_dir(&dir);
        assert_eq!(path.parent().unwrap(), dir.as_path());
    }

    #[test]
    fn get_recording_path_in_dir_always_mp4() {
        let dir = PathBuf::from("/tmp");
        let path = get_recording_path_in_dir(&dir);
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
    }

    #[test]
    fn get_recording_path_in_dir_stem_has_recording_prefix() {
        let dir = PathBuf::from("/tmp");
        let path = get_recording_path_in_dir(&dir);
        let stem = path.file_stem().and_then(|n| n.to_str()).unwrap();
        assert!(stem.starts_with("recording_"));
    }

    #[test]
    fn get_recording_path_in_dir_uses_subsecond_timestamp_precision() {
        let dir = PathBuf::from("/tmp");
        let path = get_recording_path_in_dir(&dir);
        let stem = path.file_stem().and_then(|n| n.to_str()).unwrap();
        let ts = &stem["recording_".len()..];
        assert_eq!(ts.len(), 25);
    }

    // =========================================================================
    // validate_recording_start — additional coverage
    // =========================================================================

    #[test]
    fn validate_recording_start_zero_fps_error_message() {
        let err = validate_recording_start(0).unwrap_err();
        assert!(err.contains("greater than 0"));
    }

    // --- validate_capture_region additional edge cases ---

    #[test]
    fn validate_capture_region_1x1_at_origin() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_both_zero_dimensions() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 0,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_negative_x_and_y() {
        let region = Region {
            physical_x: -10,
            physical_y: -20,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_exact_frame_size() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_one_pixel_wider_than_frame() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1921,
            physical_height: 1080,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_one_pixel_taller_than_frame() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1081,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_error_message_for_negative_coords() {
        let region = Region {
            physical_x: -1,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        };
        let err = validate_capture_region(region, 1920, 1080).unwrap_err();
        assert!(err.contains("must stay within"), "error: {err}");
    }

    #[test]
    fn validate_capture_region_error_message_for_zero_size() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 100,
        };
        let err = validate_capture_region(region, 1920, 1080).unwrap_err();
        assert!(err.contains("non-zero"), "error: {err}");
    }

    // --- frame_duration_hns edge cases ---

    #[test]
    fn frame_duration_hns_50fps() {
        assert_eq!(frame_duration_hns(50), 200_000);
    }

    #[test]
    fn frame_duration_hns_higher_fps_yields_smaller_duration() {
        assert!(frame_duration_hns(60) < frame_duration_hns(30));
        assert!(frame_duration_hns(120) < frame_duration_hns(60));
    }

    // --- round_to_even edge cases ---

    #[test]
    fn round_to_even_7_becomes_8() {
        assert_eq!(round_to_even(7), 8);
    }

    #[test]
    fn round_to_even_10_stays_10() {
        assert_eq!(round_to_even(10), 10);
    }

    // --- get_recording_path_in_dir edge cases ---

    #[test]
    fn get_recording_path_in_dir_filename_contains_underscore_separator() {
        let dir = PathBuf::from("/tmp/recs");
        let path = get_recording_path_in_dir(&dir);
        let filename = path.file_stem().unwrap().to_str().unwrap();
        assert!(
            filename.contains('_'),
            "filename should contain underscore separators"
        );
    }

    #[test]
    fn get_recording_path_in_dir_two_calls_produce_different_paths() {
        let dir = PathBuf::from("/tmp/recs");
        let p1 = get_recording_path_in_dir(&dir);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let p2 = get_recording_path_in_dir(&dir);
        // Timestamps include nanoseconds, so paths should differ
        assert_ne!(p1, p2);
    }

    // --- rgba_to_bgra_bottom_up edge cases ---

    #[test]
    fn rgba_to_bgra_bottom_up_1x1_pure_red() {
        let rgba = [255, 0, 0, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        // BGRA: blue=0, green=0, red=255, alpha=255 but swapped R and B
        assert_eq!(out, [0, 0, 255, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_1x1_pure_blue() {
        let rgba = [0, 0, 255, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(out, [255, 0, 0, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_preserves_green_channel() {
        let rgba = [0, 128, 0, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(out[1], 128);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_preserves_alpha_channel() {
        let rgba = [10, 20, 30, 200];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        assert_eq!(out[3], 200);
    }

    // -------------------------------------------------------------------------
    // frame_duration_hns additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn frame_duration_hns_30fps_is_333333() {
        assert_eq!(frame_duration_hns(30), 333_333);
    }

    #[test]
    fn frame_duration_hns_60fps_is_166666() {
        assert_eq!(frame_duration_hns(60), 166_666);
    }

    #[test]
    fn frame_duration_hns_1fps_is_10_million() {
        assert_eq!(frame_duration_hns(1), 10_000_000);
    }

    #[test]
    fn frame_duration_hns_monotonically_decreasing() {
        let fps_values = [1, 10, 24, 30, 60, 120, 240];
        for pair in fps_values.windows(2) {
            assert!(
                frame_duration_hns(pair[0]) > frame_duration_hns(pair[1]),
                "Expected duration at {}fps > duration at {}fps",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn frame_duration_hns_always_positive_for_nonzero() {
        for fps in 1..=300 {
            assert!(
                frame_duration_hns(fps) > 0,
                "fps={fps} should yield positive duration"
            );
        }
    }

    // -------------------------------------------------------------------------
    // round_to_even additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn round_to_even_all_common_resolutions_unchanged() {
        let even_values = [720, 1080, 1440, 2160, 1920, 2560, 3840];
        for v in even_values {
            assert_eq!(round_to_even(v), v, "{v} should be unchanged");
        }
    }

    #[test]
    fn round_to_even_odd_resolutions_round_up() {
        assert_eq!(round_to_even(719), 720);
        assert_eq!(round_to_even(1079), 1080);
        assert_eq!(round_to_even(1439), 1440);
        assert_eq!(round_to_even(2159), 2160);
    }

    #[test]
    fn round_to_even_100_stays_100() {
        assert_eq!(round_to_even(100), 100);
    }

    #[test]
    fn round_to_even_101_becomes_102() {
        assert_eq!(round_to_even(101), 102);
    }

    // -------------------------------------------------------------------------
    // rgba_to_bgra_bottom_up additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn rgba_to_bgra_bottom_up_empty_input() {
        let data: Vec<u8> = vec![];
        let out = rgba_to_bgra_bottom_up(&data, 0, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn rgba_to_bgra_bottom_up_pure_white_pixel() {
        let rgba = [255, 255, 255, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        // White: R=255,G=255,B=255 → B=255,G=255,R=255 — same
        assert_eq!(out, vec![255, 255, 255, 255]);
    }

    #[test]
    fn rgba_to_bgra_bottom_up_pure_red_becomes_blue_channel() {
        let rgba = [255, 0, 0, 255];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 1);
        // BGRA: B=255, G=0, R=0, A=255
        assert_eq!(out[0], 0); // B (was R=255 → now in position 2)
        assert_eq!(out[2], 255); // R (was B=0 → now has old R)
    }

    #[test]
    fn rgba_to_bgra_bottom_up_2x1_row_not_flipped() {
        // Single row: no vertical flip needed
        let rgba = [
            10, 20, 30, 40, // pixel 0
            50, 60, 70, 80, // pixel 1
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 2, 1);
        // Row order same (single row), but R↔B swapped
        assert_eq!(out[0], 30); // B=old_R
        assert_eq!(out[2], 10); // R=old_B
        assert_eq!(out[4], 70);
        assert_eq!(out[6], 50);
    }

    // -------------------------------------------------------------------------
    // validate_recording_start additional tests
    // -------------------------------------------------------------------------

    #[cfg(windows)]
    #[test]
    fn validate_recording_start_1fps_is_ok() {
        assert!(validate_recording_start(1).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn validate_recording_start_240fps_is_ok() {
        assert!(validate_recording_start(240).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn validate_recording_start_rejects_fps_above_max() {
        let err = validate_recording_start(Fps::MAX + 1).unwrap_err();
        assert!(
            err.contains("240"),
            "error should mention the max FPS: {err}"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn validate_recording_start_nonzero_fps_returns_unsupported_on_linux() {
        let result = validate_recording_start(30);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("only supported on Windows"));
    }

    #[test]
    fn validate_recording_start_zero_error_mentions_fps() {
        let err = validate_recording_start(0).unwrap_err();
        assert!(err.to_lowercase().contains("fps") || err.contains("0"));
    }

    #[test]
    fn validate_recording_start_above_max_error_mentions_limit() {
        let err = validate_recording_start(241).unwrap_err();
        assert!(
            err.contains("240"),
            "error should mention the max FPS: {err}"
        );
    }

    // -------------------------------------------------------------------------
    // pack_u32_pair additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn pack_u32_pair_roundtrip_1920_1080() {
        let packed = pack_u32_pair(1920, 1080);
        let hi = (packed >> 32) as u32;
        let lo = packed as u32;
        assert_eq!(hi, 1920);
        assert_eq!(lo, 1080);
    }

    #[test]
    fn pack_u32_pair_roundtrip_zero_max() {
        let packed = pack_u32_pair(0, u32::MAX);
        let hi = (packed >> 32) as u32;
        let lo = packed as u32;
        assert_eq!(hi, 0);
        assert_eq!(lo, u32::MAX);
    }

    #[test]
    fn pack_u32_pair_roundtrip_max_zero() {
        let packed = pack_u32_pair(u32::MAX, 0);
        let hi = (packed >> 32) as u32;
        let lo = packed as u32;
        assert_eq!(hi, u32::MAX);
        assert_eq!(lo, 0);
    }

    // -------------------------------------------------------------------------
    // get_recording_path / get_recording_dir additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn get_recording_path_has_mp4_extension() {
        let path = get_recording_path();
        assert_eq!(path.extension().unwrap(), "mp4");
    }

    #[test]
    fn get_recording_path_parent_is_recording_dir() {
        let path = get_recording_path();
        let dir = get_recording_dir();
        assert_eq!(path.parent().unwrap(), dir);
    }

    #[test]
    fn get_recording_path_in_dir_custom_dir() {
        let dir = std::path::Path::new("/tmp/my_recordings");
        let path = get_recording_path_in_dir(dir);
        assert_eq!(path.parent().unwrap(), dir);
        assert_eq!(path.extension().unwrap(), "mp4");
    }

    #[test]
    fn get_recording_path_in_dir_stem_starts_with_recording() {
        let dir = std::path::Path::new("/tmp");
        let path = get_recording_path_in_dir(dir);
        let stem = path.file_stem().unwrap().to_str().unwrap();
        assert!(
            stem.starts_with("recording_"),
            "Expected stem starting with 'recording_', got '{stem}'"
        );
    }

    #[test]
    fn get_recording_dir_last_component_is_recordings() {
        let dir = get_recording_dir();
        let last = dir.file_name().unwrap().to_str().unwrap();
        assert_eq!(last, "recordings");
    }

    // -------------------------------------------------------------------------
    // validate_capture_region additional tests
    // -------------------------------------------------------------------------

    #[test]
    fn validate_capture_region_valid_full_frame() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_valid_small_region() {
        let region = Region {
            physical_x: 100,
            physical_y: 100,
            physical_width: 200,
            physical_height: 200,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_region_exceeds_width() {
        let region = Region {
            physical_x: 1800,
            physical_y: 0,
            physical_width: 200,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_region_exceeds_height() {
        let region = Region {
            physical_x: 0,
            physical_y: 1000,
            physical_width: 100,
            physical_height: 200,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_zero_dimensions_rejected() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());

        let region2 = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 100,
            physical_height: 0,
        };
        assert!(validate_capture_region(region2, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_negative_origin_rejected() {
        let region = Region {
            physical_x: -10,
            physical_y: 0,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    // -------------------------------------------------------------------------
    // pack_u32_pair tests
    // -------------------------------------------------------------------------

    #[test]
    fn pack_u32_pair_zero_zero() {
        assert_eq!(pack_u32_pair(0, 0), 0u64);
    }

    #[test]
    fn pack_u32_pair_one_zero() {
        // hi=1 in upper 32 bits: 1 << 32 = 4294967296
        assert_eq!(pack_u32_pair(1, 0), 1u64 << 32);
    }

    #[test]
    fn pack_u32_pair_zero_one() {
        assert_eq!(pack_u32_pair(0, 1), 1u64);
    }

    #[test]
    fn pack_u32_pair_typical_frame_size() {
        // 1920x1080
        let packed = pack_u32_pair(1920, 1080);
        let hi = (packed >> 32) as u32;
        let lo = packed as u32;
        assert_eq!(hi, 1920);
        assert_eq!(lo, 1080);
    }

    #[test]
    fn pack_u32_pair_max_values() {
        let packed = pack_u32_pair(u32::MAX, u32::MAX);
        assert_eq!(packed, u64::MAX);
    }

    // -------------------------------------------------------------------------
    // frame_duration_hns edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn frame_duration_hns_zero_fps_returns_zero() {
        assert_eq!(frame_duration_hns(0), 0);
    }

    #[test]
    fn frame_duration_hns_30_fps() {
        assert_eq!(frame_duration_hns(30), 333333);
    }

    #[test]
    fn frame_duration_hns_60_fps() {
        assert_eq!(frame_duration_hns(60), 166666);
    }

    #[test]
    fn frame_duration_hns_1_fps_equals_10_million() {
        assert_eq!(frame_duration_hns(1), 10_000_000);
    }

    #[test]
    fn frame_duration_hns_is_positive_for_nonzero_fps() {
        for fps in [1, 10, 24, 30, 60, 120, 240] {
            assert!(
                frame_duration_hns(fps) > 0,
                "fps={fps} should give positive duration"
            );
        }
    }

    // -------------------------------------------------------------------------
    // round_to_even tests
    // -------------------------------------------------------------------------

    #[test]
    fn round_to_even_zero() {
        assert_eq!(round_to_even(0), 0);
    }

    #[test]
    fn round_to_even_1_rounds_up_to_2() {
        assert_eq!(round_to_even(1), 2);
    }

    #[test]
    fn round_to_even_2_unchanged() {
        assert_eq!(round_to_even(2), 2);
    }

    #[test]
    fn round_to_even_1079_rounds_to_1080() {
        assert_eq!(round_to_even(1079), 1080);
    }

    #[test]
    fn round_to_even_1920_stays_even() {
        assert_eq!(round_to_even(1920), 1920);
    }

    #[test]
    fn round_to_even_4095_rounds_to_4096() {
        assert_eq!(round_to_even(4095), 4096);
    }

    // -------------------------------------------------------------------------
    // validate_capture_region more edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn validate_capture_region_negative_y_only_is_rejected() {
        let region = Region {
            physical_x: 0,
            physical_y: -1,
            physical_width: 100,
            physical_height: 100,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_1x1_at_origin_ok() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_right_edge_overflow() {
        let region = Region {
            physical_x: 1920,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_bottom_edge_overflow() {
        let region = Region {
            physical_x: 0,
            physical_y: 1080,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    // -------------------------------------------------------------------------
    // validate_capture_region: both negative coordinates
    // -------------------------------------------------------------------------

    #[test]
    fn validate_capture_region_both_coords_negative() {
        let region = Region {
            physical_x: -10,
            physical_y: -20,
            physical_width: 100,
            physical_height: 100,
        };
        let err = validate_capture_region(region, 1920, 1080).unwrap_err();
        assert!(err.contains("within the selected monitor"));
    }

    // -------------------------------------------------------------------------
    // frame_duration_hns: boundary fps values
    // -------------------------------------------------------------------------

    #[test]
    fn frame_duration_hns_3fps() {
        assert_eq!(frame_duration_hns(3), 3_333_333);
    }

    #[test]
    fn frame_duration_hns_5fps() {
        assert_eq!(frame_duration_hns(5), 2_000_000);
    }

    // -------------------------------------------------------------------------
    // round_to_even: additional edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn round_to_even_999_becomes_1000() {
        assert_eq!(round_to_even(999), 1000);
    }

    #[test]
    fn round_to_even_1000_stays_1000() {
        assert_eq!(round_to_even(1000), 1000);
    }

    // -------------------------------------------------------------------------
    // rgba_to_bgra_bottom_up: 1x4 tall image row reversal
    // -------------------------------------------------------------------------

    #[test]
    fn rgba_to_bgra_bottom_up_1x4_rows_fully_reversed() {
        // 1x4 image: 4 rows of distinct colors (1 pixel each)
        let rgba = vec![
            10, 20, 30, 255, // row 0
            40, 50, 60, 255, // row 1
            70, 80, 90, 255, // row 2
            100, 110, 120, 255, // row 3
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 1, 4);
        // Row 3 becomes row 0 (bottom-up), with R↔B swap
        assert_eq!(&out[0..4], &[120, 110, 100, 255]); // was [100,110,120,255]
        assert_eq!(&out[4..8], &[90, 80, 70, 255]); // was [70,80,90,255]
        assert_eq!(&out[8..12], &[60, 50, 40, 255]); // was [40,50,60,255]
        assert_eq!(&out[12..16], &[30, 20, 10, 255]); // was [10,20,30,255]
    }

    // -------------------------------------------------------------------------
    // pack_u32_pair: asymmetric values
    // -------------------------------------------------------------------------

    #[test]
    fn pack_u32_pair_asymmetric_3840x2160() {
        let packed = pack_u32_pair(3840, 2160);
        assert_eq!((packed >> 32) as u32, 3840);
        assert_eq!(packed as u32, 2160);
    }

    // -------------------------------------------------------------------------
    // RecordingSession::output_path() accessor
    // -------------------------------------------------------------------------

    #[test]
    fn recording_session_start_in_dir_zero_fps_error_message() {
        let dir = PathBuf::from("/tmp/test-recordings");
        let result = RecordingSession::start_in_dir(
            0,
            0,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 100,
                physical_height: 100,
            },
            0,
            dir,
        );
        assert!(matches!(
            result,
            Err(ref e) if e == "Recording FPS must be greater than 0"
        ));
    }

    // -------------------------------------------------------------------------
    // Region derive traits
    // -------------------------------------------------------------------------

    #[test]
    fn region_clone_produces_equal_value() {
        let r = Region {
            physical_x: 10,
            physical_y: 20,
            physical_width: 100,
            physical_height: 200,
        };
        let cloned = r;
        assert_eq!(cloned.physical_x, r.physical_x);
        assert_eq!(cloned.physical_y, r.physical_y);
        assert_eq!(cloned.physical_width, r.physical_width);
        assert_eq!(cloned.physical_height, r.physical_height);
    }

    #[test]
    fn region_debug_repr_is_non_empty() {
        let r = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1,
        };
        let debug = format!("{:?}", r);
        assert!(!debug.is_empty());
        assert!(debug.contains("Region"));
    }

    // -------------------------------------------------------------------------
    // get_recording_path / get_recording_path_in_dir consistency
    // -------------------------------------------------------------------------

    #[test]
    fn get_recording_path_uses_get_recording_dir() {
        let path = get_recording_path();
        let dir = get_recording_dir();
        assert_eq!(
            path.parent().unwrap(),
            dir.as_path(),
            "get_recording_path should use get_recording_dir"
        );
    }

    #[test]
    fn get_recording_path_in_dir_extension_is_mp4() {
        let path = get_recording_path_in_dir(Path::new("/tmp"));
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("mp4"));
    }

    // -------------------------------------------------------------------------
    // frame_duration_hns — 2 fps
    // -------------------------------------------------------------------------

    #[test]
    fn frame_duration_hns_2fps_is_5_million() {
        assert_eq!(frame_duration_hns(2), 5_000_000);
    }

    // -------------------------------------------------------------------------
    // round_to_even — additional values
    // -------------------------------------------------------------------------

    #[test]
    fn round_to_even_three_becomes_four() {
        assert_eq!(round_to_even(3), 4);
    }

    #[test]
    fn round_to_even_four_stays_four() {
        assert_eq!(round_to_even(4), 4);
    }

    #[test]
    fn round_to_even_large_even_value_unchanged() {
        // 4294967294 is even
        assert_eq!(round_to_even(4_294_967_294), 4_294_967_294);
    }

    // -------------------------------------------------------------------------
    // rgba_to_bgra_bottom_up — identity for 0-height
    // -------------------------------------------------------------------------

    #[test]
    fn rgba_to_bgra_bottom_up_zero_height_returns_empty() {
        let result = rgba_to_bgra_bottom_up(&[], 100, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn rgba_to_bgra_bottom_up_zero_width_returns_empty() {
        let result = rgba_to_bgra_bottom_up(&[], 0, 100);
        assert!(result.is_empty());
    }

    // -------------------------------------------------------------------------
    // rgba_to_bgra_bottom_up — 2x2 image row reversal
    // -------------------------------------------------------------------------

    #[test]
    fn rgba_to_bgra_bottom_up_2x2_reverses_rows_and_swaps_channels() {
        // Row 0: pixel(R=1,G=2,B=3,A=4), pixel(R=5,G=6,B=7,A=8)
        // Row 1: pixel(R=9,G=10,B=11,A=12), pixel(R=13,G=14,B=15,A=16)
        let rgba = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let bgra = rgba_to_bgra_bottom_up(&rgba, 2, 2);
        // After bottom-up: row 1 becomes row 0, row 0 becomes row 1
        // After R↔B swap:
        // New row 0 (originally row 1): (B=9,G=10,R=11,A=12), (B=13,G=14,R=15,A=16)
        //   → (11,10,9,12), (15,14,13,16)
        // New row 1 (originally row 0): (B=1,G=2,R=3,A=4), (B=5,G=6,R=7,A=8)
        //   → (3,2,1,4), (7,6,5,8)
        assert_eq!(
            bgra,
            vec![11, 10, 9, 12, 15, 14, 13, 16, 3, 2, 1, 4, 7, 6, 5, 8]
        );
    }

    // -------------------------------------------------------------------------
    // validate_capture_region — boundary touching
    // -------------------------------------------------------------------------

    #[test]
    fn validate_capture_region_full_frame_exact_match() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 1920,
            physical_height: 1080,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_1x1_at_last_valid_position() {
        let region = Region {
            physical_x: 1919,
            physical_y: 1079,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_ok());
    }

    #[test]
    fn validate_capture_region_1x1_past_right_edge() {
        let region = Region {
            physical_x: 1920,
            physical_y: 0,
            physical_width: 1,
            physical_height: 1,
        };
        assert!(validate_capture_region(region, 1920, 1080).is_err());
    }

    #[test]
    fn validate_capture_region_error_message_for_negative_origin() {
        let region = Region {
            physical_x: -1,
            physical_y: -1,
            physical_width: 10,
            physical_height: 10,
        };
        let err = validate_capture_region(region, 1920, 1080).unwrap_err();
        assert!(err.contains("within the selected monitor"));
    }

    #[test]
    fn validate_capture_region_error_message_for_zero_dimensions() {
        let region = Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: 0,
            physical_height: 0,
        };
        let err = validate_capture_region(region, 1920, 1080).unwrap_err();
        assert!(err.contains("non-zero size"));
    }

    // -------------------------------------------------------------------------
    // Task 5.3: rgba_to_bgra_bottom_up and pack_u32_pair — requirement coverage
    // Requirements: 9.1, 9.2, 9.4, 9.5, 9.6, 9.7, 9.8
    // -------------------------------------------------------------------------

    /// Req 9.5: alpha channel preserved for every pixel in a multi-row buffer
    /// with varied alpha values (0, 128, 200, 255).
    #[test]
    fn rgba_to_bgra_bottom_up_alpha_preserved_for_every_pixel() {
        // 3x2 image with distinct alpha per pixel
        let rgba = vec![
            10, 20, 30, 0, 40, 50, 60, 128, 70, 80, 90, 200, // row 0
            100, 110, 120, 255, 130, 140, 150, 1, 160, 170, 180, 77, // row 1
        ];
        let out = rgba_to_bgra_bottom_up(&rgba, 3, 2);

        // Collect original alpha values per pixel (row-major order)
        let original_alphas: Vec<u8> = rgba.chunks(4).map(|px| px[3]).collect();
        // After bottom-up flip, row 1 becomes row 0 and vice versa
        // So output pixel order is: row1_px0, row1_px1, row1_px2, row0_px0, row0_px1, row0_px2
        let expected_alphas = vec![
            original_alphas[3],
            original_alphas[4],
            original_alphas[5], // row 1 pixels
            original_alphas[0],
            original_alphas[1],
            original_alphas[2], // row 0 pixels
        ];

        let output_alphas: Vec<u8> = out.chunks(4).map(|px| px[3]).collect();
        assert_eq!(
            output_alphas, expected_alphas,
            "alpha channel must be preserved for every pixel after conversion"
        );
    }

    // -------------------------------------------------------------------------
    // Property-based tests
    // -------------------------------------------------------------------------

    use proptest::prelude::*;

    proptest! {
        /// **Validates: Requirements 17.3, 7.8, 7.9, 7.11**
        ///
        /// For all even u32 values, round_to_even returns the value unchanged.
        /// For all odd u32 values (excluding u32::MAX which overflows), round_to_even
        /// returns value + 1.
        #[test]
        fn prop_round_to_even_correctness(value in any::<u32>()) {
            if value % 2 == 0 {
                // Even values are returned unchanged
                prop_assert_eq!(
                    round_to_even(value), value,
                    "round_to_even({}) should return {} unchanged (even input)",
                    value, value
                );
            } else if value < u32::MAX {
                // Odd values (excluding u32::MAX to avoid overflow) round up by 1
                prop_assert_eq!(
                    round_to_even(value), value + 1,
                    "round_to_even({}) should return {} (odd input rounds up)",
                    value, value + 1
                );
            }
            // u32::MAX is odd and would overflow on +1, so we skip it
        }

        /// **Validates: Requirements 17.4, 9.3**
        ///
        /// For all valid RGBA buffers of dimensions (w, h), applying
        /// `rgba_to_bgra_bottom_up` twice produces the original buffer.
        #[test]
        fn prop_rgba_to_bgra_bottom_up_roundtrip(
            w in 1u32..=16,
            h in 1u32..=16,
            pixel_data in proptest::collection::vec(any::<u8>(), 0..=1024),
        ) {
            let buf_len = (w as usize) * (h as usize) * 4;
            // Build a buffer of exactly w*h*4 bytes, padding or truncating as needed
            let mut buf = pixel_data;
            buf.resize(buf_len, 0u8);

            let once = rgba_to_bgra_bottom_up(&buf, w, h);
            let twice = rgba_to_bgra_bottom_up(&once, w, h);

            prop_assert_eq!(
                twice, buf,
                "applying rgba_to_bgra_bottom_up twice should produce the original buffer \
                 (w={}, h={}, buf_len={})",
                w, h, buf_len
            );
        }
    }
}
