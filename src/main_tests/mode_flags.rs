//! `should_run_cli_mode` flag-combination tests.

use super::*;

#[test]
fn should_run_cli_mode_covers_each_trigger_and_idle_case() {
    let cases = [
        (base_args(), false),
        (
            Args {
                select: true,
                ..base_args()
            },
            false,
        ),
        (
            Args {
                select_record: true,
                ..base_args()
            },
            false,
        ),
        (
            Args {
                cli: true,
                ..base_args()
            },
            true,
        ),
        (
            Args {
                list: true,
                ..base_args()
            },
            true,
        ),
        (
            Args {
                output: Some(PathBuf::from("out.png")),
                ..base_args()
            },
            true,
        ),
        (
            Args {
                screen: Some(0),
                ..base_args()
            },
            true,
        ),
        (
            Args {
                format: CliFormat::Bmp,
                ..base_args()
            },
            true,
        ),
        (
            Args {
                window: true,
                ..base_args()
            },
            true,
        ),
    ];

    for (args, expected) in cases {
        assert_eq!(should_run_cli_mode(&args), expected);
    }

    let args = base_args();
    assert!(should_run_cli_mode_with_format_explicit(&args, true));
    assert!(!should_run_cli_mode_with_format_explicit(&args, false));
}

#[test]
fn should_run_cli_mode_detects_attached_short_png_format() {
    let args = Args::try_parse_from(["rustshot", "-fpng"]).unwrap();
    let format_explicit = cli_format_flag_present_in(["rustshot", "-fpng"]);
    assert!(should_run_cli_mode_with_format_explicit(
        &args,
        format_explicit
    ));
}
