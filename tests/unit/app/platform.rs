//! Windows autorun registry helpers and `sync_autorun` policy tests.

use super::*;

#[cfg(windows)]
use crate::test_support::TestDir;
#[cfg(windows)]
use std::cell::RefCell;

#[test]
fn windows_manifest_declares_per_monitor_v2_dpi_awareness() {
    let manifest = include_str!("../../../rustshot.exe.manifest");
    let compact: String = manifest.split_whitespace().collect();

    assert!(compact.contains(">PerMonitorV2</dpiAwareness>"));
    assert!(compact.contains(">true/pm</dpiAware>"));
}

#[test]
fn installer_uses_the_same_fixed_per_user_directory_as_self_install() {
    let installer = include_str!("../../../installer.iss");

    for directive in [
        r"DefaultDirName={localappdata}\Programs\RustShot",
        "DisableDirPage=yes",
        "UsePreviousAppDir=no",
    ] {
        assert!(
            installer.lines().any(|line| line.trim() == directive),
            "installer is missing {directive}"
        );
    }
}

// --- autorun helper tests ---

#[cfg(windows)]
#[test]
fn quoted_autorun_command_wraps_paths_in_quotes() {
    let command = quoted_autorun_command(Path::new(r"C:\Program Files\RustShot\rustshot.exe"));
    assert_eq!(command, r#""C:\Program Files\RustShot\rustshot.exe""#);
}

#[cfg(windows)]
#[test]
fn autorun_registry_location_and_value_are_stable() {
    assert_eq!(
        AUTORUN_SUBKEY,
        r"Software\Microsoft\Windows\CurrentVersion\Run"
    );
    assert_eq!(AUTORUN_VALUE, "RustShot");
}

#[cfg(windows)]
#[test]
fn installed_exe_path_is_under_local_app_data_programs() {
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");

    assert_eq!(
        installed_exe_path(local_app_data),
        PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\RustShot.exe")
    );
}

#[cfg(windows)]
#[test]
fn enabled_non_installed_launch_installs_registers_and_relaunches_stable_copy() {
    let current_exe = Path::new(r"C:\Users\Ada\Downloads\RustShot.exe");
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");
    let stable_exe = PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\RustShot.exe");
    let events = RefCell::new(Vec::new());

    let action = sync_autorun_at_with_handlers(
        true,
        ExecutableInstaller::new(
            current_exe,
            local_app_data,
            |target: &Path| -> io::Result<bool> {
                assert_eq!(target, stable_exe);
                Ok(false)
            },
            |dir: &Path| -> io::Result<()> {
                assert_eq!(dir, stable_exe.parent().unwrap());
                events.borrow_mut().push("create directory");
                Ok(())
            },
            |source: &Path, target: &Path| -> io::Result<u64> {
                assert_eq!(source, current_exe);
                assert_eq!(target, stable_exe);
                events.borrow_mut().push("copy executable");
                Ok(42)
            },
        ),
        AutorunRegistration::new(
            |registered_exe: &Path| -> Result<(), String> {
                assert_eq!(registered_exe, stable_exe);
                events.borrow_mut().push("register autorun");
                Ok(())
            },
            || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
        ),
        |launched_exe: &Path| -> io::Result<()> {
            assert_eq!(launched_exe, stable_exe);
            events.borrow_mut().push("launch installed executable");
            Ok(())
        },
    )
    .expect("first-run installation should succeed");

    assert_eq!(action, StartupAction::Exit);
    assert_eq!(
        events.into_inner(),
        [
            "create directory",
            "copy executable",
            "register autorun",
            "launch installed executable",
        ]
    );
}

#[cfg(windows)]
#[test]
fn external_launch_hands_off_without_replacing_an_existing_install() {
    let current_exe = Path::new(r"C:\Users\Ada\Downloads\RustShot.exe");
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");
    let stable_exe = PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\RustShot.exe");
    let events = RefCell::new(Vec::new());

    let action = sync_autorun_at_with_handlers(
        true,
        ExecutableInstaller::new(
            current_exe,
            local_app_data,
            |target: &Path| -> io::Result<bool> {
                assert_eq!(target, stable_exe);
                Ok(true)
            },
            |_: &Path| -> io::Result<()> {
                panic!("an existing install needs no directory creation")
            },
            |_: &Path, _: &Path| -> io::Result<u64> {
                panic!("an existing installed executable must not be overwritten")
            },
        ),
        AutorunRegistration::new(
            |registered_exe: &Path| -> Result<(), String> {
                assert_eq!(registered_exe, stable_exe);
                events.borrow_mut().push("register autorun");
                Ok(())
            },
            || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
        ),
        |launched_exe: &Path| -> io::Result<()> {
            assert_eq!(launched_exe, stable_exe);
            events.borrow_mut().push("launch installed executable");
            Ok(())
        },
    )
    .expect("an existing installation should be reused");

    assert_eq!(action, StartupAction::Exit);
    assert_eq!(
        events.into_inner(),
        ["register autorun", "launch installed executable"]
    );
}

#[cfg(windows)]
#[test]
fn settings_reenable_from_portable_copy_registers_stable_path_without_relaunch() {
    let current_exe = Path::new(r"C:\Users\Ada\Downloads\RustShot.exe");
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");
    let stable_exe = PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\RustShot.exe");
    let events = RefCell::new(Vec::new());

    sync_current_exe_autorun_at_with_handlers(
        true,
        ExecutableInstaller::new(
            current_exe,
            local_app_data,
            |target: &Path| -> io::Result<bool> {
                assert_eq!(target, stable_exe);
                Ok(false)
            },
            |directory: &Path| -> io::Result<()> {
                assert_eq!(directory, stable_exe.parent().unwrap());
                events.borrow_mut().push("create directory");
                Ok(())
            },
            |source: &Path, target: &Path| -> io::Result<u64> {
                assert_eq!(source, current_exe);
                assert_eq!(target, stable_exe);
                events.borrow_mut().push("copy executable");
                Ok(42)
            },
        ),
        AutorunRegistration::new(
            |registered_exe: &Path| -> Result<(), String> {
                assert_eq!(registered_exe, stable_exe);
                events.borrow_mut().push("register autorun");
                Ok(())
            },
            || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
        ),
    )
    .expect("re-enabling startup should install the stable copy");

    assert_eq!(
        events.into_inner(),
        ["create directory", "copy executable", "register autorun"]
    );
}

#[cfg(windows)]
#[test]
fn enabled_already_installed_launch_only_registers_and_continues() {
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");
    let stable_exe = PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\RustShot.exe");
    // Windows paths are case-insensitive, and the existing installer writes the
    // executable name with this lower-case spelling.
    let installed_launch =
        PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\rustshot.exe");
    let register_count = Cell::new(0u32);

    let action = sync_autorun_at_with_handlers(
        true,
        ExecutableInstaller::new(
            &installed_launch,
            local_app_data,
            |_: &Path| -> io::Result<bool> {
                panic!("an installed launch must not probe itself as an external target")
            },
            |_: &Path| -> io::Result<()> { panic!("installed launch must not create a directory") },
            |_: &Path, _: &Path| -> io::Result<u64> {
                panic!("installed launch must not copy the executable")
            },
        ),
        AutorunRegistration::new(
            |registered_exe: &Path| -> Result<(), String> {
                assert_eq!(registered_exe, stable_exe);
                register_count.set(register_count.get() + 1);
                Ok(())
            },
            || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
        ),
        |_: &Path| -> io::Result<()> { panic!("installed launch must not relaunch itself") },
    )
    .expect("installed startup should succeed");

    assert_eq!(action, StartupAction::Continue);
    assert_eq!(register_count.get(), 1);
}

#[cfg(windows)]
#[test]
fn disabled_startup_only_removes_autorun_and_continues() {
    let current_exe = Path::new(r"C:\Users\Ada\Downloads\RustShot.exe");
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");
    let remove_count = Cell::new(0u32);

    let action = sync_autorun_at_with_handlers(
        false,
        ExecutableInstaller::new(
            current_exe,
            local_app_data,
            |_: &Path| -> io::Result<bool> {
                panic!("disabled startup must not inspect an install")
            },
            |_: &Path| -> io::Result<()> { panic!("disabled startup must not create a directory") },
            |_: &Path, _: &Path| -> io::Result<u64> {
                panic!("disabled startup must not copy the executable")
            },
        ),
        AutorunRegistration::new(
            |_: &Path| -> Result<(), String> {
                panic!("disabled startup must not register autorun")
            },
            || -> Result<(), String> {
                remove_count.set(remove_count.get() + 1);
                Ok(())
            },
        ),
        |_: &Path| -> io::Result<()> { panic!("disabled startup must not launch an executable") },
    )
    .expect("disabling startup should succeed");

    assert_eq!(action, StartupAction::Continue);
    assert_eq!(remove_count.get(), 1);
}

#[cfg(windows)]
#[test]
fn copy_failure_does_not_register_or_launch_and_is_human_readable() {
    let current_exe = Path::new(r"C:\Users\Ada\Downloads\RustShot.exe");
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");
    let stable_exe = PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\RustShot.exe");
    let registered = Cell::new(false);
    let launched = Cell::new(false);

    let error = sync_autorun_at_with_handlers(
        true,
        ExecutableInstaller::new(
            current_exe,
            local_app_data,
            |_: &Path| -> io::Result<bool> { Ok(false) },
            |_: &Path| -> io::Result<()> { Ok(()) },
            |_: &Path, _: &Path| -> io::Result<u64> {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "access denied",
                ))
            },
        ),
        AutorunRegistration::new(
            |_: &Path| -> Result<(), String> {
                registered.set(true);
                Ok(())
            },
            || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
        ),
        |_: &Path| -> io::Result<()> {
            launched.set(true);
            Ok(())
        },
    )
    .expect_err("copy failure must be reported");

    assert!(!registered.get(), "a failed copy must not be registered");
    assert!(!launched.get(), "a failed copy must not be launched");
    assert!(error.to_ascii_lowercase().contains("copy"), "{error}");
    assert!(
        error.contains(current_exe.to_string_lossy().as_ref()),
        "{error}"
    );
    assert!(
        error.contains(stable_exe.to_string_lossy().as_ref()),
        "{error}"
    );
    assert!(error.contains("access denied"), "{error}");
}

