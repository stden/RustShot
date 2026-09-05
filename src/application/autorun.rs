//! Startup installation and auto-start policy, shared by tray startup and settings.
//!
//! Handler-based entry points make copy/register/launch ordering independent of
//! the caller's event loop. Native adapters supply the filesystem and registry
//! operations; this module never imports presentation or composition modules.

#[cfg(windows)]
pub(crate) use crate::platform::autorun::{
    copy_executable_atomically, inspect_installed_executable,
};
#[cfg(all(test, windows))]
pub(crate) use crate::platform::autorun::{quoted_autorun_command, AUTORUN_SUBKEY, AUTORUN_VALUE};
#[cfg(windows)]
use crate::platform::autorun::{remove_autorun, setup_autorun};
use std::path::{Path, PathBuf};

/// What the original process should do after startup installation is prepared.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StartupAction {
    /// This process is already the installed copy and should start the tray.
    Continue,
    /// The installed copy was launched, so this source process should exit.
    Exit,
}

/// Return the stable, per-user executable path used by the Windows installer.
pub(crate) fn installed_exe_path(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("Programs")
        .join("RustShot")
        .join("RustShot.exe")
}

/// Whether two Windows executable paths identify the same spelling apart from
/// ASCII case, which Windows ignores for the ordinary paths used here.
fn same_windows_path(left: &Path, right: &Path) -> bool {
    left.as_os_str().eq_ignore_ascii_case(right.as_os_str())
}

fn install_executable_at_with_handlers<TargetExists, CreateDir, CopyExe>(
    current_exe: &Path,
    local_app_data: &Path,
    target_exists: TargetExists,
    create_dir: CreateDir,
    copy_exe: CopyExe,
) -> Result<(PathBuf, bool), String>
where
    TargetExists: FnOnce(&Path) -> std::io::Result<bool>,
    CreateDir: FnOnce(&Path) -> std::io::Result<()>,
    CopyExe: FnOnce(&Path, &Path) -> std::io::Result<u64>,
{
    let installed_exe = installed_exe_path(local_app_data);
    if same_windows_path(current_exe, &installed_exe) {
        return Ok((installed_exe, false));
    }

    let already_installed = target_exists(&installed_exe).map_err(|error| {
        format!(
            "Failed to inspect RustShot install path {}: {error}",
            installed_exe.display()
        )
    })?;
    if already_installed {
        return Ok((installed_exe, true));
    }

    let install_dir = installed_exe.parent().ok_or_else(|| {
        format!(
            "Installed executable has no parent: {}",
            installed_exe.display()
        )
    })?;
    create_dir(install_dir).map_err(|error| {
        format!(
            "Failed to create RustShot install directory {}: {error}",
            install_dir.display()
        )
    })?;
    copy_exe(current_exe, &installed_exe).map_err(|error| {
        format!(
            "Failed to copy RustShot from {} to {}: {error}",
            current_exe.display(),
            installed_exe.display()
        )
    })?;

    Ok((installed_exe, true))
}

/// Apply startup policy at explicit paths through injected side-effect handlers.
///
/// Keeping the ordering here testable prevents a failed first-run copy from
/// becoming the executable Windows tries to launch at the next sign-in.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_autorun_at_with_handlers<
    TargetExists,
    CreateDir,
    CopyExe,
    Register,
    Remove,
    Launch,
>(
    enabled: bool,
    current_exe: &Path,
    local_app_data: &Path,
    target_exists: TargetExists,
    create_dir: CreateDir,
    copy_exe: CopyExe,
    register_autorun: Register,
    remove_autorun: Remove,
    launch_exe: Launch,
) -> Result<StartupAction, String>
where
    TargetExists: FnOnce(&Path) -> std::io::Result<bool>,
    CreateDir: FnOnce(&Path) -> std::io::Result<()>,
    CopyExe: FnOnce(&Path, &Path) -> std::io::Result<u64>,
    Register: FnOnce(&Path) -> Result<(), String>,
    Remove: FnOnce() -> Result<(), String>,
    Launch: FnOnce(&Path) -> std::io::Result<()>,
{
    if !enabled {
        remove_autorun()?;
        return Ok(StartupAction::Continue);
    }

    let (installed_exe, hand_off) = install_executable_at_with_handlers(
        current_exe,
        local_app_data,
        target_exists,
        create_dir,
        copy_exe,
    )?;
    register_autorun(&installed_exe)?;
    if !hand_off {
        return Ok(StartupAction::Continue);
    }
    if let Err(error) = launch_exe(&installed_exe) {
        let launch_error = format!(
            "Failed to launch installed RustShot at {}: {error}",
            installed_exe.display()
        );
        return match remove_autorun() {
            Ok(()) => Err(launch_error),
            Err(remove_error) => Err(format!(
                "{launch_error}; additionally failed to remove its Windows startup entry: {remove_error}"
            )),
        };
    }

    Ok(StartupAction::Exit)
}

/// Exercise Windows startup-install orchestration without touching the host.
///
/// This is public only so the executable Gherkin specification can inject all
/// filesystem, registry, and process effects. `true` means the source process
/// should exit after successfully launching the installed copy.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn benchmark_sync_autorun_at_with_handlers<
    TargetExists,
    CreateDir,
    CopyExe,
    Register,
    Remove,
    Launch,
