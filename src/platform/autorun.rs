//! Windows registry and executable-installation adapters.
//!
//! This leaf owns native effects, without deciding whether startup should enable,
//! disable, or launch `RustShot`. Application workflows determine that ordering.

use crate::platform::wide;
use std::path::Path;

/// Windows registry subkey for auto-start entries.
#[cfg(windows)]
pub(crate) const AUTORUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Registry value name used by `RustShot`.
#[cfg(windows)]
pub(crate) const AUTORUN_VALUE: &str = "RustShot";

/// Report whether an installed executable is present without accepting another
/// filesystem object (for example, a directory) as something Windows can run.
#[cfg(windows)]
pub(crate) fn inspect_installed_executable(path: &Path) -> std::io::Result<bool> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "the RustShot install target exists but is not a regular file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Wrap an exe path in quotes for the Windows Run registry value.
#[cfg(windows)]
pub(crate) fn quoted_autorun_command(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy())
}

/// Register a `RustShot` executable in Windows startup using the native Registry API.
#[cfg(windows)]
pub(crate) fn setup_autorun(exe_path: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::*;

    let exe_path = quoted_autorun_command(exe_path);

    let subkey = wide(AUTORUN_SUBKEY);
    let value_name = wide(AUTORUN_VALUE);
    let exe_wide = wide(&exe_path);

    // `wide` appends the NUL, so `byte_len` counts the terminator the
    // registry needs to store with a REG_SZ value.
    let byte_len = (exe_wide.len() * 2) as u32;
    // SAFETY: all three pointers come from `wide`, so each names a
    // NUL-terminated UTF-16 buffer, and all three `Vec`s are live locals that
    // outlive this synchronous call. The value data is described by `byte_len`,
    // computed from that same `exe_wide` length, so the registry reads exactly
    // the bytes the Vec owns. `HKEY_CURRENT_USER` is a predefined key that needs
    // no open/close, which is what keeps this call free of a handle to leak.
    let status = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            REG_SZ.0,
            Some(exe_wide.as_ptr() as *const core::ffi::c_void),
            byte_len,
        )
    };
    status
        .ok()
        .map_err(|error| format!("Failed to register RustShot for Windows startup: {error}"))
}

#[cfg(windows)]
pub(crate) fn remove_autorun() -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND};
    use windows::Win32::System::Registry::{RegDeleteKeyValueW, HKEY_CURRENT_USER};

    let subkey = wide(AUTORUN_SUBKEY);
    let value_name = wide(AUTORUN_VALUE);

    // SAFETY: both pointers come from `wide`, so each names a
    // NUL-terminated UTF-16 buffer, and both `Vec`s are live locals that outlive
    // this synchronous call. Nothing is written back through them, and a missing
    // key or value is reported in the return code rather than being undefined -
    // which is why the discarded result is not a swallowed soundness problem.
    let status = unsafe {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
        )
    };
    if status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND {
        return Ok(());
    }
    status
        .ok()
        .map_err(|error| format!("Failed to remove RustShot from Windows startup: {error}"))
}

/// Stage a first-run executable beside its destination, then rename it into place.
///
/// The startup path is not registered until this succeeds, so a failed copy can
/// leave at most an unreferenced staging file, never a partial startup binary.
#[cfg(windows)]
pub(crate) fn copy_executable_atomically(source: &Path, target: &Path) -> std::io::Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVE_FILE_FLAGS};

    let file_name = target.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "RustShot install path has no file name",
        )
    })?;
    let mut staging_name = file_name.to_os_string();
    staging_name.push(format!(".{}.installing", std::process::id()));
    let staging_path = target.with_file_name(staging_name);

    let copied = match std::fs::copy(source, &staging_path) {
        Ok(copied) => copied,
        Err(error) => {
            let _ = std::fs::remove_file(&staging_path);
            return Err(error);
        }
    };
    let staging_wide: Vec<u16> = staging_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let target_wide: Vec<u16> = target
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both vectors are live, NUL-terminated UTF-16 path buffers. Zero
    // flags deliberately omit MOVEFILE_REPLACE_EXISTING, so a target created
    // after our metadata check wins the race instead of being overwritten.
    let moved = unsafe {
        MoveFileExW(
            PCWSTR(staging_wide.as_ptr()),
            PCWSTR(target_wide.as_ptr()),
            MOVE_FILE_FLAGS(0),
        )
    };
    if let Err(error) = moved {
        let _ = std::fs::remove_file(&staging_path);
        return Err(std::io::Error::other(error));
    }

    Ok(copied)
}
