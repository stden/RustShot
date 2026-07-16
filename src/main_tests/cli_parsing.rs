//! CLI argument parsing tests: clap flags, defaults, and explicit format-flag detection.

use super::*;

#[test]
fn cli_parse_defaults_to_png_and_tray_mode() {
    let args = Args::parse_from(["rustshot"]);
    assert!(matches!(decide_app_mode(&args), AppMode::Tray));
    assert!(!should_run_cli_mode(&args));
    assert!(args.output.is_none());
    assert!(!args.list);
    assert!(matches!(args.format, CliFormat::Png));
}

#[test]
fn cli_parse_flags_force_cli_mode() {
    let args = Args::parse_from(["rustshot", "--cli"]);
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    assert!(should_run_cli_mode(&args));

    let args = Args::parse_from(["rustshot", "--list"]);
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    assert!(should_run_cli_mode(&args));
}

#[test]
fn cli_parse_select_mode() {
    let args = Args::parse_from(["rustshot", "--select"]);
    assert!(matches!(decide_app_mode(&args), AppMode::Selection));
}

#[test]
fn cli_parse_output_and_screen_options() {
    let args = Args::parse_from(["rustshot", "--output", "shot.png", "--screen", "2"]);
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    assert!(should_run_cli_mode(&args));
    assert_eq!(args.output, Some(PathBuf::from("shot.png")));
    assert_eq!(args.screen, Some(2));
}

#[test]
fn cli_parse_screen_only_forces_cli_mode() {
    let args = Args::parse_from(["rustshot", "--screen", "1"]);
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    assert!(should_run_cli_mode(&args));
}

#[test]
fn cli_parse_non_default_format_forces_cli_mode() {
    let args = Args::parse_from(["rustshot", "--format", "bmp"]);
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    assert!(should_run_cli_mode(&args));
}

#[test]
fn cli_format_flag_present_detects_short_and_long_forms() {
    assert!(cli_format_flag_present_in(["rustshot", "--format", "png"]));
    assert!(cli_format_flag_present_in(["rustshot", "-f", "jpg"]));
    assert!(cli_format_flag_present_in(["rustshot", "--format=bmp"]));
    assert!(!cli_format_flag_present_in(["rustshot", "--cli"]));
}

#[test]
fn cli_parse_attached_short_format_value() {
    let args = Args::try_parse_from(["rustshot", "-fjpg"]).unwrap();
    assert!(matches!(args.format, CliFormat::Jpg));
}

#[test]
fn cli_format_flag_present_detects_attached_short_form() {
    assert!(cli_format_flag_present_in(["rustshot", "-fjpg"]));
    assert!(cli_format_flag_present_in(["rustshot", "-fpng"]));
}

#[test]
fn cli_format_flag_present_ignores_hyphenated_output_values() {
    assert!(!cli_format_flag_present_in([
        "rustshot", "--output", "-foo.png",
    ]));
    assert!(!cli_format_flag_present_in([
        "rustshot",
        "-o",
        "-frames/shot.png",
    ]));
}

#[test]
fn cli_format_flag_present_ignores_similar_prefixes() {
    assert!(!cli_format_flag_present_in([
        "rustshot",
        "--formatting=png",
        "--formatter",
        "jpg"
    ]));
}

#[test]
fn cli_parse_format_jpg_is_parsed() {
    let args = Args::parse_from(["rustshot", "--cli", "--format", "jpg"]);
    assert!(matches!(args.format, CliFormat::Jpg));
}

#[test]
fn cli_parse_format_bmp_is_parsed() {
    let args = Args::parse_from(["rustshot", "--cli", "--format", "bmp"]);
    assert!(matches!(args.format, CliFormat::Bmp));
}

#[test]
fn cli_parse_screen_zero_is_parsed() {
    let args = Args::parse_from(["rustshot", "--screen", "0"]);
    assert_eq!(args.screen, Some(0));
}

#[test]
fn cli_short_flag_output() {
    let args = Args::parse_from(["rustshot", "-o", "snap.png"]);
    assert_eq!(args.output, Some(PathBuf::from("snap.png")));
}

#[test]
fn cli_short_flag_format() {
    let args = Args::parse_from(["rustshot", "-f", "bmp"]);
    assert!(matches!(args.format, CliFormat::Bmp));
}

#[test]
fn cli_short_flag_screen() {
    let args = Args::parse_from(["rustshot", "-s", "3"]);
    assert_eq!(args.screen, Some(3));
}

#[test]
fn cli_short_flag_list() {
    let args = Args::parse_from(["rustshot", "-l"]);
    assert!(args.list);
}

