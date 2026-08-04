//! CLI argument parsing tests: clap flags, defaults, and explicit format-flag detection.

use super::*;

#[test]
fn cli_parse_bare_invocation_sets_every_default() {
    let args = Args::parse_from(["rustshot"]);

    assert!(matches!(decide_app_mode(&args), AppMode::Tray));
    assert!(!should_run_cli_mode(&args));
    assert!(!args.cli);
    assert!(!args.select);
    assert!(!args.select_record);
    assert!(!args.window);
    assert!(!args.list);
    assert!(!args.list_windows);
    assert!(!args.mcp);
    assert_eq!(args.output, None);
    assert_eq!(args.screen, None);
    assert_eq!(args.window_id, None);
    assert_eq!(args.format, CliFormat::Png);
}

#[test]
fn cli_parse_combined_long_options_and_enters_cli_mode() {
    let args = Args::parse_from([
        "rustshot",
        "--cli",
        "--output",
        "a/b/out.bmp",
        "--format",
        "bmp",
        "--screen",
        "2",
    ]);

    assert!(args.cli);
    assert_eq!(args.output, Some(PathBuf::from("a/b/out.bmp")));
    assert_eq!(args.format, CliFormat::Bmp);
    assert_eq!(args.screen, Some(2));
    assert!(matches!(decide_app_mode(&args), AppMode::Cli));
    assert!(should_run_cli_mode(&args));
}

#[test]
fn cli_parse_combined_short_options() {
    let args = Args::parse_from([
        "rustshot", "-o", "out.jpg", "-f", "jpg", "-s", "0", "-l", "-w",
    ]);

    assert_eq!(args.output, Some(PathBuf::from("out.jpg")));
    assert_eq!(args.format, CliFormat::Jpg);
    assert_eq!(args.screen, Some(0));
    assert!(args.list);
    assert!(args.window);
}

#[test]
fn cli_parse_special_modes() {
    let selection = Args::parse_from(["rustshot", "--select"]);
    assert!(matches!(decide_app_mode(&selection), AppMode::Selection));

    let recording = Args::parse_from(["rustshot", "--select-record", "--format", "jpg"]);
    assert!(recording.select_record);
    assert!(!recording.select);
    assert_eq!(recording.format, CliFormat::Jpg);
    assert!(matches!(decide_app_mode(&recording), AppMode::SelectRecord));
}

#[test]
fn cli_parse_window_and_mcp_options() {
    let args = Args::parse_from([
        "rustshot",
        "--window",
        "--list-windows",
        "--window-id",
        "42",
        "--mcp",
    ]);

    assert!(args.window);
    assert!(args.list_windows);
    assert_eq!(args.window_id, Some(42));
    assert!(args.mcp);
}

#[test]
fn cli_parse_accepts_every_supported_format_form() {
    for (arguments, expected) in [
        (vec!["rustshot", "--format", "png"], CliFormat::Png),
        (vec!["rustshot", "--format", "jpg"], CliFormat::Jpg),
        (vec!["rustshot", "--format", "bmp"], CliFormat::Bmp),
        (vec!["rustshot", "-fjpg"], CliFormat::Jpg),
    ] {
        let args = Args::try_parse_from(arguments).unwrap();
        assert_eq!(args.format, expected);
    }
}

#[test]
fn cli_parse_rejects_invalid_typed_values() {
    for arguments in [
        ["rustshot", "--format", "gif"],
        ["rustshot", "--format", "PNG"],
        ["rustshot", "--window-id", "abc"],
    ] {
        assert!(Args::try_parse_from(arguments).is_err());
    }
}

#[test]
fn cli_format_flag_detection_covers_forms_skipped_values_and_absence() {
    let cases = [
        (vec!["rustshot", "--format", "png"], true),
        (vec!["rustshot", "-f", "jpg"], true),
        (vec!["rustshot", "--format=bmp"], true),
        (vec!["rustshot", "-fjpg"], true),
        (
            vec!["rustshot", "--output", "file.png", "--format", "bmp"],
            true,
        ),
        (vec!["rustshot"], false),
        (Vec::<&str>::new(), false),
        (vec!["--format"], false),
        (vec!["rustshot", "--cli", "--list", "--window"], false),
        (vec!["rustshot", "--output", "-foo.png"], false),
        (vec!["rustshot", "-o", "--format"], false),
        (vec!["rustshot", "--screen", "-f"], false),
        (
            vec!["rustshot", "--formatting=png", "--formatter", "jpg"],
            false,
        ),
    ];

    for (arguments, expected) in cases {
        assert_eq!(cli_format_flag_present_in(arguments), expected);
    }
}
