//! `run_cli_with_capture_using_settings` tests: settings/CLI/extension format precedence and dispatch.

use super::*;

#[test]
fn run_cli_with_capture_using_settings_uses_settings_format_when_flag_absent() {
    let args = Args {
        cli: true,
        ..base_args()
    };
    let settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };
    let called = Cell::new(false);
    run_cli_with_capture_using_settings(
        args,
        &settings,
        false,
        |_output, actual_format, _screen| {
            called.set(true);
            assert!(matches!(actual_format, capture::Format::Bmp));
            Ok(PathBuf::from("shot.bmp"))
        },
        |_, _| unreachable!("window capture should not run"),
    )
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_using_settings_prefers_explicit_cli_format() {
    let args = Args {
        cli: true,
        format: CliFormat::Jpg,
        ..base_args()
    };
    let settings = settings::Settings {
        default_format: Format::Bmp,
        ..settings::Settings::default()
    };
    let called = Cell::new(false);
    run_cli_with_capture_using_settings(
        args,
        &settings,
        true,
        |_output, actual_format, _screen| {
            called.set(true);
            assert!(matches!(actual_format, capture::Format::Jpg));
            Ok(PathBuf::from("shot.jpg"))
        },
        |_, _| unreachable!("window capture should not run"),
    )
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_using_settings_prefers_output_extension_when_flag_is_absent() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("shot.bmp")),
        ..base_args()
    };
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let called = Cell::new(false);

    run_cli_with_capture_using_settings(
        args,
        &settings,
        false,
        |actual_output, actual_format, _screen| {
            called.set(true);
            assert_eq!(actual_output, Some(PathBuf::from("shot.bmp")));
            assert!(matches!(actual_format, capture::Format::Bmp));
            Ok(PathBuf::from("shot.bmp"))
        },
        |_, _| unreachable!("window capture should not run"),
    )
    .unwrap();

    assert!(called.get());
}

#[test]
fn run_cli_with_capture_using_settings_output_without_extension_uses_settings_format() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("shot")),
        ..base_args()
    };
    let settings = settings::Settings {
        default_format: Format::Jpg,
        ..settings::Settings::default()
    };
    let called = Cell::new(false);

    run_cli_with_capture_using_settings(
        args,
        &settings,
        false,
        |actual_output, actual_format, _screen| {
            called.set(true);
            assert_eq!(actual_output, Some(PathBuf::from("shot")));
            assert!(matches!(actual_format, capture::Format::Jpg));
            Ok(PathBuf::from("shot"))
        },
        |_, _| unreachable!("window capture should not run"),
    )
    .unwrap();

    assert!(called.get());
}

#[test]
fn run_cli_with_capture_using_settings_rejects_unsupported_output_extension_without_flag() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("shot.gif")),
        ..base_args()
    };

    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        false,
        |_, _, _| unreachable!("capture handler should not run"),
        |_, _| unreachable!("window capture should not run"),
    );

    assert_eq!(
        result,
        Err("Unsupported output extension '.gif'. Use png, jpg, or bmp, or pass --format explicitly.".to_string())
    );
}

#[test]
fn run_cli_with_capture_using_settings_explicit_format_allows_unsupported_extension() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("shot.gif")),
        format: CliFormat::Bmp,
        ..base_args()
    };
    let called = Cell::new(false);

    run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        true,
        |actual_output, actual_format, _screen| {
            called.set(true);
            assert_eq!(actual_output, Some(PathBuf::from("shot.gif")));
            assert!(matches!(actual_format, capture::Format::Bmp));
            Ok(PathBuf::from("shot.gif"))
        },
        |_, _| unreachable!("window capture should not run"),
    )
    .unwrap();

    assert!(called.get());
}

#[test]
fn run_cli_with_capture_using_settings_propagates_capture_error() {
    let args = Args {
        cli: true,
        screen: Some(2),
        ..base_args()
    };

    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        false,
        |_, _, actual_screen| {
            assert_eq!(actual_screen, Some(2));
            Err("capture boom".to_string())
        },
        |_, _| unreachable!("window capture should not run"),
    );

    assert_eq!(result, Err("capture boom".to_string()));
}

#[test]
fn run_cli_with_capture_using_settings_list_skips_capture_and_window_handlers() {
    let args = Args {
        list: true,
        ..base_args()
    };

    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        false,
        |_, _, _| unreachable!("capture handler should not run when listing screens"),
        |_, _| unreachable!("window handler should not run when listing screens"),
    )
    .unwrap();

    assert!(result.starts_with("Available screens:"));
}