>(
    enabled: bool,
    current_exe: &Path,
    local_app_data: &Path,
    target_exists: TargetExists,
    create_dir: CreateDir,
    copy_exe: CopyExe,
    register_autorun: Register,
    remove_autorun: Remove,
    launch_exe: Launch,
) -> Result<bool, String>
where
    TargetExists: FnOnce(&Path) -> std::io::Result<bool>,
    CreateDir: FnOnce(&Path) -> std::io::Result<()>,
    CopyExe: FnOnce(&Path, &Path) -> std::io::Result<u64>,
    Register: FnOnce(&Path) -> Result<(), String>,
    Remove: FnOnce() -> Result<(), String>,
    Launch: FnOnce(&Path) -> std::io::Result<()>,
{
    sync_autorun_at_with_handlers(
        enabled,
        current_exe,
        local_app_data,
        target_exists,
        create_dir,
        copy_exe,
        register_autorun,
        remove_autorun,
        launch_exe,
    )
    .map(|action| action == StartupAction::Exit)
}

#[cfg(test)]
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn sync_current_exe_autorun_at_with_handlers<
    TargetExists,
    CreateDir,
    CopyExe,
    Register,
    Remove,
>(
    enabled: bool,
    current_exe: &Path,
    local_app_data: &Path,
    target_exists: TargetExists,
    create_dir: CreateDir,
    copy_exe: CopyExe,
    register_autorun: Register,
    remove_autorun: Remove,
) -> Result<(), String>
where
    TargetExists: FnOnce(&Path) -> std::io::Result<bool>,
    CreateDir: FnOnce(&Path) -> std::io::Result<()>,
    CopyExe: FnOnce(&Path, &Path) -> std::io::Result<u64>,
    Register: FnOnce(&Path) -> Result<(), String>,
    Remove: FnOnce() -> Result<(), String>,
{
    if !enabled {
        return remove_autorun();
    }

    let (installed_exe, _hand_off) = install_executable_at_with_handlers(
        current_exe,
        local_app_data,
        target_exists,
        create_dir,
        copy_exe,
    )?;
    register_autorun(&installed_exe)
}

/// Exercise Settings auto-start synchronization without touching the host.
///
/// This is public only for the executable Gherkin specification. Unlike tray
/// startup synchronization, it never relaunches or asks the current process to
/// exit.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn benchmark_sync_current_exe_autorun_at_with_handlers<
    TargetExists,
    CreateDir,
    CopyExe,
    Register,
    Remove,
>(
    enabled: bool,
    current_exe: &Path,
    local_app_data: &Path,
    target_exists: TargetExists,
    create_dir: CreateDir,
    copy_exe: CopyExe,
    register_autorun: Register,
    remove_autorun: Remove,
) -> Result<(), String>
where
    TargetExists: FnOnce(&Path) -> std::io::Result<bool>,
    CreateDir: FnOnce(&Path) -> std::io::Result<()>,
    CopyExe: FnOnce(&Path, &Path) -> std::io::Result<u64>,
    Register: FnOnce(&Path) -> Result<(), String>,
    Remove: FnOnce() -> Result<(), String>,
{
    sync_current_exe_autorun_at_with_handlers(
        enabled,
        current_exe,
        local_app_data,
        target_exists,
        create_dir,
        copy_exe,
        register_autorun,
        remove_autorun,
    )
}

/// Install RustShot per-user and synchronise Windows auto-start with the setting.
///
/// An enabled launch from outside the stable install directory uses the existing
/// installed copy, or atomically creates it when absent. It registers and launches
/// that copy without arguments, then tells the caller to exit before creating a
/// tray or claiming global hotkeys. An enabled installed launch only refreshes its
/// Run entry. A disabled launch removes it.
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
pub(crate) fn sync_autorun(enabled: bool) -> Result<StartupAction, String> {
    if !enabled {
        return remove_autorun().map(|()| StartupAction::Continue);
    }

    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Failed to locate the running RustShot executable: {error}"))?;
    let local_app_data = dirs::data_local_dir()
        .ok_or_else(|| "Failed to locate the current user's local application data".to_string())?;

    sync_autorun_at_with_handlers(
        true,
        &current_exe,
        &local_app_data,
        inspect_installed_executable,
        |path| std::fs::create_dir_all(path),
        copy_executable_atomically,
        setup_autorun,
        remove_autorun,
        |path| std::process::Command::new(path).spawn().map(|_child| ()),
    )
}

/// Apply a settings-dialog change to the executable that owns the current tray.
///
/// Enabling from a portable session still copies to the stable location, but
/// does not relaunch while the dialog and tray event loops are active. Windows
/// will use the installed copy at the next sign-in.
#[cfg(windows)]
pub(crate) fn sync_current_exe_autorun(enabled: bool) -> Result<(), String> {
    if !enabled {
        return remove_autorun();
    }

    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Failed to locate the running RustShot executable: {error}"))?;
    let local_app_data = dirs::data_local_dir()
        .ok_or_else(|| "Failed to locate the current user's local application data".to_string())?;
    sync_current_exe_autorun_at_with_handlers(
        true,
        &current_exe,
        &local_app_data,
        inspect_installed_executable,
        |path| std::fs::create_dir_all(path),
        copy_executable_atomically,
        setup_autorun,
        remove_autorun,
    )
}
