#![expect(
    clippy::needless_pass_by_value,
    reason = "owned capture descriptors cross worker boundaries"
)]
//! Process-level runtime plumbing: logging setup, child-process error
//! reporting, and the selection/recording child-process modes.

use arboard::Clipboard;
use log::{error, info};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::application::capture::{
    execute_capture_with, CaptureFile, CaptureOutcome, CaptureRequest, CaptureSource,
};
use crate::{capture, cli, record, selection, settings};

/// Stop-flag file path for the recording child process with the given pid.
///
/// This single function IS the stop-flag IPC contract between the tray parent
/// and the `--select-record` child: the parent
/// (`RustShotApp::handle_recording_toggle`) creates the file from the child's
/// pid to request a stop, and the child ([`run_select_record`]) polls the same
/// path derived from its own pid. Both sides must call this helper — if the
/// paths ever drifted apart, stopping a recording would silently hang and the
/// child would record forever.
pub(crate) fn stop_flag_path(pid: u32) -> PathBuf {
    std::env::temp_dir().join(format!("rustshot_stop_{pid}.flag"))
}

/// Run the recording sub-process: show selection overlay, then record until stop-flag file appears.
pub(crate) fn run_select_record() {
    use crate::record::RecordingSession;
    let settings = settings::load_settings();
    if let Err(e) = record::validate_recording_start(settings.recording_fps.get()) {
        report_child_process_error(&format!("Failed to start recording: {}", e));
        return;
    }

    use crate::selection::select_region_for_recording;

    let selection_timer =
        crate::timing::ActionTimer::new("recording.region_selection_includes_user_wait");
    let (screen, region) = match select_region_for_recording() {
        Ok(r) => {
            selection_timer.finish("selected");
            r
        }
        Err(e) if e == "Selection cancelled" => {
            selection_timer.finish("cancelled");
            return;
        }
        Err(e) => {
            selection_timer.finish("error");
            report_child_process_error(&format!("Selection error: {}", e));
            return;
        }
    };

    let screen_x = screen.display_info.x;
    let screen_y = screen.display_info.y;

    info!(
        "Starting recording at ({}, {}), region: {:?}",
        screen_x, screen_y, region
    );
    let session = match crate::timing::time_result("recording.worker_launch", || {
        RecordingSession::start_in_dir(
            screen_x,
            screen_y,
            region,
            settings.recording_fps,
            settings.recordings_dir,
        )
    }) {
        Ok(s) => s,
        Err(e) => {
            report_child_process_error(&format!("Failed to start recording: {}", e));
            return;
        }
    };

    // Poll for stop-flag file.
    // Child side of the stop-flag IPC contract: derive the path from our OWN
    // pid via the shared `stop_flag_path` helper. The parent builds the same
    // path from the child's pid when it spawns us (see
    // `handle_recording_toggle`).
    let stop_file = stop_flag_path(std::process::id());

    loop {
        if stop_file.exists() {
            let _ = std::fs::remove_file(&stop_file);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    match crate::timing::time_result("recording.finalize_and_save", || session.stop()) {
        Ok(path) => info!("Recording saved to: {}", path.display()),
        Err(e) => report_child_process_error(&format!("Recording error: {}", e)),
    }
}

/// Run the selection overlay sub-process and copy the cropped image to clipboard.
pub(crate) fn run_selection() {
    let timing = crate::timing::ActionTimer::new("selection.includes_user_wait");
    match run_selection_with_clipboard(capture_requested_source, copy_image_to_clipboard) {
        Ok(()) => timing.finish("clipboard_written"),
        Err(error) => {
            timing.finish(if error == "Selection cancelled" {
                "cancelled"
            } else {
                "error"
            });
            if let Some(message) = selection_stderr_message(&error) {
                report_child_process_error(&message);
            }
        }
    }
}

/// Resolve a request without sampling the foreground again in the worker.
pub(crate) fn capture_requested_source(source: CaptureSource) -> Result<image::RgbaImage, String> {
    match source {
        CaptureSource::Selection => selection::select_region(),
        CaptureSource::Window(snapshot) => {
            capture_window_target_with(snapshot, capture::capture_window_target)
        }
    }
}

pub(crate) fn capture_window_target_with<Capture>(
    target: Result<capture::WindowTarget, String>,
    capture_target: Capture,
) -> Result<image::RgbaImage, String>
where
    Capture: FnOnce(capture::WindowTarget) -> Result<image::RgbaImage, String>,
{
    target.and_then(capture_target)
}

pub(crate) fn save_capture_file(
    image: &image::RgbaImage,
    file: CaptureFile,
) -> Result<PathBuf, String> {
    capture::save_active_window_image_in_dir(image, None, file.format, &file.directory)
}

pub(crate) fn log_capture_outcome(outcome: CaptureOutcome) {
    match outcome {
        CaptureOutcome::Clipboard => info!("Screenshot copied to clipboard!"),
        CaptureOutcome::ClipboardAndFile(path) => info!(
            "Window screenshot copied to clipboard and saved to: {}",
            path.display()
        ),
    }
}

/// Copy one RGBA image to the system clipboard without consuming it.
///
/// Active-window capture also saves the same pixels to disk, so borrowing the
/// image here keeps both destinations on one capture rather than racing two
/// independent foreground-window lookups.
pub(crate) fn copy_image_to_clipboard(image: &image::RgbaImage) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to access clipboard: {e}"))?;
    let image_data = arboard::ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: std::borrow::Cow::Borrowed(image.as_raw()),
    };

    clipboard
        .set_image(image_data)
        .map_err(|e| format!("Failed to copy to clipboard: {e}"))
}

/// Selection + clipboard logic with injected functions (for testing).
pub(crate) fn run_selection_with_clipboard<FSelect, FClipboard>(
    select_region_fn: FSelect,
    copy_to_clipboard_fn: FClipboard,
) -> Result<(), String>
where
    FSelect: FnOnce(CaptureSource) -> Result<image::RgbaImage, String>,
    FClipboard: FnOnce(&image::RgbaImage) -> Result<(), String>,
{
    let outcome = execute_capture_with(
        CaptureRequest::selection(),
        select_region_fn,
        |image| crate::timing::time_result("selection.clipboard", || copy_to_clipboard_fn(image)),
        save_capture_file,
    )?;
    crate::timing::mark(crate::timing::CaptureMilestone::ClipboardWritten);
    log_capture_outcome(outcome);
    Ok(())
}

/// Convert a selection error into an optional stderr message (suppresses "Selection cancelled").
pub(crate) fn selection_stderr_message(error: &str) -> Option<String> {
    cli::selection_stderr_message(error)
}

pub(crate) fn write_error_line<W: Write>(mut writer: W, message: &str) -> io::Result<()> {
    cli::write_error_line(&mut writer, message)
}

fn report_child_process_error(message: &str) {
    error!("{message}");
    let _ = write_error_line(io::stderr(), message);
}

/// Initialise file-based logging to `%APPDATA%/RustShot/rustshot.log`.
/// Silently falls back to no logging on any failure.
pub(crate) fn init_logging() {
    use simplelog::{Config, LevelFilter, WriteLogger};
    let path = settings::log_file_path();
    if let Ok(file) = open_log_file(&path) {
        let _ = WriteLogger::init(LevelFilter::Debug, Config::default(), file);
    }
}

pub(crate) fn open_log_file(path: &Path) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    OpenOptions::new().create(true).append(true).open(path)
}
