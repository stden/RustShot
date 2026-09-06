#![expect(
    clippy::needless_pass_by_value,
    reason = "recording resources transfer to worker threads"
)]
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
//! - The background thread runs `record_loop` → `wmf_record`, which:
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
//! - [`round_to_even`] / [`checked_round_to_even`] — H.264 requires even
//!   dimensions. The pipeline uses the checked form; the infallible one cannot
//!   round `u32::MAX` up (2^32 is unrepresentable) and returns `u32::MAX - 1`.
//! - [`rgba_to_bgra_bottom_up_into`] / [`rgba_to_bgra_bottom_up_in_place`] —
//!   the flip-and-swizzle between capture and encoder layouts. The screenshot
//!   path uses the in-place form: one monitor-sized buffer, not two.
//! - [`get_recording_dir`] / [`get_recording_path`] — output path logic.

mod bench;
mod domain;
mod encoder;
use crate::pixels::frame;
#[cfg(test)]
#[path = "../tests/unit/record/mod.rs"]
mod tests;

use crate::domain::{Fps, Region};
#[cfg(test)]
use crate::pixels::validate_capture_region;
pub use crate::pixels::{
    rgba_to_bgra_bottom_up, rgba_to_bgra_bottom_up_in_place, rgba_to_bgra_bottom_up_into,
};
#[cfg(test)]
use crate::platform::worker::{cached_worker, forget_cached_worker, next_worker_step, WorkerStep};
pub use bench::*;
pub use domain::{checked_round_to_even, round_to_even};
pub(crate) use domain::{Bitrate, FrameIndex, FrameTiming, RecordingDimensions};
#[cfg(test)]
pub(crate) use domain::{Height, Width};
// Existing unit suites exercise the shared transforms through this facade.
#[cfg(test)]
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

/// Validated source geometry and encoding policy passed to the recording worker.
#[cfg_attr(
    not(windows),
    allow(
        dead_code,
        reason = "Non-Windows builds reject recording before consuming the shared worker parameters."
    )
)]
struct RecordingParameters {
    monitor_origin: crate::domain::LogicalPoint,
    region: Region,
    dimensions: RecordingDimensions,
    fps: Fps,
}

impl RecordingSession {
    /// Start recording the given screen region at the specified frame rate.
    ///
    /// `screen_x`/`screen_y` identify the monitor by its display origin.
    /// `region` is the capture area within that monitor (screen-local coords).
    /// Returns immediately; encoding runs on a background thread.
    pub fn start(screen_x: i32, screen_y: i32, region: Region, fps: Fps) -> Result<Self, String> {
        Self::start_in_dir(screen_x, screen_y, region, fps, get_recording_dir())
    }

    /// Start recording and save the MP4 under the provided recordings directory.
    pub fn start_in_dir(
        screen_x: i32,
        screen_y: i32,
        region: Region,
        fps: Fps,
        recordings_dir: PathBuf,
    ) -> Result<Self, String> {
        validate_recording_platform()?;
        let dimensions = RecordingDimensions::try_from(region)?;
        let output_path = get_recording_path_in_dir(&recordings_dir);
        let path_clone = output_path.clone();
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let parameters = RecordingParameters {
            monitor_origin: crate::domain::LogicalPoint::new(screen_x, screen_y),
            region,
            dimensions,
            fps,
        };
        let join = thread::spawn(move || record_loop(path_clone, parameters, stop_rx));
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
            .unwrap_or_else(|| Ok(self.output_path.clone()))
    }

    /// Returns a reference to the output MP4 file path.
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
pub fn get_recording_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RustShot")
        .join("recordings")
}

/// Returns a timestamped MP4 path inside [`get_recording_dir`].
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
    let _fps = Fps::try_from(fps)?;
    validate_recording_platform()
}