#[test]
fn cli_parse_short_output_with_format() {
    let args = Args::parse_from(["rustshot", "-o", "out.jpg", "-f", "jpg"]);
    assert_eq!(args.output, Some(PathBuf::from("out.jpg")));
    assert!(matches!(args.format, CliFormat::Jpg));
}

#[test]
fn cli_parse_screen_defaults_to_none() {
    let args = Args::parse_from(["rustshot"]);
    assert_eq!(args.screen, None);
}

#[test]
fn cli_parse_all_long_flags_combined() {
    let args = Args::parse_from([
        "rustshot", "--output", "out.bmp", "--format", "bmp", "--screen", "2", "--cli",
    ]);
    assert!(args.cli);
    assert_eq!(args.output, Some(PathBuf::from("out.bmp")));
    assert!(matches!(args.format, CliFormat::Bmp));
    assert_eq!(args.screen, Some(2));
}

#[test]
fn cli_parse_output_nested_path() {
    let args = Args::parse_from(["rustshot", "--output", "a/b/c.png"]);
    assert_eq!(args.output, Some(PathBuf::from("a/b/c.png")));
}

#[test]
fn cli_parse_select_record_flag_is_parsed() {
    let args = Args::parse_from(["rustshot", "--select-record"]);
    assert!(args.select_record);
    assert!(!args.select);
    assert!(!args.cli);
}

#[test]
fn cli_parse_select_record_and_format_combined() {
    let args = Args::parse_from(["rustshot", "--select-record", "--format", "jpg"]);
    assert!(args.select_record);
    assert!(matches!(args.format, CliFormat::Jpg));
}

#[test]
fn args_default_select_record_is_false() {
    let args = Args::parse_from(["rustshot"]);
    assert!(!args.select_record);
}

#[test]
fn cli_parse_version_flag_exists() {
    // --version should cause clap to exit with version info (we test it doesn't panic)
    let result = Args::try_parse_from(["rustshot", "--version"]);
    // clap returns an error (with kind DisplayVersion) for --version
    assert!(result.is_err());
}

#[test]
fn cli_parse_help_flag_exists() {
    let result = Args::try_parse_from(["rustshot", "--help"]);
    assert!(result.is_err()); // clap exits with help
}

// --- window capture CLI tests ---

#[test]
fn cli_parse_window_flag_is_parsed() {
    let args = Args::parse_from(["rustshot", "--window"]);
    assert!(args.window);
}

#[test]
fn cli_parse_window_short_flag() {
    let args = Args::parse_from(["rustshot", "-w"]);
    assert!(args.window);
}

#[test]
fn args_default_window_is_false() {
    let args = Args::parse_from(["rustshot"]);
    assert!(!args.window);
}

// --- CLI parse additional tests ---

#[test]
fn cli_parse_window_flag() {
    let args = Args::parse_from(["rustshot", "--window"]);
    assert!(args.window);
    assert!(should_run_cli_mode(&args));
}

#[test]
fn cli_parse_format_jpg() {
    let args = Args::parse_from(["rustshot", "--format", "jpg"]);
    assert_eq!(args.format, CliFormat::Jpg);
}

#[test]
fn cli_parse_short_output() {
    let args = Args::parse_from(["rustshot", "-o", "my_shot.png"]);
    assert_eq!(args.output, Some(PathBuf::from("my_shot.png")));
}

#[test]
fn cli_parse_short_screen() {
    let args = Args::parse_from(["rustshot", "-s", "0"]);
    assert_eq!(args.screen, Some(0));
}

#[test]
fn cli_parse_combined_flags() {
    let args = Args::parse_from([
        "rustshot", "--cli", "--format", "bmp", "--output", "out.bmp", "--screen", "1",
    ]);
    assert!(args.cli);
    assert_eq!(args.format, CliFormat::Bmp);
    assert_eq!(args.output, Some(PathBuf::from("out.bmp")));
    assert_eq!(args.screen, Some(1));
}

// --- CliFormat value_enum parsing ---

#[test]
fn cli_parse_invalid_format_is_error() {
    let result = Args::try_parse_from(["rustshot", "--format", "gif"]);
    assert!(result.is_err());
}

#[test]
fn cli_parse_format_is_case_sensitive() {
    // clap value_enum is case-sensitive: "PNG" is invalid, only "png" works
    let result = Args::try_parse_from(["rustshot", "--format", "PNG"]);
    assert!(result.is_err());
}

// --- CLI arg defaults ---

