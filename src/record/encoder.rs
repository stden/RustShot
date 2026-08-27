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
    /// # Safety
    ///
    /// COM's initialisation count is per *thread*, so the returned guard must be
    /// dropped on the thread that created it — it is a unit struct and therefore
    /// `Send`, which the type system cannot use to enforce that. Every COM
    /// interface pointer obtained while the guard is alive must also be released
    /// before it drops: `CoUninitialize` tears the apartment down, and using a
    /// pointer afterwards is undefined.
    unsafe fn init() -> Result<Self, String> {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

        // SAFETY: the reserved first parameter is `None`, so no pointer is passed
        // and the call only changes this thread's apartment state. The `?` below
        // is what keeps `Drop` balanced: on a failing HRESULT — including
        // `RPC_E_CHANGED_MODE`, where COM was left untouched because the thread
        // already had a different apartment model — no `Self` is constructed, so
        // no unpaired `CoUninitialize` can follow.
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|e| format!("CoInitializeEx failed: {e}"))?;
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for CoGuard {
    fn drop(&mut self) {
        // SAFETY: this balances exactly one successful `CoInitializeEx`. `Self` is
        // only ever constructed by `init()` after that call returned success, the
        // type is neither `Clone` nor `Copy`, so `drop` runs once per
        // initialisation. Its remaining precondition — same thread, no live COM
        // pointers — is the contract `init()` documents; `wmf_record`, the only
        // constructor site, holds the guard in a local that never leaves the
        // recording thread and releases every MF object in an inner scope first.
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
    /// # Safety
    ///
    /// Media Foundation keeps a startup count, and the returned guard's `Drop`
    /// spends one unit of it. So the caller must keep the guard alive for as long
    /// as it uses any MF API, and must release every MF object it created before
    /// dropping it: `MFShutdown` invalidates them, and MF objects also need COM,
    /// so the guard has to be dropped before the [`CoGuard`] it sits under.
    unsafe fn startup() -> Result<Self, String> {
        use windows::Win32::Media::MediaFoundation::{MFStartup, MFSTARTUP_NOSOCKET, MF_VERSION};

        // SAFETY: neither argument is a pointer — `MF_VERSION` is the version
        // constant this build of windows-rs was generated against, and
        // `MFSTARTUP_NOSOCKET` skips the network sockets a local recorder has no
        // use for. As in `CoGuard::init`, the `?` means a failed startup produces
        // no `Self`, so `Drop` cannot call an unpaired `MFShutdown`.
        unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) }
            .map_err(|e| format!("MFStartup failed: {e}"))?;
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for MfGuard {
    fn drop(&mut self) {
        // SAFETY: this balances exactly one successful `MFStartup`: `Self` is only
        // constructed by `startup()` on success, and the type is neither `Clone`
        // nor `Copy`. In `wmf_record` the guard is declared after `CoGuard`, so
        // drop order shuts MF down while COM is still initialised, and the sink
        // writer and media types were already released with the inner closure.
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
    dimensions: RecordingDimensions,
    fps: Fps,
    stop_rx: mpsc::Receiver<()>,
) -> Result<PathBuf, String> {
    use windows::{core::PCWSTR, Win32::Media::MediaFoundation::*};

    // SAFETY: both guards' contracts are discharged by this function's own
    // `# Safety` section plus its structure. COM is untouched on this thread when
    // we are called, both guards stay in locals that never leave the thread, and
    // every COM/MF object below is created inside the closure and released when it
    // returns — before either guard drops. Declaration order matters: `_mf` drops
    // first, so `MFShutdown` runs while COM is still initialised.
    let _co = unsafe { CoGuard::init() }?;
    // SAFETY: as above; MF startup happens with COM already initialised.
    let _mf = unsafe { MfGuard::startup() }?;

    // Release all Media Foundation COM objects before shutting down MF/COM.
    let result = (|| -> Result<PathBuf, String> {
        let width = dimensions.width().get();
        let height = dimensions.height().get();
        let mut frame_source = RecordingFrameSource::new(screen_x, screen_y, region)?;

        let path_wide = crate::settings::wide(&output_path.to_string_lossy());
        let pcwstr = PCWSTR(path_wide.as_ptr());

        // SAFETY: `pcwstr` borrows `path_wide`, a NUL-terminated UTF-16 vector
        // built by `settings::wide` that lives until the end of this closure, so
        // the string stays valid for the call. The two optional arguments are
        // `None`, so no other pointer is passed - which is also why no async
        // callback exists, the fact `write_video_sample`'s doc turns on. MF is
        // started, per `_mf` above.
        let sink_writer: IMFSinkWriter =
            unsafe { MFCreateSinkWriterFromURL(pcwstr, None, None) }
                .map_err(|e| format!("MFCreateSinkWriterFromURL failed: {e}"))?;

        // Output type: H264
        // SAFETY: `MFCreateMediaType` takes no arguments and only allocates; MF is
        // started for this thread by `_mf`.
        let out_video: IMFMediaType =
            unsafe { MFCreateMediaType() }.map_err(|e| format!("MFCreateMediaType: {e}"))?;
        // SAFETY: every call in this block is a method on the `out_video` interface
        // pointer just created, so the COM object is alive throughout, and MF is
        // started. The `SetGUID` arguments are references to `static` GUID
        // constants, valid for `'static`; the `SetUINT32`/`SetUINT64` arguments are
        // plain integers, so no pointer this block passes can dangle.
        unsafe {
            out_video
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(|e| format!("out SetGUID major: {e}"))?;
            out_video
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)
                .map_err(|e| format!("out SetGUID H264: {e}"))?;
            out_video
                .SetUINT32(&MF_MT_AVG_BITRATE, recording_bitrate(dimensions, fps).get())
                .map_err(|e| format!("out SetUINT32 bitrate: {e}"))?;
            out_video
                .SetUINT64(&MF_MT_FRAME_SIZE, pack_u32_pair(width, height))
                .map_err(|e| format!("out SetUINT64 frame size: {e}"))?;
            out_video
                .SetUINT64(&MF_MT_FRAME_RATE, pack_u32_pair(fps.get(), 1))
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
        }

        // SAFETY: `sink_writer` and `out_video` are the two live interface
        // pointers created above, and `AddStream` only reads the media type it is
        // handed. Called before `BeginWriting`, which is the order the sink writer
        // requires - adding a stream afterwards is rejected with an error, not by
        // corrupting anything.
        let video_idx =
            unsafe { sink_writer.AddStream(&out_video) }.map_err(|e| format!("AddStream: {e}"))?;

        // Input type: RGB32 (BGRA bottom-up)
        // SAFETY: as for `out_video` - no arguments, and MF is started.
        let in_video: IMFMediaType =
            unsafe { MFCreateMediaType() }.map_err(|e| format!("MFCreateMediaType in: {e}"))?;
        // SAFETY: as for the `out_video` block above: methods on the live
        // `in_video` pointer, taking `'static` GUID references and integers only.
        unsafe {
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
                .SetUINT64(&MF_MT_FRAME_RATE, pack_u32_pair(fps.get(), 1))
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
        }

        // SAFETY: `sink_writer` and `in_video` are both live, and `video_idx` is
        // the index `AddStream` returned for this same writer, so it names a stream
        // that exists rather than an unchecked number. The trailing `None` passes
        // no encoder-attribute pointer.
        unsafe { sink_writer.SetInputMediaType(video_idx, &in_video, None) }
            .map_err(|e| format!("SetInputMediaType: {e}"))?;

        // SAFETY: `sink_writer` is live and now in the state `BeginWriting`
        // expects - one stream added, with its input type set.
        unsafe { sink_writer.BeginWriting() }.map_err(|e| format!("BeginWriting: {e}"))?;

        let frame_interval = Duration::from_secs(1) / fps.get();

        let mut frame_index = FrameIndex::ZERO;
        let mut has_frame = false;
        let mut timeline_start = None;
        let mut terminal_error = None;
        // Preallocated once and reused for every Media Foundation sample. The
        // per-sample copy out of it is deliberate rather than a missed
        // optimisation: this buffer has to survive an iteration that captures
        // nothing, so a sample can never borrow it. See `write_video_sample`.
        let mut bgra_buffer = vec![0u8; dimensions.rgba_len()];

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

            let timing = frame_timing_hns(frame_index, fps);

            // SAFETY: `write_video_sample` wants a live sink writer from an active
            // MF session and a BGRA frame buffer. `sink_writer` is that writer,
            // `BeginWriting` has succeeded above and `Finalize` has not run yet, and
            // `bgra_buffer` holds `dimensions.rgba_len()` bytes - exactly the padded
            // frame `capture_into` just filled for these `width`/`height`.
            let write_result =
                unsafe { write_video_sample(&sink_writer, video_idx, &bgra_buffer, timing) };
            if let Err(e) = write_result {
                terminal_error = Some(format!("Failed to write frame {frame_index}: {e}"));
                break;
            }

            let timeline_start = *timeline_start.get_or_insert(t0);
            // Anchored to the wall clock, not to the number of frames written:
            // when a frame costs more than its interval the timeline skips ahead
            // instead of stretching real time across too few timestamps.
            frame_index = next_frame_index(frame_index, timeline_start.elapsed(), fps);

            let next_deadline = timeline_start
                + frame_interval
                    .checked_mul(frame_index.deadline_multiplier())
                    .unwrap_or(Duration::MAX);
            if let Some(remaining) = next_deadline.checked_duration_since(std::time::Instant::now())
            {
                thread::sleep(remaining);
            }
        }

        // SAFETY: `sink_writer` is still live here - it is a local of this closure,
        // and every path out of the loop above only `break`s. `Finalize` is called
        // exactly once, after `BeginWriting`, which is what it requires; the closure
        // returns immediately afterwards, so no further sample can be written to a
        // finalized writer.
        let finalize_result =
            unsafe { sink_writer.Finalize() }.map_err(|e| format!("Finalize: {e}"));
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
/// A fresh `IMFMediaBuffer` plus `IMFSample` per frame, and a full copy of
/// `data` into that buffer, sit right next to a `bgra_buffer` the caller
/// preallocates exactly once - 8 MB of copying per frame at 1080p, 250 MB/s at
/// 30 fps. Both allocations and the copy are deliberate; reusing either object
/// across frames would be a correctness bug, not an optimisation:
///
/// - The moment the sink writer lets go of a sample is not observable from here.
///   `MFCreateSinkWriterFromURL` is called with no attribute store, so there is
///   no `MF_SINK_WRITER_ASYNC_CALLBACK`, and `WriteSample` returning only tells
///   us the sample was accepted downstream - not that the encoder has released
///   it. Media Foundation ships `MFCreateTrackedSample` and
///   `IMFTrackedSample::SetAllocator` precisely because a caller cannot
///   otherwise learn when its buffer is free again, which is the strongest
///   evidence available from outside that the release point is not ours to
///   assume. Refilling a submitted buffer early would rewrite pixels the
///   encoder may still be reading, and that damage would surface as corrupted
///   frames inside a finished MP4 - never as an error anyone can act on.
/// - The caller's staging buffer has to outlive the sample. When
///   `RecordingFrameSource::capture_into` reports `Ok(false)` the compositor
///   produced no new frame and leaves the buffer untouched on purpose, so the
///   loop re-encodes the previous contents and the timeline keeps its constant
///   rate. Capturing straight into this per-frame buffer instead would leave
///   nothing to re-encode: every duplicate frame would carry a staging buffer
///   capture never filled, i.e. a black flash.
///
/// Removing the copy therefore means adding a tracked-sample pool that observes
/// the real release point - a far larger change than one warm-buffer copy, and
/// one that should not be attempted without a measurement showing this copy
/// actually costs frames.
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
    timing: FrameTiming,
) -> Result<(), String> {
    use windows::Win32::Media::MediaFoundation::*;

    // SAFETY: the argument is a length, not a pointer, and MF is started for this
    // thread by the caller's `MfGuard`. The cast matters rather than being cosmetic:
    // a truncated length would under-allocate the buffer that `copy_nonoverlapping`
    // below fills with `data.len()` bytes. It cannot truncate here, because
    // `wmf_record` only reaches this call once `capture_into` has succeeded, and that
    // requires the recorded region to fit inside a real monitor's frame - so
    // `data.len()` is at most one monitor's worth of BGRA, orders of magnitude below
    // `u32::MAX`.
    let buffer = unsafe { MFCreateMemoryBuffer(data.len() as u32) }
        .map_err(|e| format!("MFCreateMemoryBuffer: {e}"))?;

    let mut ptr: *mut u8 = std::ptr::null_mut();
    // SAFETY: `buffer` is the live buffer just created, and the only pointer passed
    // is the `&mut` to the local `ptr` it fills in; both optional out-parameters are
    // `None`. Nothing else locks this buffer, so the lock cannot be nested.
    unsafe { buffer.Lock(&mut ptr, None, None) }.map_err(|e| format!("Lock: {e}"))?;
    // SAFETY: the `?` above means this is reached only when `Lock` succeeded, and a
    // successful `IMFMediaBuffer::Lock` is contracted to leave `ptr` pointing at the
    // start of the buffer's own storage - writable up to its maximum length, which is
    // the `data.len()` requested from `MFCreateMemoryBuffer` and exactly the number of
    // bytes copied. The source is a `&[u8]` of that same length. The two regions
    // cannot overlap: this is a fresh MF allocation, `data` is the caller's staging
    // Vec. Nothing has unlocked the buffer in between, so the pointer is still live.
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len()) };
    // SAFETY: `buffer` is live and currently locked by the `Lock` above, which is
    // the one state `Unlock` may be called in. `ptr` is not used again afterwards.
    unsafe { buffer.Unlock() }.map_err(|e| format!("Unlock: {e}"))?;
    // SAFETY: `buffer` is live, and `data.len()` is the same value its maximum
    // length was created from, so the current length cannot exceed capacity.
    unsafe { buffer.SetCurrentLength(data.len() as u32) }
        .map_err(|e| format!("SetCurrentLength: {e}"))?;

    // SAFETY: no arguments; MF is started for this thread by the caller's `MfGuard`.
    let sample: IMFSample =
        unsafe { MFCreateSample() }.map_err(|e| format!("MFCreateSample: {e}"))?;
    // SAFETY: `sample` and `buffer` are both live interface pointers created above.
    // `AddBuffer` takes its own reference on the buffer, so the sample keeps it
    // alive past the end of this function - which is what lets the local `buffer`
    // binding drop here while the encoder may still be reading the pixels.
    unsafe { sample.AddBuffer(&buffer) }.map_err(|e| format!("AddBuffer: {e}"))?;
    // SAFETY: `sample` is live and both arguments are plain HNS integers from the
    // validated `FrameTiming`.
    unsafe { sample.SetSampleTime(timing.timestamp().get()) }
        .map_err(|e| format!("SetSampleTime: {e}"))?;
    // SAFETY: as above - live `sample`, integer argument.
    unsafe { sample.SetSampleDuration(timing.duration().get()) }
        .map_err(|e| format!("SetSampleDuration: {e}"))?;
    // SAFETY: the only caller, `wmf_record`, calls this from inside its capture loop
    // - after `BeginWriting` has succeeded and before `Finalize` - so `sink_writer`
    // is a live writer accepting samples, and `stream_index` is the index its own
    // `AddStream` returned. `sample` is the fully populated sample above.
    unsafe { sink_writer.WriteSample(stream_index, &sample) }
        .map_err(|e| format!("WriteSample: {e}"))?;

    Ok(())
}
