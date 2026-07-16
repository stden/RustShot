//! Persistent Windows monitor capture for the recording loop.

mod gdi;
mod wgc;

use self::gdi::GdiRegionCapture;
use self::wgc::{GraphicsCaptureMonitor, SendableHmonitor};
use super::Region;
use std::collections::HashMap;
use std::sync::{mpsc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const MAX_WGC_RESTARTS: u8 = 3;
const SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(3);
const SCREENSHOT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MonitorCaptureKey {
    raw_monitor: SendableHmonitor,
}

struct MonitorCaptureRequest {
    key: MonitorCaptureKey,
    deadline: Instant,
    response: mpsc::SyncSender<Result<image::RgbaImage, String>>,
}

static MONITOR_CAPTURE_WORKER: OnceLock<Result<mpsc::SyncSender<MonitorCaptureRequest>, String>> =
    OnceLock::new();

pub(super) struct RecordingFrameSource {
    backend: RecordingFrameBackend,
    raw_monitor: SendableHmonitor,
    region: Region,
    full_monitor: bool,
    wgc_restarts: u8,
}

enum RecordingFrameBackend {
    GraphicsCapture(GraphicsCaptureMonitor),
    Gdi(GdiRegionCapture),
}

impl RecordingFrameSource {
    pub(super) fn new(screen_x: i32, screen_y: i32, region: Region) -> Result<Self, String> {
        let raw_monitor = monitor_handle_at_origin(screen_x, screen_y)?;
        Self::new_for_monitor(raw_monitor, region, false)
    }

    fn new_full(raw_monitor: SendableHmonitor) -> Result<Self, String> {
        Self::new_for_monitor(
            raw_monitor,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 1,
                physical_height: 1,
            },
            true,
        )
    }

    fn new_for_monitor(
        raw_monitor: SendableHmonitor,
        fallback_region: Region,
        full_monitor: bool,
    ) -> Result<Self, String> {
        let (backend, region) = Self::create_backend(raw_monitor, fallback_region, full_monitor)?;
        Ok(Self {
            backend,
            raw_monitor,
            region,
            full_monitor,
            wgc_restarts: 0,
        })
    }

    fn create_backend(
        raw_monitor: SendableHmonitor,
        fallback_region: Region,
        full_monitor: bool,
    ) -> Result<(RecordingFrameBackend, Region), String> {
        let wgc_capture = if full_monitor {
            GraphicsCaptureMonitor::new_full(raw_monitor)
        } else {
            GraphicsCaptureMonitor::new(raw_monitor, fallback_region)
        };
        match wgc_capture {
            Ok(capture) => {
                let region = capture.region();
                Ok((RecordingFrameBackend::GraphicsCapture(capture), region))
            }
            Err(wgc_error) => {
                let reason = wgc_error.into_reason();
                log::warn!(
                    "Windows Graphics Capture initialization failed ({reason}); using GDI fallback"
                );
                let gdi_capture = if full_monitor {
                    GdiRegionCapture::new_full(raw_monitor.raw())
                } else {
                    GdiRegionCapture::new(raw_monitor.raw(), fallback_region)
                };
                gdi_capture
                    .map(|capture| {
                        let region = capture.region();
                        (RecordingFrameBackend::Gdi(capture), region)
                    })
                    .map_err(|gdi_error| {
                        format!(
                            "Windows Graphics Capture failed ({reason}); GDI fallback also failed: {gdi_error}"
                        )
                    })
            }
        }
    }

    /// Update the output buffer with a new frame. False means no compositor
    /// frame was available, so the encoder may reuse its prior frame.
    pub(super) fn capture_into(
        &mut self,
        output_width: u32,
        output_height: u32,
        output: &mut [u8],
        wait: Duration,
    ) -> Result<bool, String> {
        let result = match &mut self.backend {
            RecordingFrameBackend::GraphicsCapture(capture) => {
                capture.capture_into(output_width, output_height, output, wait)
            }
            RecordingFrameBackend::Gdi(capture) => {
                return capture
                    .capture_into(output_width, output_height, output)
                    .map(|()| true);
            }
        };

        match result {
            Ok(updated) => {
                if updated {
                    self.wgc_restarts = 0;
                }
                Ok(updated)
            }
            Err(CaptureRuntimeError::Fatal(error)) => Err(error),
            Err(CaptureRuntimeError::Recoverable(error)) => self.recover_wgc(error),
        }
    }

    fn recover_wgc(&mut self, reason: String) -> Result<bool, String> {
        self.wgc_restarts = self.wgc_restarts.saturating_add(1);
        log::warn!(
            "Windows Graphics Capture needs recovery (attempt {}/{}): {reason}",
            self.wgc_restarts,
            MAX_WGC_RESTARTS
        );

        if self.wgc_restarts < MAX_WGC_RESTARTS {
            let restarted = if self.full_monitor {
                GraphicsCaptureMonitor::new_full(self.raw_monitor)
            } else {
                GraphicsCaptureMonitor::new(self.raw_monitor, self.region)
            };
            match restarted {
                Ok(capture) => {
                    self.region = capture.region();
                    self.backend = RecordingFrameBackend::GraphicsCapture(capture);
                    return Ok(false);
                }
                Err(error) => {
                    log::warn!(
                        "Failed to restart Windows Graphics Capture: {}",
                        error.into_reason()
                    );
                }
            }
        }

        let fallback = if self.full_monitor {
            GdiRegionCapture::new_full(self.raw_monitor.raw())
        } else {
            GdiRegionCapture::new(self.raw_monitor.raw(), self.region)
        };
        match fallback {
            Ok(capture) => {
                log::warn!("Continuing recording with the persistent GDI fallback");
                self.region = capture.region();
                self.backend = RecordingFrameBackend::Gdi(capture);
                Ok(false)
            }
            Err(gdi_error) => Err(format!(
                "Windows Graphics Capture could not recover ({reason}); GDI fallback failed: {gdi_error}"
            )),
        }
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.region.physical_width, self.region.physical_height)
    }
}