#[cfg(windows)]
#[test]
fn launch_failure_removes_the_new_autorun_entry_before_reporting_the_error() {
    let current_exe = Path::new(r"C:\Users\Ada\Downloads\RustShot.exe");
    let local_app_data = Path::new(r"C:\Users\Ada\AppData\Local");
    let stable_exe = PathBuf::from(r"C:\Users\Ada\AppData\Local\Programs\RustShot\RustShot.exe");
    let events = RefCell::new(Vec::new());

    let error = sync_autorun_at_with_handlers(
        true,
        ExecutableInstaller::new(
            current_exe,
            local_app_data,
            |_: &Path| -> io::Result<bool> { Ok(false) },
            |_: &Path| -> io::Result<()> { Ok(()) },
            |_: &Path, _: &Path| -> io::Result<u64> { Ok(42) },
        ),
        AutorunRegistration::new(
            |registered_exe: &Path| -> Result<(), String> {
                assert_eq!(registered_exe, stable_exe);
                events.borrow_mut().push("register autorun");
                Ok(())
            },
            || -> Result<(), String> {
                events.borrow_mut().push("remove autorun");
                Ok(())
            },
        ),
        |launched_exe: &Path| -> io::Result<()> {
            assert_eq!(launched_exe, stable_exe);
            events.borrow_mut().push("launch installed executable");
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "launch blocked",
            ))
        },
    )
    .expect_err("launch failure must be reported instead of requesting exit");

    assert_eq!(
        events.into_inner(),
        [
            "register autorun",
            "launch installed executable",
            "remove autorun",
        ],
        "a failed handoff must not leave a stale autorun entry"
    );
    assert!(error.to_ascii_lowercase().contains("launch"), "{error}");
    assert!(
        error.contains(stable_exe.to_string_lossy().as_ref()),
        "{error}"
    );
    assert!(error.contains("launch blocked"), "{error}");
}

