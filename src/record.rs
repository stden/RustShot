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
//!   3. Captures frames through one persistent Windows Graphics Capture
//!      session (legacy GDI fallback), crops/pads them into Media Foundation's
//!      bottom-up BGRA layout, and writes them as samples.
//!   4. Stops when requested explicitly or when the owning session is dropped.
//! - [`RecordingSession::stop`] sends the stop signal and joins the thread.
//!
//! Helper functions extracted for testability:
//! - [`frame_duration_hns`] — frame duration in 100ns units.
//! - [`round_to_even`] — H.264 requires even dimensions.
//! - [`get_recording_dir`] / [`get_recording_path`] — output path logic.

#[cfg(windows)]
pub(crate) mod capture;
mod encoder;
mod frame;
#[cfg(test)]
mod tests;

use crate::selection::Region;
use crate::settings::Fps;
// The frame helpers are consumed by the cfg(windows) recording loop only.
#[cfg_attr(not(windows), allow(unused_imports))]
use frame::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

#[cfg(windows)]
use self::encoder::wmf_record;

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
        validate_recording_region_dimensions(region)?;
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
        self.request_stop();
        self.join_recording_thread()
    }

    fn request_stop(&self) {
        let _ = self.stop_tx.try_send(());
    }

    fn join_recording_thread(&mut self) -> Result<PathBuf, String> {
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

impl Drop for RecordingSession {
    fn drop(&mut self) {
        if self.join.is_none() {
            return;
        }
        self.request_stop();
        if let Err(error) = self.join_recording_thread() {
            log::error!("Failed to stop recording while dropping its session: {error}");
        }
    }
}

fn recording_stop_requested(stop_rx: &mpsc::Receiver<()>) -> bool {
    matches!(
        stop_rx.try_recv(),
        Ok(()) | Err(mpsc::TryRecvError::Disconnected)
    )
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

/// Return an exact fixed-rate timestamp and duration without cumulative drift.
/// Which frame index the wall clock says we should be on after `elapsed`.
///
/// `floor(elapsed * fps / 1s)`, saturating at `i64::MAX`. Anchoring the
/// recording timeline to the wall clock (instead of counting emitted frames)
/// lets the loop skip ahead when the encoder falls behind rather than
/// accumulating drift. `fps == 0` yields frame 0 - the validation layer
/// rejects it before recording starts, so this is a belt-and-suspenders guard.
#[allow(dead_code)] // wiring lands with the wall-clock-anchored recording loop
pub(crate) fn frame_index_for_elapsed(elapsed: std::time::Duration, fps: u32) -> i64 {
    if fps == 0 {
        return 0;
    }
    let index = elapsed.as_nanos().saturating_mul(u128::from(fps)) / 1_000_000_000;
    i64::try_from(index).unwrap_or(i64::MAX)
}

pub(crate) fn frame_timing_hns(frame_index: i64, fps: u32) -> (i64, i64) {
    if frame_index < 0 || fps == 0 {
        return (0, 0);
    }
    let start = i128::from(frame_index) * 10_000_000 / i128::from(fps);
    let end = i128::from(frame_index + 1) * 10_000_000 / i128::from(fps);
    (start as i64, (end - start) as i64)
}

/// Choose a screen-recording bitrate from pixel throughput.
pub(crate) fn recording_bitrate(width: u32, height: u32, fps: u32) -> u32 {
    let bits_per_second = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(u64::from(fps))
        .saturating_mul(12)
        / 100;
    bits_per_second.clamp(2_000_000, 50_000_000) as u32
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
    value.saturating_add(1) & !1
}

pub fn checked_round_to_even(value: u32) -> Result<u32, String> {
    if value == u32::MAX {
        return Err("Recording dimension is too large to round to an even value".to_string());
    }
    Ok(round_to_even(value))
}

fn checked_rgba_len(width: u32, height: u32) -> Result<usize, String> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Pixel buffer dimensions are too large".to_string())
}

fn validate_recording_region_dimensions(region: Region) -> Result<(), String> {
    if region.physical_width == 0 || region.physical_height == 0 {
        return Err("Recording region must have non-zero size".to_string());
    }
    checked_round_to_even(region.physical_width)?;
    checked_round_to_even(region.physical_height)?;
    checked_rgba_len(
        checked_round_to_even(region.physical_width)?,
        checked_round_to_even(region.physical_height)?,
    )?;
    Ok(())
}

#[cfg_attr(not(windows), allow(dead_code))]
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
/// let bgra = rgba_to_bgra_bottom_up(&rgba, 2, 1).unwrap();
/// // Single row, so bottom-up is the same order; R↔B swapped:
/// assert_eq!(bgra, vec![30, 20, 10, 255, 60, 50, 40, 255]);
/// ```
#[allow(dead_code)]
pub fn rgba_to_bgra_bottom_up(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let required = checked_rgba_len(width, height)?;
    let mut out = vec![0u8; required];
    rgba_to_bgra_bottom_up_into(rgba, &mut out, width, height)?;
    Ok(out)
}

/// Optimized RGBA to BGRA bottom-up conversion.
///
/// Writes the converted pixels into the provided `out` buffer.
/// Returns an error when either buffer is shorter than `width * height * 4`
/// or when the dimension calculation overflows.
pub fn rgba_to_bgra_bottom_up_into(
    rgba: &[u8],
    out: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let required = checked_rgba_len(width, height)?;
    if rgba.len() < required {
        return Err(format!(
            "RGBA input buffer is too short: expected {required} bytes, got {}",
            rgba.len()
        ));
    }
    if out.len() < required {
        return Err(format!(
            "BGRA output buffer is too short: expected {required} bytes, got {}",
            out.len()
        ));
    }
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Pixel row is too large".to_string())?;
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
    Ok(())
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
