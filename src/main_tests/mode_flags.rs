//! `should_run_cli_mode` flag-combination tests.

use super::*;

#[test]
fn should_run_cli_mode_is_true_for_cli_flag() {
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
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_list_flag() {
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
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_output_path() {
    let args = Args {
        cli: false,
        select: false,
        output: Some(PathBuf::from("out.png")),
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_screen_selection() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: Some(1),
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_non_default_format() {
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
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_false_for_select_only() {
    let args = Args {
        cli: false,
        select: true,
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
    assert!(!should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_when_cli_and_select_are_both_set() {
    let args = Args {
        cli: true,
        select: true,
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
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_with_attached_short_png_format() {
    let args = Args::try_parse_from(["rustshot", "-fpng"]).unwrap();
    let format_explicit = cli_format_flag_present_in(["rustshot", "-fpng"]);
    assert!(should_run_cli_mode_with_format_explicit(
        &args,
        format_explicit
    ));
}

#[test]
fn should_run_cli_mode_with_format_explicit_true_for_explicit_png() {
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
        window: false,
    };
    assert!(should_run_cli_mode_with_format_explicit(&args, true));
}

#[test]
fn should_run_cli_mode_is_false_for_select_record_only() {
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
        select_record: true,
        window: false,
    };
    assert!(!should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_false_when_no_flags() {
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
        window: false,
    };
    assert!(!should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_true_when_all_cli_flags_set() {
    let args = Args {
        cli: true,
        select: false,
        output: Some(PathBuf::from("x.png")),
        format: CliFormat::Png,
        screen: None,
        list: true,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_window_flag() {
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
    assert!(should_run_cli_mode(&args));
}

// --- should_run_cli_mode: window flag ---

#[test]
fn should_run_cli_mode_window_only_is_true() {
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
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_false_for_all_defaults() {
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
        window: false,
    };
    assert!(!should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_false_for_select_record() {
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
        select_record: true,
        window: false,
    };
    assert!(!should_run_cli_mode(&args));
}

// --- should_run_cli_mode: each flag in isolation ---

#[test]
fn should_run_cli_mode_only_screen_flag() {
    let args = Args {
        cli: false,
        select: false,
        output: None,
        format: CliFormat::Png,
        screen: Some(0),
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_only_output_flag() {
    let args = Args {
        cli: false,
        select: false,
        output: Some(PathBuf::from("x.png")),
        format: CliFormat::Png,
        screen: None,
        list: false,
        list_windows: false,
        window_id: None,
        mcp: false,
        select_record: false,
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_only_jpg_format() {
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
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_only_bmp_format() {
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
        window: false,
    };
    assert!(should_run_cli_mode(&args));
}

// --- should_run_cli_mode_with_format_explicit ---

#[test]
fn should_run_cli_mode_with_format_explicit_false_and_no_flags() {
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
        window: false,
    };
    assert!(!should_run_cli_mode_with_format_explicit(&args, false));
}

#[test]
fn should_run_cli_mode_with_format_explicit_true_triggers_cli() {
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
        window: false,
    };
    assert!(should_run_cli_mode_with_format_explicit(&args, true));
}

#[test]
fn should_run_cli_mode_with_format_explicit_window_flag_triggers_cli() {
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
    assert!(should_run_cli_mode_with_format_explicit(&args, false));
}

// -------------------------------------------------------------------------
// should_run_cli_mode_with_format_explicit — window flag
// -------------------------------------------------------------------------

#[test]
fn should_run_cli_mode_with_format_explicit_false_and_window_triggers_cli() {
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
    assert!(should_run_cli_mode_with_format_explicit(&args, false));
}
