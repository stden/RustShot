//! Cursor position adapter shared by screenshot capture and the overlay.

use crate::domain::LogicalPoint;

/// Get the current cursor position in virtual screen coordinates.
///
/// Returns a typed point in logical (virtual desktop) pixels.
#[cfg(windows)]
pub fn get_cursor_position() -> Result<LogicalPoint, String> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    // SAFETY: the only pointer `GetCursorPos` receives is the `&mut` to this
    // stack `POINT`, which is fully initialised (`Default`) and outlives the
    // synchronous call. It writes two `i32`s and nothing else, so the widest
    // failure mode left is an error code, which is mapped below rather than
    // leaving `point` read as if it had been filled in.
    unsafe { GetCursorPos(&mut point) }.map_err(|e| format!("GetCursorPos failed: {e}"))?;
    Ok(LogicalPoint::new(point.x, point.y))
}

/// Get the current cursor position in virtual screen coordinates (Linux/other).
#[cfg(not(windows))]
pub fn get_cursor_position() -> Result<LogicalPoint, String> {
    Err("Cursor position detection is not supported on this platform".to_string())
}
