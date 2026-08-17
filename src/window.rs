//! Enumerating and capturing individual top-level windows.
//!
//! Two capabilities: [`list_windows`] returns the visible, titled top-level
//! windows, and [`capture_window_image`] / [`capture_window`] screenshot one of
//! them by id. Both are used by the CLI (`--list-windows`, `--window <id>`) and
//! by the MCP server.
//!
//! A window's `id` is its Win32 `HWND` widened to `u64`. That keeps the value
//! opaque and stable for the lifetime of the window, and lets it round-trip
//! through JSON (MCP) and the command line as a plain integer.
//!
//! Window enumeration is Windows-only. On other platforms [`list_windows`]
//! returns an empty list and the capture functions return an error, mirroring
//! how the rest of the crate treats Windows-only features.

use crate::capture::{self, ensure_parent_directory, resolve_output_path_in_dir, Format};
use std::path::{Path, PathBuf};

/// A top-level window that can be listed and captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// Opaque window id (the Win32 `HWND` as a `u64`).
    pub id: u64,
    /// The window's title-bar text.
    pub title: String,
}

/// Decide whether an enumerated window belongs in the user-facing list.
///
/// Kept as a pure function — separate from the Win32 enumeration — so the policy
/// is unit-testable without a live desktop. We only surface windows that are
/// both visible and titled: invisible windows and the many untitled helper/tool
/// windows are noise a user would never mean to screenshot.
///
/// # Examples
///
/// ```
/// use rustshot::window::is_listable_window;
///
/// assert!(is_listable_window(true, "Untitled - Notepad"));
/// assert!(!is_listable_window(false, "Background window")); // hidden
/// assert!(!is_listable_window(true, "")); // untitled
/// assert!(!is_listable_window(true, "   ")); // whitespace-only
/// ```
pub fn is_listable_window(is_visible: bool, title: &str) -> bool {
    is_visible && !title.trim().is_empty()
}

/// Render the window list as human-readable lines, one window per line.
///
/// Pure and platform-independent so the exact CLI/MCP text output is testable.
///
/// # Examples
///
/// ```
/// use rustshot::window::{format_window_list, WindowInfo};
///
/// let windows = vec![
///     WindowInfo { id: 66048, title: "Notepad".to_string() },
///     WindowInfo { id: 197660, title: "Terminal".to_string() },
/// ];
/// assert_eq!(
///     format_window_list(&windows),
///     "Open windows:\n  66048  Notepad\n  197660  Terminal",
/// );
/// assert_eq!(format_window_list(&[]), "No open windows found.");
/// ```
pub fn format_window_list(windows: &[WindowInfo]) -> String {
    if windows.is_empty() {
        return "No open windows found.".to_string();
    }
    let mut out = String::from("Open windows:");
    for w in windows {
        out.push_str(&format!("\n  {}  {}", w.id, w.title));
    }
    out
}

/// List the visible, titled top-level windows, most-recently-active first.
///
/// Returns an empty list on non-Windows platforms.
pub fn list_windows() -> Vec<WindowInfo> {
    list_windows_platform()
}

/// Capture window `id` into an RGBA image.
///
/// Errors if the id does not refer to a live window, or on non-Windows platforms.
pub fn capture_window_image(id: u64) -> Result<image::RgbaImage, String> {
    capture_window_image_platform(id)
}

/// Capture window `id` and save it, choosing the path the same way as a normal
/// screenshot when `output` is not given.
///
/// `#[allow(dead_code)]`: the binary always passes an explicit directory via
/// [`capture_window_in_dir`]; this default-directory convenience exists for
/// library consumers (the MCP server, tests).
#[allow(dead_code)]
pub fn capture_window(id: u64, output: Option<PathBuf>, format: Format) -> Result<PathBuf, String> {
    capture_window_in_dir(id, output, format, &capture::get_screenshots_dir())
}

/// Capture window `id`, saving under an explicit screenshots directory.
pub fn capture_window_in_dir(
    id: u64,
    output: Option<PathBuf>,
    format: Format,
    screenshots_dir: &Path,
) -> Result<PathBuf, String> {
    let image = capture_window_image(id)?;
    let output_path = resolve_output_path_in_dir(output, format, screenshots_dir);
    ensure_parent_directory(&output_path)?;
    capture::save_image_optimized(&image, &output_path, format)?;
    Ok(output_path)
}

// ---------------------------------------------------------------------------
// Platform implementations
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn list_windows_platform() -> Vec<WindowInfo> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, TRUE};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
    };

    // The callback appends to this Vec, reached through `lparam`.
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `&mut Vec<WindowInfo>` we passed to
        // `EnumWindows` below; the reference is valid for the whole enumeration,
        // which runs synchronously on this thread.
        let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

        let visible = IsWindowVisible(hwnd).as_bool();
        let len = GetWindowTextLengthW(hwnd);
        let title = if len > 0 {
            // +1 for the terminating NUL that GetWindowTextW writes.
            let mut buf = vec![0u16; (len + 1) as usize];
            let copied = GetWindowTextW(hwnd, &mut buf);
            String::from_utf16_lossy(&buf[..copied as usize])
        } else {
            String::new()
        };

        if is_listable_window(visible, &title) {
            windows.push(WindowInfo {
                id: hwnd.0 as u64,
                title,
            });
        }
        TRUE // keep enumerating
    }

    let mut result: Vec<WindowInfo> = Vec::new();
    // SAFETY: `enum_proc` matches the WNDENUMPROC signature and only touches the
    // Vec pointer we hand it. EnumWindows is synchronous, so `result` outlives
    // the call.
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut result as *mut _ as isize));
    }
    result
}

#[cfg(windows)]
fn capture_window_image_platform(id: u64) -> Result<image::RgbaImage, String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;

    let hwnd = HWND(id as usize as *mut core::ffi::c_void);
    // SAFETY: `IsWindow` accepts any handle value and simply reports validity.
    if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
        return Err(format!("No window with id {id}"));
    }
    capture::capture_hwnd(hwnd)
}

#[cfg(not(windows))]
fn list_windows_platform() -> Vec<WindowInfo> {
    Vec::new()
}

#[cfg(not(windows))]
fn capture_window_image_platform(_id: u64) -> Result<image::RgbaImage, String> {
    Err("Window capture is only supported on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listable_requires_visible_and_titled() {
        assert!(is_listable_window(true, "Editor"));
        assert!(!is_listable_window(false, "Editor"));
        assert!(!is_listable_window(true, ""));
        assert!(!is_listable_window(true, "\t \n"));
    }

    #[test]
    fn format_window_list_empty() {
        assert_eq!(format_window_list(&[]), "No open windows found.");
    }

    #[test]
    fn format_window_list_lines() {
        let windows = vec![
            WindowInfo {
                id: 10,
                title: "A".to_string(),
            },
            WindowInfo {
                id: 20,
                title: "B".to_string(),
            },
        ];
        assert_eq!(
            format_window_list(&windows),
            "Open windows:\n  10  A\n  20  B"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_returns_empty_and_errors() {
        assert!(list_windows().is_empty());
        assert!(capture_window_image(1).is_err());
    }

    /// A `--window-id` from a stale window list must be rejected by name before
    /// any capture backend is started, rather than surfacing as a WGC or GDI
    /// failure further down.
    #[cfg(windows)]
    #[test]
    fn unknown_window_id_is_rejected_before_capture() {
        assert_eq!(
            capture_window_image(u64::MAX),
            Err(format!("No window with id {}", u64::MAX))
        );
    }
}
