//! Capture output delivery policy, independent of desktop and UI adapters.
//!
//! Requests describe the two supported interactive workflows: selection to the
//! clipboard, or a callback-snapshotted window to clipboard and file. Private
//! request fields prevent destinations from drifting away from their source.
//! Both outputs borrow one captured image. Clipboard failure must not suppress
//! file saving, and partial success names the output the user actually received.

use std::path::PathBuf;

use crate::domain::Format;
use crate::platform::window::WindowTarget;

#[cfg(test)]
#[path = "../../tests/unit/application/capture.rs"]
mod tests;

/// Native acquisition input, consumed once by the presentation adapter.
pub(crate) enum CaptureSource {
    Selection,
    /// Retain failed snapshots too: a worker must never replace one with a later
    /// foreground lookup. Successful targets retain their native generation lease.
    Window(Result<WindowTarget, String>),
}

/// File policy sampled with the request, rather than reloading settings later.
pub(crate) struct CaptureFile {
    pub(crate) format: Format,
    pub(crate) directory: PathBuf,
}

enum CaptureDelivery {
    Clipboard,
    ClipboardAndFile(CaptureFile),
}

/// An interactive capture with exactly the outputs supported by its source.
pub(crate) struct CaptureRequest {
    source: CaptureSource,
    delivery: CaptureDelivery,
}

impl CaptureRequest {
    pub(crate) fn selection() -> Self {
        Self {
            source: CaptureSource::Selection,
            delivery: CaptureDelivery::Clipboard,
        }
    }

    pub(crate) fn window(snapshot: Result<WindowTarget, String>, file: CaptureFile) -> Self {
        Self {
            source: CaptureSource::Window(snapshot),
            delivery: CaptureDelivery::ClipboardAndFile(file),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CaptureOutcome {
    Clipboard,
    ClipboardAndFile(PathBuf),
}

/// Acquire once, then deliver the same image according to the request's policy.
///
/// Capture handlers return raw errors; this workflow adds the source context.
/// Native acquisition, clipboard access, file encoding, and their timing remain
/// adapter responsibilities, so this workflow has no desktop or event-loop calls.
pub(crate) fn execute_capture_with<Capture, Clipboard, Save>(
    request: CaptureRequest,
    capture: Capture,
    copy_to_clipboard: Clipboard,
    save: Save,
) -> Result<CaptureOutcome, String>
where
    Capture: FnOnce(CaptureSource) -> Result<image::RgbaImage, String>,
    Clipboard: FnOnce(&image::RgbaImage) -> Result<(), String>,
    Save: FnOnce(&image::RgbaImage, CaptureFile) -> Result<PathBuf, String>,
{
    let image = match request.source {
        source @ CaptureSource::Selection => capture(source).map_err(selection_capture_error)?,
        source @ CaptureSource::Window(_) => {
            capture(source).map_err(|error| window_capture_error(&error))?
        }
    };
    match request.delivery {
        CaptureDelivery::Clipboard => copy_to_clipboard(&image).map(|()| CaptureOutcome::Clipboard),
        CaptureDelivery::ClipboardAndFile(file) => {
            deliver_window_image_with_handlers(&image, copy_to_clipboard, |image| save(image, file))
                .map(CaptureOutcome::ClipboardAndFile)
        }
    }
}

fn selection_capture_error(error: String) -> String {
    if error == "Selection cancelled" {
        error
    } else {
        format!("Selection error: {error}")
    }
}

fn window_capture_error(error: &str) -> String {
    format!("Failed to capture active window: {error}")
}

/// Capture one active-window image and deliver the same pixels to clipboard and disk.
pub(crate) fn capture_active_window_outputs_with_handlers<Capture, Clipboard, Save>(
    capture: Capture,
    copy_to_clipboard: Clipboard,
    save: Save,
) -> Result<std::path::PathBuf, String>
where
    Capture: FnOnce() -> Result<image::RgbaImage, String>,
    Clipboard: FnOnce(&image::RgbaImage) -> Result<(), String>,
    Save: FnOnce(&image::RgbaImage) -> Result<std::path::PathBuf, String>,
{
    let image = capture().map_err(|error| window_capture_error(&error))?;
    deliver_window_image_with_handlers(&image, copy_to_clipboard, save)
}

fn deliver_window_image_with_handlers<Clipboard, Save>(
    image: &image::RgbaImage,
    copy_to_clipboard: Clipboard,
    save: Save,
) -> Result<PathBuf, String>
where
    Clipboard: FnOnce(&image::RgbaImage) -> Result<(), String>,
    Save: FnOnce(&image::RgbaImage) -> Result<PathBuf, String>,
{
    let clipboard_result = copy_to_clipboard(image);
    let save_result = save(image);

    match (clipboard_result, save_result) {
        (Ok(()), Ok(path)) => Ok(path),
        (Err(clipboard_error), Ok(path)) => Err(format!(
            "Window screenshot saved to {}, but clipboard delivery failed: {clipboard_error}",
            path.display()
        )),
        (Ok(()), Err(save_error)) => Err(format!(
            "Window screenshot copied to clipboard, but file save failed: {save_error}"
        )),
        (Err(clipboard_error), Err(save_error)) => Err(format!(
            "Window screenshot clipboard delivery failed: {clipboard_error}; file save also failed: {save_error}"
        )),
    }
}

/// Exercise active-window output delivery without touching the real desktop.
#[doc(hidden)]
pub fn benchmark_capture_active_window_outputs_with_handlers<Capture, Clipboard, Save>(
    capture: Capture,
    copy_to_clipboard: Clipboard,
    save: Save,
) -> Result<std::path::PathBuf, String>
where
    Capture: FnOnce() -> Result<image::RgbaImage, String>,
    Clipboard: FnOnce(&image::RgbaImage) -> Result<(), String>,
    Save: FnOnce(&image::RgbaImage) -> Result<std::path::PathBuf, String>,
{
    capture_active_window_outputs_with_handlers(capture, copy_to_clipboard, save)
}
