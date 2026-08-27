//! Infrastructure helper tests: logging, error reporting, folder opening, hotkeys, child processes.

use super::*;
use std::io::Write;

#[test]
fn register_hotkey_with_returns_id_on_success() {
    use global_hotkey::hotkey::Code;

    let hotkey = HotKey::new(None, Code::PrintScreen);
    let expected_id = hotkey.id();
    let result = register_hotkey_with(|_| Ok::<(), io::Error>(()), hotkey, "PrintScreen");
    assert_eq!(result.unwrap(), expected_id);
}

#[test]
fn register_hotkey_with_wraps_registration_error() {
    use global_hotkey::hotkey::Code;

    let hotkey = HotKey::new(None, Code::PrintScreen);
    let err = register_hotkey_with(
        |_| Err(io::Error::new(io::ErrorKind::AddrInUse, "already in use")),
        hotkey,
        "PrintScreen",
    )
    .unwrap_err();

    assert!(err.contains("PrintScreen"));
    assert!(err.contains("already in use"));
}

#[test]
fn open_log_file_appends_without_truncating_existing_content() {
    use std::io::Read;

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("rustshot_log_test_{unique}"))
        .join("rustshot.log");

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"old\n").unwrap();

    {
        let mut file = open_log_file(&path).unwrap();
        file.write_all(b"new\n").unwrap();
    }

    let mut content = String::new();
    std::fs::File::open(&path)
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
    assert_eq!(content, "old\nnew\n");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(path.parent().unwrap());
}

#[test]
fn open_log_file_creates_missing_parent_directories() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("rustshot_log_create_dir_{unique}"));
    let path = base.join("nested").join("rustshot.log");

    {
        let mut file = open_log_file(&path).unwrap();
        file.write_all(b"created\n").unwrap();
    }

    assert!(path.exists(), "log file should be created");
    assert!(path.parent().unwrap().is_dir(), "parent dir should exist");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(path.parent().unwrap());
    let _ = std::fs::remove_dir(&base);
}

#[test]
fn open_log_file_returns_error_when_parent_path_is_blocked_by_file() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("rustshot_log_blocked_parent_{unique}"));
    std::fs::write(&base, b"not a directory").unwrap();

    let path = base.join("nested").join("rustshot.log");
    let err = open_log_file(&path).unwrap_err();
    assert!(
        matches!(
            err.kind(),
            io::ErrorKind::AlreadyExists
                | io::ErrorKind::NotADirectory
                | io::ErrorKind::PermissionDenied
                | io::ErrorKind::Other
        ),
        "unexpected error kind: {:?}",
        err.kind()
    );

    let _ = std::fs::remove_file(&base);
}

#[test]
fn open_log_file_reports_a_path_with_no_parent_instead_of_creating_one() {
    // An empty path has no parent component, so the directory step is skipped
    // entirely and the failure has to come from the open itself. Logging is
    // best-effort — `init_logging` swallows the error — so the contract here is
    // simply that it returns rather than panicking on the way.
    assert!(open_log_file(Path::new("")).is_err());
}

#[test]
fn open_log_file_returns_error_when_target_path_is_directory() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rustshot_log_dir_target_{unique}"));
    std::fs::create_dir_all(&path).unwrap();

    let err = open_log_file(&path).unwrap_err();
    assert!(
        matches!(
            err.kind(),
            io::ErrorKind::PermissionDenied
                | io::ErrorKind::AlreadyExists
                | io::ErrorKind::IsADirectory
                | io::ErrorKind::Other
        ),
        "unexpected error kind: {:?}",
        err.kind()
    );

    let _ = std::fs::remove_dir(&path);
}

#[test]
fn open_screenshots_folder_with_handlers_calls_open_on_create_success() {
    let created = Cell::new(false);
    let opened = Cell::new(false);

    open_screenshots_folder_with_handlers(
        |dir| {
            created.set(true);
            assert!(dir.ends_with("RustShot"));
            Ok(())
        },
        |_| {
            opened.set(true);
        },
    );

    assert!(created.get());
    assert!(opened.get());
}

#[test]
fn open_screenshots_folder_with_handlers_skips_open_on_create_failure() {
    let created = Cell::new(false);
    let opened = Cell::new(false);

    open_screenshots_folder_with_handlers(
        |_| {
            created.set(true);
            Err("cannot create".to_string())
        },
        |_| {
            opened.set(true);
        },
    );

    assert!(created.get());
    assert!(!opened.get());
}

#[test]
fn open_screenshots_folder_at_with_handlers_uses_provided_dir() {
    let dir = PathBuf::from("/tmp/custom-shots");
    let create_called = Cell::new(false);
    let open_called = Cell::new(false);
    open_screenshots_folder_at_with_handlers(
        &dir,
        |actual_dir| {
            create_called.set(true);
            assert_eq!(actual_dir, dir.as_path());
            Ok(())
        },
        |actual_dir| {
            open_called.set(true);
            assert_eq!(actual_dir, dir.as_path());
        },
    );
    assert!(create_called.get());
    assert!(open_called.get());
}

#[test]
fn open_screenshots_folder_at_with_handlers_skips_open_when_create_dir_fails() {
    let dir = PathBuf::from("/tmp/custom-shots");
    let open_called = Cell::new(false);

    open_screenshots_folder_at_with_handlers(
        &dir,
        |_| Err("mkdir failed".to_string()),
        |_| open_called.set(true),
    );

    assert!(!open_called.get());
}

#[test]
fn child_process_is_running_covers_every_try_wait_outcome() {
    assert!(child_process_is_running(Ok::<Option<()>, ()>(None)));
    assert!(!child_process_is_running(Ok::<Option<()>, ()>(Some(()))));
    assert!(!child_process_is_running(Err::<Option<()>, ()>(())));
}

#[test]
fn recording_stop_reports_signal_write_failure() {
    let path = std::path::Path::new("blocked.stop");
    let error = request_recording_stop_with(path, |_, _| {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ))
    })
    .unwrap_err();

    assert!(error.contains("blocked.stop"));
    assert!(error.contains("denied"));
}

// --- write_error_line edge cases ---

#[test]
fn write_error_line_with_special_chars() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "error: \"bad\" path\\n").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "error: \"bad\" path\\n\n");
}

// -------------------------------------------------------------------------
// stop_flag_path — the parent/child recording stop IPC contract
// -------------------------------------------------------------------------

#[test]
fn stop_flag_path_pins_the_exact_ipc_filename_pattern() {
    // The tray parent and the --select-record child both derive this path;
    // the pattern IS the IPC contract, so pin it byte-for-byte. Changing it
    // breaks stopping any recording started by an older binary.
    assert_eq!(
        stop_flag_path(1234),
        std::env::temp_dir().join("rustshot_stop_1234.flag")
    );
}

// -------------------------------------------------------------------------
// write_error_line — unicode content
// -------------------------------------------------------------------------

#[test]
fn write_error_line_handles_unicode() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "ошибка 日本語").unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("ошибка"));
    assert!(output.contains("日本語"));
    assert!(output.ends_with('\n'));
}

#[test]
fn window_capture_slot_admits_one_capture_at_a_time() {
    // Never run, and finished, both admit a new capture.
    assert!(window_capture_slot_is_free(None));
    assert!(window_capture_slot_is_free(Some(true)));

    // A capture still in flight blocks another: holding Alt+PrintScreen down
    // would otherwise spawn a thread per repeat, all contending for the same
    // Graphics Capture backend.
    assert!(!window_capture_slot_is_free(Some(false)));
}
