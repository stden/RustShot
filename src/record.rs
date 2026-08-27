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
#[cfg(windows)]
pub(crate) mod capture;
mod domain;
mod encoder;
mod frame;
#[cfg(test)]
mod tests;

use crate::selection::Region;
use crate::settings::Fps;
pub use bench::*;
pub(crate) use domain::{Bitrate, FrameIndex, FrameTiming, RecordingDimensions};
#[cfg(test)]
pub(crate) use domain::{Height, Width};
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
        let join = thread::spawn(move || {
            record_loop(
                path_clone, screen_x, screen_y, region, dimensions, fps, stop_rx,
            )
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
            .unwrap_or_else(|| Ok(self.output_path.clone()))
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

/// Fetch a cached worker handle, starting one on first use and again after a
/// failed start.
///
/// The slot is a `Mutex<Option<_>>` rather than a `OnceLock<Result<_, _>>`
/// precisely so that a failure is *not* remembered: with a `OnceLock`, one
/// transient "failed to start thread" would be cached and every later call would
/// replay that same error, disabling the worker for the rest of the process
/// lifetime. Here the next caller simply tries again.
///
/// A poisoned lock is recovered rather than propagated. The guarded value is
/// just a channel handle, so a panic elsewhere leaves nothing inconsistent, and
/// refusing to hand it out would turn one unrelated panic into a permanently
/// broken worker — the very failure mode this function exists to avoid.
pub(crate) fn cached_worker<T, Start>(
    slot: &std::sync::Mutex<Option<T>>,
    start: Start,
) -> Result<T, String>
where
    T: Clone,
    Start: FnOnce() -> Result<T, String>,
{
    let mut cached = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(handle) = cached.as_ref() {
        return Ok(handle.clone());
    }

    let handle = start()?;
    *cached = Some(handle.clone());
    Ok(handle)
}

/// Drop the cached worker handle so the next [`cached_worker`] call starts a new
/// one. Called when the worker is found to have died.
pub(crate) fn forget_cached_worker<T>(slot: &std::sync::Mutex<Option<T>>) {
    *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
}

/// What a capture worker should do next.
pub(crate) enum WorkerStep<T> {
    /// Serve this request.
    Request(T),
    /// Nothing arrived within the idle timeout — a good moment to release
    /// whatever the worker was holding on to between requests.
    Idle,
    /// Every sender is gone; the worker has nothing left to serve.
    Stop,
}

/// Wait for the next worker request, reporting a quiet spell as [`WorkerStep::Idle`]
/// rather than as shutdown.
///
/// Separated from the worker loop so the distinction is unit-tested: a worker
/// that treated the timeout as "stop" would exit after the first idle period,
/// and the handle cached by [`cached_worker`] would then hand every later caller
/// a dead channel.
pub(crate) fn next_worker_step<T>(
    receiver: &mpsc::Receiver<T>,
    idle_timeout: std::time::Duration,
) -> WorkerStep<T> {
    match receiver.recv_timeout(idle_timeout) {
        Ok(request) => WorkerStep::Request(request),
        Err(mpsc::RecvTimeoutError::Timeout) => WorkerStep::Idle,
        Err(mpsc::RecvTimeoutError::Disconnected) => WorkerStep::Stop,
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
#[allow(dead_code)]
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

/// Round a dimension up to the nearest even number, saturating at `u32::MAX - 1`.
///
/// H.264 encoders require even width and height. Odd values round up by 1, with
/// exactly one exception: `u32::MAX`.
///
/// Algorithm:
/// 1. Add 1 to the input, saturating.
/// 2. Clear the least-significant bit.
/// 3. Even values remain unchanged; odd values below `u32::MAX` round up to the
///    next even, and `u32::MAX` saturates one below itself (see below).
///
/// This bit trick is branch-free and cheap, which is useful in sizing code
/// that may run repeatedly during capture setup.
///
/// # The `u32::MAX` exception
///
/// `u32::MAX` is odd and the next even value, 2^32, does not fit in a `u32`, so
/// step 1 saturates back to `u32::MAX` and step 2 yields `u32::MAX - 1` — one
/// *below* the input. This function therefore does **not** guarantee
/// `result >= value`, and no infallible `u32 -> u32` function can: the remaining
/// options at that input are returning the odd `u32::MAX` (not even, so useless
/// to the encoder) or panicking.
///
/// Use [`checked_round_to_even`] wherever `result >= value` must hold. It rejects
/// `u32::MAX` with an error instead of handing back a smaller size, and it is the
/// form the recording pipeline uses — every recorded dimension is built through
/// `RecordingDimensions`, which rounds with the checked helper, so the saturating
/// case is unreachable in production. The infallible form stays public for the
/// branch-free benchmark and for callers whose input is already bounded.
///
/// Possible improvements:
/// - Nothing outstanding. The two ideas previously listed here — "switch to
///   `saturating_add`" and "return a checked error instead" — had both already
///   been implemented (the add saturates; [`checked_round_to_even`] exists), and
///   leaving them standing as future work is how this doc came to describe a
///   plain `+ 1` that rounds every input up.
///
/// # Examples
///
/// ```
/// use rustshot::record::round_to_even;
///
/// assert_eq!(round_to_even(1920), 1920); // already even
/// assert_eq!(round_to_even(1921), 1922); // rounded up
/// assert_eq!(round_to_even(0), 0);
/// // The single input that comes back SMALLER: 2^32 is not representable.
/// assert_eq!(round_to_even(u32::MAX), u32::MAX - 1);
/// ```
#[allow(dead_code)]
pub fn round_to_even(value: u32) -> u32 {
    value.saturating_add(1) & !1
}

/// Round a dimension up to the nearest even number, or reject it.
///
/// The guaranteed form of [`round_to_even`]: every `Ok` value is even and
/// `>= value`. `u32::MAX` is the only rejected input, because the even value it
/// would need is 2^32; returning `u32::MAX - 1` there — which is what the
/// infallible helper does — would encode a frame one row or column short of the
/// request with nothing reported. Failing at the boundary keeps that impossible
/// geometry visible.
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
/// Allocates the destination and delegates to [`rgba_to_bgra_bottom_up_into`],
/// which flips and swizzles each row in one pass through the crate-private
/// `PixelSwizzle` (plain backticks, not a link: it is `pub(crate)`, and rustdoc
/// runs with `-D warnings`).
///
/// No production path calls this. Every caller that matters supplies its own
/// buffer - the screenshot worker converts in place via
/// [`rgba_to_bgra_bottom_up_in_place`], and the recorder crops through
/// `frame::copy_bgra_region_bottom_up` - so this allocating form survives for
/// benchmarks, the BDD scenarios and doc examples. Prefer either of the other two
/// in new code.
///
/// Possible improvements:
/// - Nothing outstanding. This doc used to describe a two-pass "memcpy rows, then
///   swap R/B in place" algorithm and list "fuse the row flip and channel swap"
///   plus "reuse a caller-provided scratch buffer" as future work. All three had
///   already happened: the pass is fused, `_into` takes the caller's buffer and
///   `_in_place` needs no second one. Stale "possible improvements" are how
///   [`round_to_even`]'s doc came to promise a contract the code did not honour.
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
///
/// No production path calls this any more -- `rgba_to_bgra_bottom_up_in_place` is what
/// the screenshot worker uses -- and it is `pub` and benchmarked anyway for three
/// reasons that are each easy to miss while deleting it. Two `recording.feature`
/// scenarios drive it through a BDD step in `tests/bdd.rs`, so it is specified
/// behaviour rather than spare code. `pixels.rs`'s
/// `rgba_to_bgra_bottom_up_in_place_matches_the_two_buffer_conversion` uses it as the
/// ORACLE the in-place form is checked against, so removing it removes the only thing
/// proving the two agree byte for byte. And its `PixelSwizzle::convert` kernel is a
/// different function from `convert_in_place`'s (`swap_red_blue_into_avx2` versus
/// `bgra_to_rgba_avx2`), live in `capture::platform::rgba_from_bgra_rows` and in the
/// GDI recording fallback, which is why the performance gate still benchmarks this
/// signature: it has no other way to reach that kernel.
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
    // Resolved once for the frame, not once per row.
    let swizzle = crate::capture::PixelSwizzle::detect();

    for y in 0..height {
        let src_start = y * row_bytes;
        let dst_start = (height - 1 - y) * row_bytes;
        let src_row = &rgba[src_start..src_start + row_bytes];
        let dst_row = &mut out[dst_start..dst_start + row_bytes];

        swizzle.convert(src_row, dst_row);
    }
    Ok(())
}

/// The same flip-and-swizzle as [`rgba_to_bgra_bottom_up_into`], performed inside
/// a single buffer.
///
/// The flip pairs row `y` with row `height - 1 - y` and the swizzle only trades
/// red for blue, so the transform is its own inverse and needs no destination:
/// swapping a row pair and swizzling both halves converts both rows at once. That
/// is also why the screenshot path may run it in the opposite direction --
/// bottom-up BGRA in, top-down RGBA out -- and get byte-identical results.
///
/// Deleting the destination is the point. `capture_monitor_image_on_worker` used
/// to hand [`rgba_to_bgra_bottom_up_into`] a fresh `vec![0; width * height * 4]`,
/// ~33 MB for one 4K monitor, which the conversion then overwrote byte for byte.
/// The zero-fill was pure waste, and first-touching those ~8000 fresh pages costs
/// milliseconds of the hotkey-to-overlay budget this module keeps under 100 ms.
/// Peak footprint per screenshot halves too, from two monitor-sized buffers to one.
///
/// An odd `height` leaves the middle row without a partner, so it is swizzled
/// exactly once, on its own. Swizzling it twice is the same as skipping it -- the
/// swizzle is an involution -- and either mistake leaves a single scanline in the
/// middle of an otherwise perfect screenshot with red and blue crossed, which no
/// check on buffer length or corner pixels would notice.
pub fn rgba_to_bgra_bottom_up_in_place(
    pixels: &mut [u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    let required = checked_rgba_len(width, height)?;
    if pixels.len() < required {
        return Err(format!(
            "Pixel buffer is too short: expected {required} bytes, got {}",
            pixels.len()
        ));
    }
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Pixel row is too large".to_string())?;
    let height = height as usize;
    // Resolved once for the frame, not once per row.
    let swizzle = crate::capture::PixelSwizzle::detect();

    // Split into three disjoint pieces so the borrow checker can see that a row
    // and its partner never alias: the top half, the unpaired middle row, and the
    // bottom half walked backwards. `middle` is empty for an even height, which is
    // what keeps the odd case from needing a branch that could count it twice.
    let pairs = height / 2;
    let (top, rest) = pixels[..required].split_at_mut(pairs * row_bytes);
    let (middle, bottom) = rest.split_at_mut((height % 2) * row_bytes);

    // Indexed rather than `chunks_exact_mut`: a zero-width frame makes `row_bytes`
    // zero, and a zero chunk size panics, where the two-buffer form converts such
    // a frame to nothing at all.
    for y in 0..pairs {
        let top_row = &mut top[y * row_bytes..(y + 1) * row_bytes];
        let bottom_row = &mut bottom[(pairs - 1 - y) * row_bytes..(pairs - y) * row_bytes];
        // Move the bytes first, then swizzle each row where it has landed.
        // `PixelSwizzle::convert` cannot do this in one pass without scratch
        // space: writing either flipped row would destroy its partner's source.
        top_row.swap_with_slice(bottom_row);
        swizzle.convert_in_place(top_row);
        swizzle.convert_in_place(bottom_row);
    }
    // Empty for an even height; the frame's only row when `height` is 1.
    swizzle.convert_in_place(middle);
    Ok(())
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
        source,
        source_row_pitch,
        width,
        height,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        width,
        height,
        peak_luminance_nits,
        output,
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
        source,
        source_row_pitch,
        width,
        height,
        Region {
            physical_x: 0,
            physical_y: 0,
            physical_width: width,
            physical_height: height,
        },
        width,
        height,
        output,
    )
}

#[cfg(windows)]
fn record_loop(
    output_path: PathBuf,
    screen_x: i32,
    screen_y: i32,
    region: Region,
    dimensions: RecordingDimensions,
    fps: Fps,
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
    unsafe {
        wmf_record(
            &output_path,
            screen_x,
            screen_y,
            region,
            dimensions,
            fps,
            stop_rx,
        )
    }
}

#[cfg(not(windows))]
fn record_loop(
    _output_path: PathBuf,
    _screen_x: i32,
    _screen_y: i32,
    _region: Region,
    _dimensions: RecordingDimensions,
    _fps: Fps,
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
