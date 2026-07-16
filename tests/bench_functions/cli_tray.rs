use rustshot::capture::Format;

// ---------------------------------------------------------------------------
// tray module — menu constants
// ---------------------------------------------------------------------------

#[test]
fn tray_menu_constants_are_non_empty() {
    assert!(!rustshot::tray::MENU_CAPTURE.is_empty());
    assert!(!rustshot::tray::MENU_OPEN_FOLDER.is_empty());
    assert!(!rustshot::tray::MENU_EXIT.is_empty());
    assert!(!rustshot::tray::MENU_CAPTURE_WINDOW.is_empty());
    assert!(!rustshot::tray::MENU_SETTINGS.is_empty());
    assert!(!rustshot::tray::MENU_START_RECORD.is_empty());
    assert!(!rustshot::tray::MENU_STOP_RECORD.is_empty());
}

#[test]
fn tray_menu_constants_all_unique() {
    let ids = [
        rustshot::tray::MENU_CAPTURE,
        rustshot::tray::MENU_OPEN_FOLDER,
        rustshot::tray::MENU_EXIT,
        rustshot::tray::MENU_CAPTURE_WINDOW,
        rustshot::tray::MENU_SETTINGS,
        rustshot::tray::MENU_START_RECORD,
        rustshot::tray::MENU_STOP_RECORD,
    ];
    for (i, a) in ids.iter().enumerate() {
        for (j, b) in ids.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "menu IDs at index {i} and {j} must differ");
            }
        }
    }
}

#[test]
fn tray_menu_constants_contain_no_whitespace() {
    let ids = [
        rustshot::tray::MENU_CAPTURE,
        rustshot::tray::MENU_OPEN_FOLDER,
        rustshot::tray::MENU_EXIT,
        rustshot::tray::MENU_CAPTURE_WINDOW,
        rustshot::tray::MENU_SETTINGS,
        rustshot::tray::MENU_START_RECORD,
        rustshot::tray::MENU_STOP_RECORD,
    ];
    for id in &ids {
        assert!(
            !id.contains(char::is_whitespace),
            "menu ID should have no whitespace: '{id}'"
        );
    }
}

#[test]
fn tray_menu_capture_value() {
    assert_eq!(rustshot::tray::MENU_CAPTURE, "capture");
}

#[test]
fn tray_menu_exit_value() {
    assert_eq!(rustshot::tray::MENU_EXIT, "exit");
}

#[test]
fn tray_menu_settings_value() {
    assert_eq!(rustshot::tray::MENU_SETTINGS, "settings");
}

#[test]
fn tray_menu_start_and_stop_record_are_distinct() {
    assert_ne!(
        rustshot::tray::MENU_START_RECORD,
        rustshot::tray::MENU_STOP_RECORD
    );
}

#[test]
fn tray_menu_capture_and_capture_window_are_distinct() {
    assert_ne!(
        rustshot::tray::MENU_CAPTURE,
        rustshot::tray::MENU_CAPTURE_WINDOW
    );
}

// ---------------------------------------------------------------------------
// cli module
// ---------------------------------------------------------------------------

/// Was `cli::settings_capture_format`. That mapper is gone — parsing a settings
/// format string is now the `Format` enum's own constructor. Same behaviour.
#[test]
fn settings_format_string_maps_to_supported_values() {
    assert_eq!(Format::from_settings_str("jpg"), Format::Jpg);
    assert_eq!(Format::from_settings_str("bmp"), Format::Bmp);
    assert_eq!(Format::from_settings_str("gif"), Format::Png);
}

#[test]
fn cli_format_flag_present_detects_explicit_forms() {
    assert!(rustshot::cli::cli_format_flag_present_in([
        "rustshot", "--format", "png"
    ]));
    assert!(rustshot::cli::cli_format_flag_present_in([
        "rustshot",
        "--format=jpg"
    ]));
    assert!(rustshot::cli::cli_format_flag_present_in([
        "rustshot", "-fbmp"
    ]));
}

#[test]
fn cli_format_flag_present_skips_output_values() {
    assert!(!rustshot::cli::cli_format_flag_present_in([
        "rustshot", "--output", "--format"
    ]));
    assert!(!rustshot::cli::cli_format_flag_present_in([
        "rustshot", "-o", "-fpng"
    ]));
}

#[test]
fn cli_output_extension_extracts_png() {
    assert_eq!(
        rustshot::cli::output_extension(Some(std::path::Path::new("/tmp/shot.png"))),
        Some("png")
    );
}

#[test]
fn cli_format_from_extension_supports_jpeg_alias() {
    assert!(matches!(
        rustshot::cli::format_from_extension("JPEG"),
        Some(rustshot::capture::Format::Jpg)
    ));
}

#[test]
fn cli_format_from_output_extension_returns_none_for_missing_extension() {
    assert!(
        rustshot::cli::format_from_output_extension(Some(std::path::Path::new("/tmp/shot")))
            .is_none()
    );
}

#[test]
fn cli_unsupported_output_extension_returns_unknown_extension() {
    assert_eq!(
        rustshot::cli::unsupported_output_extension(Some(std::path::Path::new("/tmp/shot.webp"))),
        Some("webp".to_string())
    );
}

#[test]
fn cli_effective_cli_format_prefers_explicit_format() {
    assert!(matches!(
        rustshot::cli::effective_cli_format(
            rustshot::capture::Format::Bmp,
            true,
            Some(std::path::Path::new("/tmp/shot.png")),
            rustshot::capture::Format::Jpg,
        ),
        rustshot::capture::Format::Bmp
    ));
}

#[test]
fn cli_effective_cli_format_uses_output_extension_before_settings() {
    assert!(matches!(
        rustshot::cli::effective_cli_format(
            rustshot::capture::Format::Png,
            false,
            Some(std::path::Path::new("/tmp/shot.bmp")),
            rustshot::capture::Format::Jpg,
        ),
        rustshot::capture::Format::Bmp
    ));
}

#[test]
fn cli_should_run_cli_mode_uses_any_trigger_flag() {
    assert!(rustshot::cli::should_run_cli_mode(
        false, false, false, false, false, true
    ));
    assert!(rustshot::cli::should_run_cli_mode(
        false, false, true, false, false, false
    ));
    assert!(!rustshot::cli::should_run_cli_mode(
        false, false, false, false, false, false
    ));
}

#[test]
fn cli_format_screen_list_output_indents_each_entry() {
    let output =
        rustshot::cli::format_screen_list_output(vec!["Screen 0: 1920x1080", "Screen 1: 1280x720"]);
    assert_eq!(
        output,
        "Available screens:\n  Screen 0: 1920x1080\n  Screen 1: 1280x720"
    );
}

#[test]
fn cli_selection_stderr_message_omits_cancelled_error() {
    assert_eq!(
        rustshot::cli::selection_stderr_message("Selection cancelled"),
        None
    );
}

#[test]
fn cli_write_error_line_appends_newline() {
    let mut buf = Vec::new();
    rustshot::cli::write_error_line(&mut buf, "boom").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "boom\n");
}

#[test]
fn cli_child_process_is_running_matches_pending_result() {
    assert!(rustshot::cli::child_process_is_running::<(), ()>(Ok(None)));
    assert!(!rustshot::cli::child_process_is_running::<(), ()>(Ok(
        Some(())
    )));
    assert!(!rustshot::cli::child_process_is_running::<(), ()>(Err(())));
}
