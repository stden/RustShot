//! Capture output delivery policy, independent of desktop and UI adapters.
//!
//! A capture produces one image for both destinations. Clipboard failure must
//! not suppress file saving, and partial success must name the completed output
//! so callers can report exactly what the user received.

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
    let image = capture().map_err(|error| format!("Failed to capture active window: {error}"))?;
    let clipboard_result = copy_to_clipboard(&image);
    let save_result = save(&image);

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
