//! Windows auto-start integration: HKCU Run-key registration and the two-way
//! sync policy applied at tray startup.

#[cfg(windows)]
use crate::settings;

/// Windows registry subkey for auto-start entries.
#[cfg(windows)]
pub(crate) const AUTORUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Registry value name used by RustShot.
#[cfg(windows)]
pub(crate) const AUTORUN_VALUE: &str = "RustShot";

/// Wrap an exe path in quotes for the Windows Run registry value.
#[cfg(windows)]
pub(crate) fn quoted_autorun_command(path: &std::path::Path) -> String {
    format!("\"{}\"", path.to_string_lossy())
}

/// Register RustShot in Windows startup (HKCU\...\Run) using the native Registry API.
/// Uses RegSetKeyValueW which creates the key and sets the value in one call.
#[cfg(windows)]
fn setup_autorun() {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::*;

    let exe_path = match std::env::current_exe() {
        Ok(path) => quoted_autorun_command(&path),
        Err(_) => return,
    };

    let subkey = settings::wide(AUTORUN_SUBKEY);
    let value_name = settings::wide(AUTORUN_VALUE);
    let exe_wide = settings::wide(&exe_path);

    // SAFETY: RegSetKeyValueW is a stable Win32 API. The `subkey`, `value_name`,
    // and `exe_wide` buffers are null-terminated UTF-16 vectors that remain valid
    // for the duration of the call. `byte_len` is computed from the Vec length.
    unsafe {
        let byte_len = (exe_wide.len() * 2) as u32;
        let _ = RegSetKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            REG_SZ.0,
            Some(exe_wide.as_ptr() as *const core::ffi::c_void),
            byte_len,
        );
    }
}

#[cfg(windows)]
fn remove_autorun() {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegDeleteKeyValueW, HKEY_CURRENT_USER};

    let subkey = settings::wide(AUTORUN_SUBKEY);
    let value_name = settings::wide(AUTORUN_VALUE);

    // SAFETY: RegDeleteKeyValueW is a stable Win32 API. The `subkey` and
    // `value_name` buffers are null-terminated UTF-16 vectors that outlive the call.
    unsafe {
        let _ = RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
        );
    }
}

#[cfg(any(test, windows))]
pub(crate) fn sync_autorun_with_handlers<Enable, Disable>(
    enabled: bool,
    enable_autorun: Enable,
    disable_autorun: Disable,
) where
    Enable: FnOnce(),
    Disable: FnOnce(),
{
    if enabled {
        enable_autorun();
    } else {
        disable_autorun();
    }
}

/// Synchronise the Windows auto-start registry entry with the current setting.
///
/// When `enabled` is `true`, registers the current exe path under
/// `HKCU\...\Run`; when `false`, removes it.
///
/// This is deliberately a **two-way** sync. The original implementation only
/// ever registered, so once auto-start was on the user could never turn it off.
///
/// Two invariants worth keeping:
/// - It writes to `HKCU` only, never `HKLM`. That is what keeps RustShot free of
///   any elevation/admin requirement.
/// - It is applied only in Tray mode, so the `--select` / `--select-record`
///   child processes never touch the registry.
#[cfg(windows)]
pub(crate) fn sync_autorun(enabled: bool) {
    sync_autorun_with_handlers(enabled, setup_autorun, remove_autorun);
}
