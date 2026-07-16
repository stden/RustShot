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
fn register_hotkey_with_wraps_custom_display_error() {
    use global_hotkey::hotkey::Code;

    struct FakeError;

    impl std::fmt::Display for FakeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "custom registration failure")
        }
    }

    let hotkey = HotKey::new(None, Code::PrintScreen);
    let err = register_hotkey_with(|_| Err(FakeError), hotkey, "PrintScreen").unwrap_err();

    assert!(err.contains("PrintScreen"));
    assert!(err.contains("custom registration failure"));
}

#[test]
fn write_error_line_appends_newline() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "Selection error: boom").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "Selection error: boom\n");
}

#[test]
fn write_error_line_empty_message_writes_single_newline() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "\n");
}

#[test]
fn write_error_line_preserves_whitespace_message() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "  ").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "  \n");
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
fn open_log_file_creates_new_empty_file() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = std::env::temp_dir().join(format!("rustshot_log_new_file_{unique}"));
    let path = base.join("rustshot.log");

    let file = open_log_file(&path).unwrap();
    drop(file);

    assert!(path.exists(), "new log file should exist");
    assert_eq!(
        std::fs::metadata(&path).unwrap().len(),
        0,
        "newly created log file should be empty"
    );

    let _ = std::fs::remove_file(&path);
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

// --- open_screenshots_folder_with_handlers additional tests ---

#[test]
fn open_screenshots_folder_with_handlers_passes_correct_dir() {
    let opened = Cell::new(false);
    open_screenshots_folder_with_handlers(
        |_| Ok(()),
        |dir| {
            opened.set(true);
            assert_eq!(
                dir.file_name().and_then(|n| n.to_str()),
                Some("RustShot"),
                "should open the RustShot folder"
            );
        },
    );
    assert!(opened.get());
}

#[test]
fn open_screenshots_folder_with_handlers_create_dir_receives_rustshot_path() {
    let called = Cell::new(false);
    open_screenshots_folder_with_handlers(
        |dir| {
            called.set(true);
            assert!(dir.ends_with("RustShot"));
            Ok(())
        },
        |_| {},
    );
    assert!(called.get());
}

#[test]
fn open_screenshots_folder_with_handlers_can_be_called_twice() {
    let open_count = Cell::new(0u32);
    open_screenshots_folder_with_handlers(
        |_| Ok(()),
        |_| {
            open_count.set(open_count.get() + 1);
        },
    );
    open_screenshots_folder_with_handlers(
        |_| Ok(()),
        |_| {
            open_count.set(open_count.get() + 1);
        },
    );
    assert_eq!(
        open_count.get(),
        2,
        "open_dir should be called once per invocation"
    );
}

// --- open_screenshots_folder_with_handlers path ---

#[test]
fn open_screenshots_folder_with_handlers_passes_rustshot_dir() {
    let received_path = std::cell::RefCell::new(None::<PathBuf>);
    open_screenshots_folder_with_handlers(
        |_| Ok(()),
        |dir| {
            received_path.replace(Some(dir.to_path_buf()));
        },
    );
    let path = received_path.borrow();
    let path = path.as_ref().expect("open_dir should be called");
    assert_eq!(path.file_name().and_then(|n| n.to_str()), Some("RustShot"));
}

// -------------------------------------------------------------------------
// open_screenshot_directory (Linux) — does not panic
// -------------------------------------------------------------------------

#[cfg(target_os = "linux")]
#[test]
fn open_screenshot_directory_does_not_panic() {
    // xdg-open may not be available in CI but should not panic
    let path = std::path::Path::new("/tmp");
    open_screenshot_directory(path);
}

// --- open_screenshots_folder_with_handlers: error message ---

#[test]
fn open_screenshots_folder_with_handlers_create_error_skips_open() {
    let open_called = Cell::new(false);
    open_screenshots_folder_with_handlers(
        |_| Err("permission denied".to_string()),
        |_| {
            open_called.set(true);
        },
    );
    assert!(
        !open_called.get(),
        "open_dir must not be called when create_dir fails"
    );
}

#[test]
fn child_process_is_running_true_for_ok_none() {
    let running: Result<Option<()>, ()> = Ok(None);
    assert!(child_process_is_running(running));
}

#[test]
fn child_process_is_running_false_for_finished_child() {
    let finished: Result<Option<()>, ()> = Ok(Some(()));
    assert!(!child_process_is_running(finished));
}

#[test]
fn child_process_is_running_false_for_try_wait_error() {
    let errored: Result<Option<()>, ()> = Err(());
    assert!(!child_process_is_running(errored));
}

// --- write_error_line edge cases ---

#[test]
fn write_error_line_with_special_chars() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "error: \"bad\" path\\n").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "error: \"bad\" path\\n\n");
}

// -------------------------------------------------------------------------
// open_screenshots_folder_at_with_handlers edge case
// -------------------------------------------------------------------------

#[test]
fn open_screenshots_folder_at_with_handlers_calls_both_on_success() {
    let created = Cell::new(false);
    let opened = Cell::new(false);
    let dir = std::env::temp_dir();
    open_screenshots_folder_at_with_handlers(
        &dir,
        |_| {
            created.set(true);
            Ok(())
        },
        |_| opened.set(true),
    );
    assert!(created.get());
    assert!(opened.get());
}

// -------------------------------------------------------------------------
// child_process_is_running — exhaustive pattern coverage
// -------------------------------------------------------------------------

#[test]
fn child_process_is_running_ok_some_means_exited() {
    assert!(!child_process_is_running(Ok::<Option<i32>, String>(Some(
        42
    ))));
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