#[cfg(windows)]
#[test]
fn a_directory_at_the_stable_exe_path_is_rejected_before_register_or_launch() {
    let root = TestDir::new("rustshot_install_target_directory");
    let current_exe = root.file("downloaded.exe");
    let stable_exe = installed_exe_path(root.path());
    std::fs::create_dir_all(&stable_exe).unwrap();
    let registered = Cell::new(false);
    let launched = Cell::new(false);

    let error = sync_autorun_at_with_handlers(
        true,
        ExecutableInstaller::new(
            &current_exe,
            root.path(),
            inspect_installed_executable,
            |_: &Path| -> io::Result<()> {
                panic!("an invalid occupied target must not create another directory")
            },
            |_: &Path, _: &Path| -> io::Result<u64> {
                panic!("an invalid occupied target must not be overwritten")
            },
        ),
        AutorunRegistration::new(
            |_: &Path| -> Result<(), String> {
                registered.set(true);
                Ok(())
            },
            || -> Result<(), String> { panic!("enabled startup must not remove autorun") },
        ),
        |_: &Path| -> io::Result<()> {
            launched.set(true);
            Ok(())
        },
    )
    .expect_err("a directory cannot be reused as the installed executable");

    assert!(!registered.get(), "a directory must not be registered");
    assert!(!launched.get(), "a directory must not be launched");
    assert!(
        error.contains(stable_exe.to_string_lossy().as_ref()),
        "{error}"
    );
    assert!(
        error.to_ascii_lowercase().contains("file"),
        "the error must explain that the occupied target is not a file: {error}"
    );
}

