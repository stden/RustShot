//! `should_run_cli_mode` flag-combination tests.

use super::*;

#[test]
fn should_run_cli_mode_is_true_for_cli_flag() {
    let args = Args {
        cli: true,
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_list_flag() {
    let args = Args {
        list: true,
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_output_path() {
    let args = Args {
        output: Some(PathBuf::from("out.png")),
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_screen_selection() {
    let args = Args {
        screen: Some(1),
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_non_default_format() {
    let args = Args {
        format: CliFormat::Bmp,
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_false_for_select_only() {
    let args = Args {
        select: true,
        ..base_args()
    };
    assert!(!should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_when_cli_and_select_are_both_set() {
    let args = Args {
        cli: true,
        select: true,
        ..base_args()
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
    let args = base_args();
    assert!(should_run_cli_mode_with_format_explicit(&args, true));
}

#[test]
fn should_run_cli_mode_is_false_for_select_record_only() {
    let args = Args {
        select_record: true,
        ..base_args()
    };
    assert!(!should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_false_when_no_flags() {
    let args = base_args();
    assert!(!should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_true_when_all_cli_flags_set() {
    let args = Args {
        cli: true,
        output: Some(PathBuf::from("x.png")),
        list: true,
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_is_true_for_window_flag() {
    let args = Args {
        window: true,
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

// --- should_run_cli_mode: each flag in isolation ---

#[test]
fn should_run_cli_mode_only_screen_flag() {
    let args = Args {
        screen: Some(0),
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_only_output_flag() {
    let args = Args {
        output: Some(PathBuf::from("x.png")),
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

#[test]
fn should_run_cli_mode_only_jpg_format() {
    let args = Args {
        format: CliFormat::Jpg,
        ..base_args()
    };
    assert!(should_run_cli_mode(&args));
}

// --- should_run_cli_mode_with_format_explicit ---

#[test]
fn should_run_cli_mode_with_format_explicit_false_and_no_flags() {
    let args = base_args();
    assert!(!should_run_cli_mode_with_format_explicit(&args, false));
}

#[test]
fn should_run_cli_mode_with_format_explicit_window_flag_triggers_cli() {
    let args = Args {
        window: true,
        ..base_args()
    };
    assert!(should_run_cli_mode_with_format_explicit(&args, false));
}
