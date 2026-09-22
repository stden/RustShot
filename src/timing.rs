#![expect(
    clippy::needless_pass_by_value,
    reason = "timing events are small owned values"
)]
//! Request and stage durations for user actions and the hotkey-to-clipboard path.
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

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> u64 {
    NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

/// One action's timing state, movable into a worker without sharing mutable
/// state between threads. Shared borrows let successive injected handlers mark
/// the same request without requiring separate timers for each closure.
pub(crate) struct ActionTimer {
    action: &'static str,
    process_id: u32,
    request_id: u64,
    origin: Instant,
    previous: Cell<Instant>,
    finished: bool,
}

enum ActionEvent {
    Started,
    Stage(&'static str),
    Finished(&'static str),
    Abandoned,
}

impl std::fmt::Display for ActionEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Started => formatter.write_str("stage=started"),
            Self::Stage(stage) => write!(formatter, "stage={stage}"),
            Self::Finished(outcome) => write!(formatter, "stage=finished outcome={outcome}"),
            Self::Abandoned => formatter.write_str("stage=abandoned outcome=abandoned"),
        }
    }
}

impl ActionTimer {
    pub(crate) fn new(action: &'static str) -> Self {
        let origin = Instant::now();
        let timer = Self {
            action,
            process_id: std::process::id(),
            request_id: next_request_id(),
            origin,
            previous: Cell::new(origin),
            finished: false,
        };
        let record = timer.record_at(ActionEvent::Started, origin);
        log::info!("{record}");
        timer
    }

    /// Record both the time since the previous stage and the whole action.
    pub(crate) fn mark(&self, stage: &'static str) {
        let record = self.record_at(ActionEvent::Stage(stage), Instant::now());
        log::info!("{record}");
    }

    /// Record an explicit outcome exactly once. Consuming the timer prevents
    /// later stages from being attributed to an already completed request.
    pub(crate) fn finish(mut self, outcome: &'static str) {
        self.finished = true;
        let record = self.record_at(ActionEvent::Finished(outcome), Instant::now());
        log::info!("{record}");
    }

    fn record_at(&self, event: ActionEvent, now: Instant) -> String {
        let previous = self.previous.replace(now);
        let step = now.saturating_duration_since(previous);
        let total = now.saturating_duration_since(self.origin);
        format!(
            "[action] pid={} request={} action={} {event} step_ms={:.3} total_ms={:.3}",
            self.process_id,
            self.request_id,
            self.action,
            step.as_secs_f64() * 1000.0,
            total.as_secs_f64() * 1000.0,
        )
    }

    fn abandonment_record_at(&self, now: Instant) -> Option<String> {
        (!self.finished).then(|| self.record_at(ActionEvent::Abandoned, now))
    }
}

impl Drop for ActionTimer {
    fn drop(&mut self) {
        // A failed thread spawn drops its captured timer without running the
        // worker, as do early returns and unwinding. None implies success.
        if let Some(record) = self.abandonment_record_at(Instant::now()) {
            log::info!("{record}");
        }
    }
}

/// Time a fallible operation without inspecting or changing its payloads.
pub(crate) fn time_result<T, E>(
    action: &'static str,
    operation: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let timer = ActionTimer::new(action);
    let result = operation();
    timer.finish(if result.is_ok() { "ok" } else { "error" });
    result
}

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
static CAPTURE_REQUEST_ID: OnceLock<u64> = OnceLock::new();
static CAPTURE_PREVIOUS: Mutex<Option<Instant>> = Mutex::new(None);

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
        let now = Instant::now();
        let previous = {
            let mut previous = CAPTURE_PREVIOUS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            previous.replace(now).unwrap_or(*origin)
        };
        log::info!(
            "{}",
            format_capture_record(
                milestone,
                std::process::id(),
                *CAPTURE_REQUEST_ID.get_or_init(next_request_id),
                now.saturating_duration_since(previous),
                now.saturating_duration_since(*origin),
            )
        );
    }
}

fn format_capture_record(
    milestone: CaptureMilestone,
    process_id: u32,
    request_id: u64,
    step: Duration,
    total: Duration,
) -> String {
    // Preserve the milestone label for existing log filters, while making the
    // user's drag time distinguishable from screenshot-processing latency.
    let stage = match milestone {
        CaptureMilestone::RegionSelected => "user_selection_wait",
        _ => milestone.label(),
    };
    format!(
        "{} pid={process_id} request={request_id} stage={stage} step_ms={:.3} total_ms={:.3}",
        format_milestone(milestone, total),
        step.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0,
    )
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