#[cfg(windows)]
#[test]
fn atomic_first_install_leaves_only_the_complete_executable() {
    let root = TestDir::new("rustshot_atomic_install");
    let source = root.file("downloaded.exe");
    let target = root.file("RustShot.exe");
    std::fs::write(&source, b"complete executable").unwrap();

    let copied = copy_executable_atomically(&source, &target).unwrap();

    assert_eq!(copied, b"complete executable".len() as u64);
    assert_eq!(std::fs::read(&target).unwrap(), b"complete executable");
    let names: Vec<_> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 2, "a staging executable was left behind");
}

#[cfg(windows)]
#[test]
fn atomic_copy_refuses_to_overwrite_an_existing_executable_and_cleans_staging() {
    let root = TestDir::new("rustshot_atomic_existing_install");
    let source = root.file("downloaded.exe");
    let target = root.file("RustShot.exe");
    let staging_name = format!("RustShot.exe.{}.installing", std::process::id());
    let staging = root.file(&staging_name);
    std::fs::write(&source, b"replacement executable").unwrap();
    std::fs::write(&target, b"existing sentinel").unwrap();

    copy_executable_atomically(&source, &target)
        .expect_err("atomic installation must not replace an existing executable");

    assert_eq!(
        std::fs::read(&target).unwrap(),
        b"existing sentinel",
        "the existing installation was overwritten"
    );
    assert!(
        !staging.exists(),
        "the failed copy left a staging executable"
    );
}

#[cfg(windows)]
#[test]
fn sync_autorun_with_handlers_enable_and_disable_are_mutually_exclusive() {
    let enable_count = Cell::new(0u32);
    let disable_count = Cell::new(0u32);

    sync_autorun_with_handlers(
        true,
        || enable_count.set(enable_count.get() + 1),
        || disable_count.set(disable_count.get() + 1),
    );
    assert_eq!(enable_count.get(), 1);
    assert_eq!(disable_count.get(), 0);

    sync_autorun_with_handlers(
        false,
        || enable_count.set(enable_count.get() + 1),
        || disable_count.set(disable_count.get() + 1),
    );
    assert_eq!(enable_count.get(), 1);
    assert_eq!(disable_count.get(), 1);
}
