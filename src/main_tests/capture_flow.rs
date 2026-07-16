//! `run_cli_with_capture` tests: format/screen/output forwarding and error paths.

use super::*;

#[test]
fn run_cli_with_capture_uses_selected_output_and_format() {
    let output_path = PathBuf::from("custom.png");
    let args = Args {
        output: Some(output_path.clone()),
        format: CliFormat::Jpg,
        screen: Some(1),
        ..base_args()
    };
    let called = Cell::new(false);
    let path_for_assert = output_path.clone();

    let result = run_cli_with_capture_test(args, |actual_output, actual_format, _actual_screen| {
        called.set(true);
        assert_eq!(actual_output, Some(path_for_assert.clone()));
        assert!(matches!(actual_format, capture::Format::Jpg));
        Ok(output_path.clone())
    })
    .unwrap();

    assert_eq!(
        result,
        format!("Screenshot saved to: {}", output_path.display())
    );
    assert!(called.get());
}

#[test]
fn run_cli_from_args_roundtrip_format() {
    assert!(matches!(
        capture::Format::from(CliFormat::Png),
        capture::Format::Png
    ));
    assert!(matches!(
        capture::Format::from(CliFormat::Bmp),
        capture::Format::Bmp
    ));
}

#[test]
fn run_cli_with_capture_reports_capture_error() {
    let args = base_args();

    let result = run_cli_with_capture_test(args, |_, _, _| Err("capture failed".to_string()));
    assert_eq!(result, Err("capture failed".to_string()));
}

// --- run_cli_with_capture additional tests ---

#[test]
fn run_cli_with_capture_passes_screen_index_correctly() {
    let args = Args {
        output: Some(PathBuf::from("custom.png")),
        format: CliFormat::Jpg,
        screen: Some(1),
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_output, _format, actual_screen| {
        called.set(true);
        assert_eq!(actual_screen, Some(1));
        Ok(PathBuf::from("custom.png"))
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_screen_none_passes_none() {
    let args = Args {
        screen: None,
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_output, _format, actual_screen| {
        called.set(true);
        assert_eq!(actual_screen, None);
        Ok(PathBuf::from("shot.png"))
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_bmp_format_is_forwarded() {
    let args = Args {
        format: CliFormat::Bmp,
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_output, actual_format, _screen| {
        called.set(true);
        assert!(matches!(actual_format, capture::Format::Bmp));
        Ok(PathBuf::from("shot.bmp"))
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_png_format_is_forwarded() {
    let args = Args {
        format: CliFormat::Png,
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_output, actual_format, _screen| {
        called.set(true);
        assert!(matches!(actual_format, capture::Format::Png));
        Ok(PathBuf::from("shot.png"))
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_output_path_is_forwarded() {
    let output_path = PathBuf::from("my_screenshot.jpg");
    let args = Args {
        output: Some(output_path.clone()),
        format: CliFormat::Jpg,
        ..base_args()
    };
    let called = Cell::new(false);
    let expected = output_path.clone();
    run_cli_with_capture_test(args, |actual_output, _format, _screen| {
        called.set(true);
        assert_eq!(actual_output, Some(expected.clone()));
        Ok(output_path.clone())
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_result_message_contains_path() {
    let args = base_args();
    let result =
        run_cli_with_capture_test(args, |_, _, _| Ok(PathBuf::from("my_output.png"))).unwrap();
    assert!(result.contains("my_output.png"));
    assert!(result.starts_with("Screenshot saved to:"));
}

#[test]
fn run_cli_with_capture_jpg_format_is_forwarded() {
    let args = Args {
        format: CliFormat::Jpg,
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_output, actual_format, _screen| {
        called.set(true);
        assert!(matches!(actual_format, capture::Format::Jpg));
        Ok(PathBuf::from("shot.jpg"))
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_result_starts_with_screenshot_saved() {
    let args = base_args();
    let result = run_cli_with_capture_test(args, |_, _, _| Ok(PathBuf::from("out.png"))).unwrap();
    assert!(result.starts_with("Screenshot saved to:"));
}

#[test]
fn run_cli_with_capture_error_passthrough_preserves_message() {
    let args = base_args();
    let result = run_cli_with_capture_test(args, |_, _, _| Err("disk full".to_string()));
    assert_eq!(result.unwrap_err(), "disk full");
}

#[test]
fn run_cli_with_capture_result_message_display_format() {
    // Result should be "Screenshot saved to: <path>" with a space after the colon
    let args = base_args();
    let result = run_cli_with_capture_test(args, |_, _, _| Ok(PathBuf::from("test.png"))).unwrap();
    assert_eq!(result, "Screenshot saved to: test.png");
}

#[test]
fn run_cli_with_capture_capture_fn_called_exactly_once() {
    let call_count = Cell::new(0u32);
    let args = base_args();
    let _ = run_cli_with_capture_test(args, |_, _, _| {
        call_count.set(call_count.get() + 1);
        Ok(PathBuf::from("shot.png"))
    });
    assert_eq!(
        call_count.get(),
        1,
        "capture fn should be called exactly once"
    );
}

#[test]
fn run_cli_with_capture_output_none_passes_none() {
    let args = Args {
        cli: true,
        output: None,
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |output, _, _| {
        called.set(true);
        assert!(output.is_none());
        Ok(PathBuf::from("shot.png"))
    })
    .unwrap();
    assert!(called.get());
}

// --- run_cli_with_capture: screen index forwarding ---

#[test]
fn run_cli_with_capture_screen_index_0_forwarded() {
    let args = Args {
        cli: true,
        screen: Some(0),
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_, _, screen| {
        called.set(true);
        assert_eq!(screen, Some(0));
        Ok(PathBuf::from("shot.png"))
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_screen_index_5_forwarded() {
    let args = Args {
        cli: true,
        screen: Some(5),
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_, _, screen| {
        called.set(true);
        assert_eq!(screen, Some(5));
        Ok(PathBuf::from("shot.png"))
    })
    .unwrap();
    assert!(called.get());
}

// --- run_cli_with_capture: error from window capture vs screen capture ---

#[test]
fn run_cli_with_capture_capture_error_preserves_exact_message() {
    let args = Args {
        cli: true,
        ..base_args()
    };
    let result = run_cli_with_capture_test(args, |_, _, _| {
        Err("Screen index 99 is out of range (0-1)".to_string())
    });
    assert_eq!(result.unwrap_err(), "Screen index 99 is out of range (0-1)");
}
