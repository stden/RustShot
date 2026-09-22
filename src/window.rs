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
/// The tray passes an explicit directory via [`capture_window_in_dir`]; this
/// default-directory convenience remains available to library consumers.
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

    /// The callback appends to this Vec, reached through `lparam`.
    ///
    /// # Safety
    ///
    /// `lparam` must carry a pointer to a live, exclusively-borrowed
    /// `Vec<WindowInfo>`; it is dereferenced as `&mut` for the duration of the
    /// call. Only the `EnumWindows` call below installs this callback, and it
    /// passes exactly that.
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: per the contract above, `lparam` holds the pointer to the
        // caller's `Vec<WindowInfo>`. `EnumWindows` is synchronous, so that Vec
        // is still borrowed and alive, and no other reference to it exists while
        // the enumeration runs on this thread.
        let windows = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };

        // SAFETY: `hwnd` is supplied by `EnumWindows` itself, so it is a window
        // handle rather than an arbitrary value. The query takes the handle by
        // value and touches no memory of ours; a window destroyed mid-enumeration
        // makes it report "not visible" rather than misbehave.
        let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
        // SAFETY: same handle, same reasoning - by-value query, and a window that
        // has gone away yields length 0, which the branch below already handles.
        let len = unsafe { GetWindowTextLengthW(hwnd) };
        let title = if len > 0 {
            // +1 for the terminating NUL that GetWindowTextW writes.
            let mut buf = vec![0u16; (len + 1) as usize];
            // SAFETY: windows-rs derives both `lpString` and `nMaxCount` from
            // this one slice, so the count it passes cannot exceed the `len + 1`
            // UTF-16 units just allocated - the NUL included. `copied` is the
            // count excluding that NUL, hence never past the end.
            let copied = unsafe { GetWindowTextW(hwnd, &mut buf) };
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
    // SAFETY: `enum_proc` has the `WNDENUMPROC` signature, and its documented
    // requirement is met here: the `lparam` we pass is a pointer to `result`,
    // which is borrowed mutably for the whole expression and lives until the end
    // of this function. `EnumWindows` is synchronous and calls the callback only
    // on this thread before returning, so the pointer cannot outlive the borrow.
    unsafe {
        let _ = EnumWindows(
            Some(enum_proc),
            LPARAM(std::ptr::from_mut(&mut result) as isize),
        );
    }
    result
}

#[cfg(windows)]
fn capture_window_image_platform(id: u64) -> Result<image::RgbaImage, String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;

    let hwnd = HWND(id as usize as *mut core::ffi::c_void);
    // SAFETY: this is the one call that may receive a bogus handle - `id` comes
    // from the CLI or from a stale window list - and `IsWindow` is defined for
    // exactly that: it takes the handle by value, dereferences nothing of ours,
    // and reports whether the value still names a live window.
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
#[path = "../tests/unit/window/mod.rs"]
mod tests;