#[test]
fn args_defaults_all_false_and_none() {
    let args = Args::parse_from(["rustshot"]);
    assert!(!args.cli);
    assert!(!args.select);
    assert!(!args.window);
    assert!(!args.list);
    assert!(!args.select_record);
    assert_eq!(args.output, None);
    assert_eq!(args.screen, None);
    assert_eq!(args.format, CliFormat::Png);
}

#[test]
fn cli_parse_screen_large_value() {
    let args = Args::parse_from(["rustshot", "--screen", "999"]);
    assert_eq!(args.screen, Some(999));
}

// --- cli_format_flag_present_in edge cases ---

#[test]
fn cli_format_flag_present_in_skips_first_arg() {
    // First argument is the program name, should be skipped
    assert!(!cli_format_flag_present_in(["--format"]));
}

#[test]
fn cli_format_flag_present_in_format_equals_value() {
    assert!(cli_format_flag_present_in(["rustshot", "--format=png"]));
}

#[test]
fn cli_format_flag_present_in_empty_args() {
    assert!(!cli_format_flag_present_in(Vec::<String>::new()));
}

// -------------------------------------------------------------------------
// cli_format_flag_present_in edge cases
// -------------------------------------------------------------------------

#[test]
fn cli_format_flag_present_in_only_program_name_returns_false() {
    assert!(!cli_format_flag_present_in(["rustshot"]));
}

#[test]
fn cli_format_flag_present_in_f_shorthand_with_value() {
    assert!(cli_format_flag_present_in(["rustshot", "-fbmp"]));
}

#[test]
fn cli_format_flag_present_in_skips_value_after_output_flag() {
    // "--output png" should NOT treat "png" as a format flag
    assert!(!cli_format_flag_present_in(
        ["rustshot", "--output", "png",]
    ));
}

#[test]
fn cli_format_flag_present_in_skips_value_after_o_flag() {
    assert!(!cli_format_flag_present_in(["rustshot", "-o", "--format",]));
}

#[test]
fn cli_format_flag_present_in_skips_value_after_screen_flag() {
    assert!(!cli_format_flag_present_in(["rustshot", "--screen", "-f",]));
}

#[test]
fn cli_format_flag_present_in_skips_value_after_s_flag() {
    assert!(!cli_format_flag_present_in(["rustshot", "-s", "--format",]));
}

#[test]
fn cli_format_flag_present_in_format_after_skipped_value() {
    // The value after --output is skipped, then --format is detected
    assert!(cli_format_flag_present_in([
        "rustshot", "--output", "file.png", "--format", "bmp",
    ]));
}

#[test]
fn cli_format_flag_present_in_format_equals_bmp() {
    assert!(cli_format_flag_present_in(["rustshot", "--format=bmp"]));
}

// -------------------------------------------------------------------------
// cli_format_flag_present_in — double flags
// -------------------------------------------------------------------------

#[test]
fn cli_format_flag_present_in_no_format_with_multiple_flags() {
    assert!(!cli_format_flag_present_in([
        "rustshot", "--cli", "--list", "--window",
    ]));
}

// -------------------------------------------------------------------------
// Args::parse_from — combined flags
// -------------------------------------------------------------------------

#[test]
fn cli_parse_all_flags_at_once() {
    let args = Args::parse_from([
        "rustshot", "--cli", "--output", "out.bmp", "--format", "bmp", "--screen", "0",
    ]);
    assert!(args.cli);
    assert_eq!(args.output, Some(PathBuf::from("out.bmp")));
    assert!(matches!(args.format, CliFormat::Bmp));
    assert_eq!(args.screen, Some(0));
}

// -------------------------------------------------------------------------
// --list-windows / --window-id / --mcp parsing
// -------------------------------------------------------------------------

#[test]
fn cli_parse_list_windows_flag_is_parsed() {
    let args = Args::parse_from(["rustshot", "--list-windows"]);
    assert!(args.list_windows);
}

#[test]
fn cli_parse_window_id_value_is_parsed() {
    let args = Args::parse_from(["rustshot", "--window-id", "42"]);
    assert_eq!(args.window_id, Some(42));
}

#[test]
fn cli_parse_window_id_non_numeric_is_error() {
    let result = Args::try_parse_from(["rustshot", "--window-id", "abc"]);
    assert!(result.is_err());
}

#[test]
fn cli_parse_mcp_flag_is_parsed() {
    let args = Args::parse_from(["rustshot", "--mcp"]);
    assert!(args.mcp);
}

#[test]
fn args_default_list_windows_window_id_and_mcp_are_off() {
    let args = Args::parse_from(["rustshot"]);
    assert!(!args.list_windows);
    assert_eq!(args.window_id, None);
    assert!(!args.mcp);
}
