//! Millisecond timing for the hotkey-to-clipboard path.
//!
//! The `<100 ms` capture target in `IDEAS.md` was unfalsifiable: the log writes
//! whole seconds, and in a normal run the process start and the overlay
//! appearing land inside the same second, so the log could not say whether that
//! step took 5 ms or 900 ms. Every milestone here carries an explicit delta from
//! one origin instead, which is why the log's own timestamp precision does not
//! matter and no new dependency is needed to fix it.
//!
//! Timing is deliberately coarse-grained and log-based rather than a sampling
//! profiler: the costs being hunted are whole stages — enumerating monitors,
//! standing up a Graphics Capture session, letting the compositor settle the
//! overlay window — not individual functions.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// A point on the hotkey-to-clipboard path worth timing.
///
/// Ordered as the capture reaches them, so a log read top to bottom is the
/// timeline. Named rather than free-form strings so a run can be diffed against
/// an earlier one without the labels drifting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureMilestone {
    /// `Screen::all()` returned the monitor list.
    ScreensEnumerated,
    /// The monitor under the cursor has been captured into an image.
    MonitorCaptured,
    /// The overlay window and its drawing surface exist.
    OverlayWindowCreated,
    /// The first overlay frame has been presented — the moment the user can see
    /// and act on the frozen screenshot. This is the latency that matters.
    OverlayFirstFrame,
    /// The drag finished and resolved to a region.
    RegionSelected,
    /// The region has been cropped out of the captured frame.
    ImageCropped,
    /// The cropped image reached the clipboard.
    ClipboardWritten,
}

impl CaptureMilestone {
    /// Stable, greppable label.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::ScreensEnumerated => "screens_enumerated",
            Self::MonitorCaptured => "monitor_captured",
            Self::OverlayWindowCreated => "overlay_window_created",
            Self::OverlayFirstFrame => "overlay_first_frame",
            Self::RegionSelected => "region_selected",
            Self::ImageCropped => "image_cropped",
            Self::ClipboardWritten => "clipboard_written",
        }
    }
}

/// Origin of the current process's capture timeline.
///
/// One origin per process is exactly one capture: `--select` and
/// `--select-record` each run as their own short-lived child process.
static ORIGIN: OnceLock<Instant> = OnceLock::new();

/// Start the timeline. Ignored if one was already started.
pub(crate) fn start_capture_timeline() {
    let _ = ORIGIN.set(Instant::now());
}

/// Log `milestone` with its offset from the timeline origin.
///
/// Silent when no timeline was started, so library consumers and the tray
/// process pay nothing.
pub(crate) fn mark(milestone: CaptureMilestone) {
    if let Some(origin) = ORIGIN.get() {
        log::info!("{}", format_milestone(milestone, origin.elapsed()));
    }
}

/// Render one milestone line, e.g. `[timing] overlay_first_frame +18.204 ms`.
///
/// Microsecond resolution: the stages being measured range from tens of
/// microseconds to hundreds of milliseconds, and rounding to whole milliseconds
/// would erase the cheap ones entirely.
pub(crate) fn format_milestone(milestone: CaptureMilestone, elapsed: Duration) -> String {
    format!(
        "[timing] {} +{:.3} ms",
        milestone.label(),
        elapsed.as_secs_f64() * 1000.0
    )
}

#[cfg(test)]
#[path = "../tests/unit/timing/mod.rs"]
mod tests;