/// Capture one complete monitor through the same DPI/HDR-aware backend used
/// for recording.
pub(crate) fn capture_monitor_image(raw_monitor: isize) -> Result<image::RgbaImage, String> {
    let worker = monitor_capture_worker()?;
    let deadline = Instant::now() + SCREENSHOT_RESPONSE_TIMEOUT;
    let (response, result) = mpsc::sync_channel(1);
    let mut request = MonitorCaptureRequest {
        key: MonitorCaptureKey {
            raw_monitor: SendableHmonitor::from_raw(raw_monitor),
        },
        deadline,
        response,
    };
    loop {
        match worker.try_send(request) {
            Ok(()) => break,
            Err(mpsc::TrySendError::Full(returned)) => {
                request = returned;
                if Instant::now() >= deadline {
                    return Err("Timed out waiting to queue the monitor capture".to_string());
                }
                thread::sleep(Duration::from_millis(1));
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return Err("Monitor capture worker stopped unexpectedly".to_string())
            }
        }
    }
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| "Timed out waiting for the monitor capture worker".to_string())?;
    result
        .recv_timeout(remaining)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => {
                "Timed out waiting for the monitor capture worker".to_string()
            }
            mpsc::RecvTimeoutError::Disconnected => {
                "Monitor capture worker stopped before returning an image".to_string()
            }
        })?
}

fn monitor_capture_worker() -> Result<&'static mpsc::SyncSender<MonitorCaptureRequest>, String> {
    match MONITOR_CAPTURE_WORKER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel(4);
        thread::Builder::new()
            .name("rustshot-monitor-capture".to_string())
            .spawn(move || run_monitor_capture_worker(receiver))
            .map_err(|error| format!("Failed to start monitor capture worker: {error}"))?;
        Ok(sender)
    }) {
        Ok(worker) => Ok(worker),
        Err(error) => Err(error.clone()),
    }
}