#[test]
fn run_cli_with_capture_using_settings_list_skips_unsupported_output_extension_validation() {
    let args = Args {
        output: Some(PathBuf::from("listing.gif")),
        list: true,
        ..base_args()
    };

    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        false,
        |_, _, _| unreachable!("capture handler should not run when listing screens"),
        |_, _| unreachable!("window handler should not run when listing screens"),
    )
    .unwrap();

    assert!(result.starts_with("Available screens:"));
}

#[test]
fn run_cli_with_capture_using_settings_list_takes_priority_over_window_mode() {
    let args = Args {
        output: Some(PathBuf::from("window.gif")),
        format: CliFormat::Bmp,
        screen: Some(3),
        list: true,
        window: true,
        ..base_args()
    };

    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        true,
        |_, _, _| unreachable!("capture handler should not run when listing screens"),
        |_, _| unreachable!("window handler should not run when listing screens"),
    )
    .unwrap();

    assert!(result.starts_with("Available screens:"));
}

#[test]
fn run_cli_with_capture_using_settings_uses_window_handler_for_window_mode() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("window-shot.bmp")),
        screen: Some(7),
        window: true,
        ..base_args()
    };
    let called = Cell::new(false);

    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings {
            default_format: Format::Bmp,
            ..settings::Settings::default()
        },
        false,
        |_, _, _| unreachable!("screen capture handler should not run in window mode"),
        |actual_output, actual_format| {
            called.set(true);
            assert_eq!(actual_output, Some(PathBuf::from("window-shot.bmp")));
            assert!(matches!(actual_format, capture::Format::Bmp));
            Ok(PathBuf::from("window-shot.bmp"))
        },
    )
    .unwrap();

    assert!(called.get());
    assert_eq!(result, "Screenshot saved to: window-shot.bmp");
}

#[test]
fn run_cli_with_capture_using_settings_propagates_window_error() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("window-shot.png")),
        format: CliFormat::Jpg,
        screen: Some(5),
        window: true,
        ..base_args()
    };

    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        true,
        |_, _, _| unreachable!("screen capture handler should not run in window mode"),
        |actual_output, actual_format| {
            assert_eq!(actual_output, Some(PathBuf::from("window-shot.png")));
            assert!(matches!(actual_format, capture::Format::Jpg));
            Err("window boom".to_string())
        },
    );

    assert_eq!(result, Err("window boom".to_string()));
}

// -------------------------------------------------------------------------
// run_cli_with_capture_using_settings — unsupported ext with explicit format
// -------------------------------------------------------------------------

#[test]
fn run_cli_with_capture_using_settings_allows_unsupported_ext_when_format_explicit() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("shot.gif")),
        format: CliFormat::Bmp,
        window: true,
        ..base_args()
    };
    let called = Cell::new(false);
    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        true, // format_explicit = true, so unsupported ext check is bypassed
        |_, _, _| unreachable!("screen capture should not run in window mode"),
        |_, actual_format| {
            called.set(true);
            assert!(matches!(actual_format, capture::Format::Bmp));
            Ok(PathBuf::from("shot.gif"))
        },
    );
    assert!(called.get());
    assert!(result.is_ok());
}

#[test]
fn run_cli_with_capture_using_settings_window_error_message_contains_path() {
    let args = Args {
        cli: true,
        window: true,
        ..base_args()
    };
    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        false,
        |_, _, _| unreachable!(),
        |_, _| Err("no foreground window".to_string()),
    );
    assert_eq!(result, Err("no foreground window".to_string()));
}

// -------------------------------------------------------------------------
// run_cli_with_capture_using_settings — window mode uses output extension
// -------------------------------------------------------------------------

#[test]
fn run_cli_with_capture_using_settings_window_mode_uses_output_extension() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("capture.jpg")),
        window: true,
        ..base_args()
    };
    let called = Cell::new(false);
    let result = run_cli_with_capture_using_settings(
        args,
        &settings::Settings::default(),
        false, // not explicit — will use output extension
        |_, _, _| unreachable!("screen capture should not run in window mode"),
        |_, actual_format| {
            called.set(true);
            // output is "capture.jpg" → format_from_output_extension → Jpg
            assert!(matches!(actual_format, capture::Format::Jpg));
            Ok(PathBuf::from("capture.jpg"))
        },
    );
    assert!(called.get());
    assert!(result.is_ok());
}