fn validate_recording_platform() -> Result<(), String> {
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
pub fn frame_duration_hns(fps: u32) -> i64 {
    if fps == 0 {
        return 0;
    }
    10_000_000i64 / fps as i64
}

/// Which frame index the wall clock says we should be on after `elapsed`.
///
/// `floor(elapsed * fps / 1s)`, saturating at `i64::MAX`. Anchoring the
/// recording timeline to the wall clock (instead of counting emitted frames)
/// lets the loop skip ahead when the encoder falls behind rather than
/// accumulating drift. The validated [`Fps`] type makes a zero rate
/// unrepresentable before this calculation begins.
pub(crate) fn frame_index_for_elapsed(elapsed: std::time::Duration, fps: Fps) -> FrameIndex {
    let index = elapsed.as_nanos().saturating_mul(u128::from(fps.get())) / 1_000_000_000;
    FrameIndex::saturating_from_u128(index)
}

/// The index of the next frame to emit, `elapsed` after the timeline started.
///
/// Normally that is simply one past the last written frame. When capture and
/// encoding cost more than a frame interval — a 4K HDR region tone-maps 8.3
/// million pixels per frame — the wall clock has moved further, and the timeline
/// jumps to where it actually is. Counting emitted frames instead would hand the
/// muxer fewer timestamps than the real elapsed time, and the recording would
/// play back sped up rather than showing a lower frame rate.
///
/// It never moves backwards: a loop that ran ahead of the clock still gets a
/// strictly increasing timestamp.
pub(crate) fn next_frame_index(
    written: FrameIndex,
    elapsed: std::time::Duration,
    fps: Fps,
) -> FrameIndex {
    written.next().max(frame_index_for_elapsed(elapsed, fps))
}

pub(crate) fn frame_timing_hns(frame_index: FrameIndex, fps: Fps) -> FrameTiming {
    let fps = i128::from(fps.get());
    let start = i128::from(frame_index.get()) * 10_000_000 / fps;
    let end = i128::from(frame_index.next().get()) * 10_000_000 / fps;
    FrameTiming::new(start as i64, (end - start) as i64)
}

/// Choose a screen-recording bitrate from pixel throughput.
pub(crate) fn recording_bitrate(dimensions: RecordingDimensions, fps: Fps) -> Bitrate {
    let bits_per_second = u64::from(dimensions.width().get())
        .saturating_mul(u64::from(dimensions.height().get()))
        .saturating_mul(u64::from(fps.get()))
        .saturating_mul(12)
        / 100;
    Bitrate::new(bits_per_second.clamp(2_000_000, 50_000_000) as u32)
}

/// Expose the production FP16 scRGB tone-mapping frame copy for benchmarks.
///
/// This is the heaviest per-pixel path in the recorder: an HDR 4K frame decodes
/// three halves, tone-maps and looks up three channels for each of 8.3 million
/// pixels, 30 times a second. The plain BGRA copy below is a memcpy by
/// comparison, so it is this function that decides whether HDR recording keeps
/// up with its frame interval.
#[doc(hidden)]
pub fn benchmark_copy_scrgb_frame_exact_fit(
    source: &[u8],
    width: u32,
    height: u32,
    peak_luminance_nits: f32,
    output: &mut [u8],
) -> Result<(), String> {
    let source_row_pitch = (width as usize)
        .checked_mul(8)
        .ok_or_else(|| "Captured FP16 frame row is too large".to_string())?;
    frame::copy_scrgb_region_bottom_up(
        frame::FrameView::new(
            source,
            crate::domain::PhysicalSize::new(width, height),
            source_row_pitch,
        ),
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        peak_luminance_nits,
        frame::BgraFrameMut::new(output, crate::domain::PhysicalSize::new(width, height)),
    )
}

/// Expose the production exact-fit BGRA frame copy for benchmarks.
#[doc(hidden)]
pub fn benchmark_copy_bgra_frame_exact_fit(
    source: &[u8],
    width: u32,
    height: u32,
    output: &mut [u8],
) -> Result<(), String> {
    let source_row_pitch = width
        .checked_mul(4)
        .ok_or_else(|| "Captured frame row is too large".to_string())?
        as usize;
    frame::copy_bgra_region_bottom_up(
        frame::FrameView::new(
            source,
            crate::domain::PhysicalSize::new(width, height),
            source_row_pitch,
        ),
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        frame::BgraFrameMut::new(output, crate::domain::PhysicalSize::new(width, height)),
    )
}

/// Expose the production pitched BGRA region copy for benchmarks.
///
/// The exact-fit adapter above cannot measure the recorder's common selection
/// path: a smaller rectangle is cropped from a monitor-sized source whose row
/// pitch may exceed its visible byte width. Keeping this adapter on
/// `copy_bgra_region_bottom_up` prevents the benchmark from drifting into a
/// lookalike implementation that skips the production validation and row flip.
#[doc(hidden)]
#[expect(
    clippy::too_many_arguments,
    reason = "Retain the public pitched-copy benchmark adapter signature; production copies use borrowed frame descriptors."
)]
pub fn benchmark_copy_bgra_region(
    source: &[u8],
    source_row_pitch: usize,
    source_size: (u32, u32),
    region: (i32, i32, u32, u32),
    output_size: (u32, u32),
    output: &mut [u8],
) -> Result<(), String> {
    frame::copy_bgra_region_bottom_up(
        frame::FrameView::new(source, source_size.into(), source_row_pitch),
        Region {
            physical_x: region.0,
            physical_y: region.1,
            physical_width: region.2,
            physical_height: region.3,
        },
        frame::BgraFrameMut::new(output, output_size.into()),
    )
}

#[cfg(windows)]
fn record_loop(
    output_path: PathBuf,
    parameters: RecordingParameters,
    stop_rx: mpsc::Receiver<()>,
) -> Result<PathBuf, String> {
    use std::fs;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create recording dir: {}", e))?;
    }
    // SAFETY: `wmf_record` requires a thread on which COM has not been
    // initialised yet, because it initialises and uninitialises COM itself via
    // `CoGuard`. That holds: this function is the entry point of the thread
    // `RecordingSession::start_in_dir` spawns, and the only thing it does before
    // this call is create the output directory - no COM call has run here. The
    // thread also does nothing after `wmf_record` returns, so every COM object it
    // created is released before its `CoUninitialize`.
    unsafe { wmf_record(&output_path, parameters, stop_rx) }
}

#[cfg(not(windows))]
fn record_loop(
    _output_path: PathBuf,
    _parameters: RecordingParameters,
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
pub fn pack_u32_pair(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | (lo as u64)
}