fn run_monitor_capture_worker(receiver: mpsc::Receiver<MonitorCaptureRequest>) {
    let mut sources = HashMap::<MonitorCaptureKey, RecordingFrameSource>::new();
    while let Ok(request) = receiver.recv() {
        let result = if Instant::now() >= request.deadline {
            Err("Monitor capture request expired in the worker queue".to_string())
        } else {
            capture_monitor_image_on_worker(&mut sources, request.key, request.deadline)
        };
        let _ = request.response.send(result);
    }
}

fn capture_monitor_image_on_worker(
    sources: &mut HashMap<MonitorCaptureKey, RecordingFrameSource>,
    key: MonitorCaptureKey,
    request_deadline: Instant,
) -> Result<image::RgbaImage, String> {
    prune_disconnected_monitor_sources(sources);
    if let std::collections::hash_map::Entry::Vacant(entry) = sources.entry(key) {
        entry.insert(RecordingFrameSource::new_full(key.raw_monitor)?);
    }

    let (mut width, mut height) = sources
        .get(&key)
        .expect("monitor capture source must exist")
        .dimensions();
    let mut bgra_bottom_up = allocate_monitor_buffer(width, height)?;
    let deadline = (Instant::now() + SCREENSHOT_TIMEOUT).min(request_deadline);

    let capture_result = loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| "Timed out waiting for a monitor screenshot frame".to_string());
        let remaining = match remaining {
            Ok(remaining) => remaining,
            Err(error) => break Err(error),
        };
        let source = sources
            .get_mut(&key)
            .expect("monitor capture source must exist");
        match source.capture_into(
            width,
            height,
            &mut bgra_bottom_up,
            remaining.min(Duration::from_millis(100)),
        ) {
            Ok(true) => break Ok(()),
            Ok(false) => {
                let new_dimensions = source.dimensions();
                if new_dimensions != (width, height) {
                    (width, height) = new_dimensions;
                    bgra_bottom_up = allocate_monitor_buffer(width, height)?;
                }
            }
            Err(error) => break Err(error),
        }
    };
    if let Err(error) = capture_result {
        sources.remove(&key);
        return Err(error);
    }

    let mut rgba_top_down = vec![0; bgra_bottom_up.len()];
    super::rgba_to_bgra_bottom_up_into(&bgra_bottom_up, &mut rgba_top_down, width, height)?;
    image::RgbaImage::from_raw(width, height, rgba_top_down)
        .ok_or_else(|| "Failed to construct the monitor screenshot image".to_string())
}

fn allocate_monitor_buffer(width: u32, height: u32) -> Result<Vec<u8>, String> {
    let output_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Monitor screenshot is too large".to_string())?;
    Ok(vec![0; output_len])
}

fn monitor_handle_at_origin(screen_x: i32, screen_y: i32) -> Result<SendableHmonitor, String> {
    screenshots::Screen::all()
        .map_err(|error| format!("Failed to get screens: {error}"))?
        .into_iter()
        .find(|screen| screen.display_info.x == screen_x && screen.display_info.y == screen_y)
        .map(|screen| SendableHmonitor::from_raw(screen.display_info.raw_handle.0))
        .ok_or_else(|| format!("Screen at ({screen_x},{screen_y}) not found"))
}

fn prune_disconnected_monitor_sources(
    sources: &mut HashMap<MonitorCaptureKey, RecordingFrameSource>,
) {
    let Ok(screens) = screenshots::Screen::all() else {
        return;
    };
    let active: Vec<_> = screens
        .into_iter()
        .map(|screen| SendableHmonitor::from_raw(screen.display_info.raw_handle.0))
        .collect();
    sources.retain(|key, _| active.contains(&key.raw_monitor));
}

pub(super) enum GraphicsCaptureInitError {
    Unsupported(String),
    Failed(String),
}

impl GraphicsCaptureInitError {
    fn into_reason(self) -> String {
        match self {
            Self::Unsupported(reason) | Self::Failed(reason) => reason,
        }
    }
}

pub(super) enum CaptureRuntimeError {
    Recoverable(String),
    Fatal(String),
}
