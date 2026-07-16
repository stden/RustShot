//! `run_cli_with_capture` window-capture and window-listing tests.

use super::*;

#[test]
fn run_cli_list_windows_returns_window_listing() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: true,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };

    let output =
        run_cli_with_capture_test(args, |_, _, _| unreachable!("capture fn should not run"))
            .unwrap();
    // Empty on a headless CI host, populated on a desktop; both are valid.
    assert!(
        output == "No open windows found." || output.starts_with("Open windows:"),
        "unexpected window listing: {output:?}"
    );
}

#[test]
fn run_cli_window_id_unknown_returns_error() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        // A handle value that is never a live window: capture must fail
        // cleanly rather than blit garbage.
        window_id: Some(u64::MAX),
        mcp: false,
        select_record: false,
        window: false,
    };

    let result =
        run_cli_with_capture_test(args, |_, _, _| unreachable!("capture fn should not run"));
    assert!(result.is_err(), "expected an error, got {result:?}");
}

#[test]
fn run_cli_with_capture_window_flag_calls_window_fn() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let capture_called = Cell::new(false);
    let window_called = Cell::new(false);
    let result = run_cli_with_capture(
        args,
        |_, _, _| {
            capture_called.set(true);
            Ok(PathBuf::from("screen.png"))
        },
        |_, _| {
            window_called.set(true);
            Ok(PathBuf::from("window.png"))
        },
    )
    .unwrap();
    assert!(!capture_called.get(), "capture fn should not be called");
    assert!(window_called.get(), "window fn should be called");
    assert!(result.contains("window.png"));
}

#[test]
fn run_cli_with_capture_no_window_flag_calls_capture_fn() {
    let args = Args {
        cli: true,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    let capture_called = Cell::new(false);
    let window_called = Cell::new(false);
    let result = run_cli_with_capture(
        args,
        |_, _, _| {
            capture_called.set(true);
            Ok(PathBuf::from("screen.png"))
        },
        |_, _| {
            window_called.set(true);
            Ok(PathBuf::from("window.png"))
        },
    )
    .unwrap();
    assert!(capture_called.get(), "capture fn should be called");
    assert!(!window_called.get(), "window fn should not be called");
    assert!(result.contains("screen.png"));
}

#[test]
fn run_cli_with_capture_window_error_propagates() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| Ok(PathBuf::from("x.png")),
        |_, _| Err("window not found".to_string()),
    );
    assert_eq!(result.unwrap_err(), "window not found");
}

#[test]
fn run_cli_with_capture_window_output_path_forwarded() {
    let args = Args {
        cli: false,
        select: false,
        output: Some(PathBuf::from("custom_window.png")),
        format: CliFormat::Jpg,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| unreachable!("should not call capture"),
        |output, format| {
            assert_eq!(output, Some(PathBuf::from("custom_window.png")));
            assert!(matches!(format, capture::Format::Jpg));
            Ok(PathBuf::from("custom_window.png"))
        },
    )
    .unwrap();
    assert!(result.contains("custom_window.png"));
}

// --- run_cli_with_capture: window with different formats ---

#[test]
fn run_cli_with_capture_window_bmp_format() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Bmp,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| unreachable!("should not call capture"),
        |_output, format| {
            assert!(matches!(format, capture::Format::Bmp));
            Ok(PathBuf::from("window.bmp"))
        },
    )
    .unwrap();
    assert!(result.contains("window.bmp"));
}

#[test]
fn run_cli_with_capture_window_none_output() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| unreachable!("should not call capture"),
        |output, _format| {
            assert!(output.is_none());
            Ok(PathBuf::from("auto_window.png"))
        },
    )
    .unwrap();
    assert!(result.contains("auto_window.png"));
}

// --- run_cli_with_capture: list takes priority over window ---

#[test]
fn run_cli_with_capture_list_takes_priority_over_window() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: true,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| unreachable!("should not call capture"),
        |_, _| unreachable!("should not call window capture"),
    )
    .unwrap();
    assert!(result.starts_with("Available screens:"));
}

// --- run_cli_with_capture: window with jpg and png formats ---

#[test]
fn run_cli_with_capture_window_jpg_format() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Jpg,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| unreachable!("should not call capture"),
        |_output, format| {
            assert!(matches!(format, capture::Format::Jpg));
            Ok(PathBuf::from("window.jpg"))
        },
    )
    .unwrap();
    assert!(result.contains("window.jpg"));
}

#[test]
fn run_cli_with_capture_window_png_format() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| unreachable!("should not call capture"),
        |_output, format| {
            assert!(matches!(format, capture::Format::Png));
            Ok(PathBuf::from("window.png"))
        },
    )
    .unwrap();
    assert!(result.contains("window.png"));
}

#[test]
fn run_cli_with_capture_window_error_preserves_exact_message() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: true,
    };
    let result = run_cli_with_capture(
        args,
        |_, _, _| unreachable!(),
        |_, _| Err("Active window capture is only supported on Windows".to_string()),
    );
    assert_eq!(
        result.unwrap_err(),
        "Active window capture is only supported on Windows"
    );
}
