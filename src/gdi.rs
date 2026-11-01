//! Shared Win32 monitor capture setup and allocation failure cleanup.

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateDCW, DeleteDC, GetMonitorInfoW, HBITMAP, HDC, HMONITOR,
    MONITORINFOEXW,
};

/// Read the monitor descriptor used for both screenshot and recording capture.
///
/// # Safety
/// The caller must provide a valid monitor handle.
pub(crate) unsafe fn monitor_info(raw_hmonitor: isize) -> Result<MONITORINFOEXW, String> {
    let mut info: MONITORINFOEXW = std::mem::zeroed();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    GetMonitorInfoW(
        HMONITOR(raw_hmonitor),
        (&mut info as *mut MONITORINFOEXW).cast(),
    )
    .ok()
    .map_err(|e| format!("GetMonitorInfoW failed: {e}"))?;
    Ok(info)
}

/// Create a monitor DC. On success the caller owns it and must call DeleteDC.
///
/// # Safety
/// The descriptor must come from GetMonitorInfoW and contain a terminated name.
pub(crate) unsafe fn monitor_dc(info: &MONITORINFOEXW) -> Result<HDC, String> {
    let name = PCWSTR(info.szDevice.as_ptr());
    let dc = CreateDCW(name, name, PCWSTR(std::ptr::null()), None);
    if dc.is_invalid() {
        return Err("CreateDCW for monitor failed".to_string());
    }
    Ok(dc)
}

/// Allocate a capture bitmap, releasing both caller-owned DCs on failure.
///
/// # Safety
/// Both DCs must be valid, distinct, owned by the caller and not previously
/// released. On success ownership stays with the caller, including the bitmap;
/// on error the caller must not use or delete either DC again.
pub(crate) unsafe fn capture_bitmap_or_cleanup(
    monitor: HDC,
    memory: HDC,
    width: i32,
    height: i32,
) -> Result<HBITMAP, String> {
    let bitmap = CreateCompatibleBitmap(monitor, width, height);
    if bitmap.is_invalid() {
        let _ = DeleteDC(memory);
        let _ = DeleteDC(monitor);
        return Err("CreateCompatibleBitmap failed".to_string());
    }
    Ok(bitmap)
}
