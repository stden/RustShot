//! `CliFormat` conversion tests.

use super::*;

#[test]
fn cli_format_to_capture_format_all_variants() {
    let conversions: Vec<(CliFormat, &str)> = vec![
        (CliFormat::Png, "png"),
        (CliFormat::Jpg, "jpg"),
        (CliFormat::Bmp, "bmp"),
    ];
    for (cli_fmt, expected_ext) in conversions {
        let capture_fmt: capture::Format = cli_fmt.into();
        assert_eq!(capture_fmt.extension(), expected_ext);
    }
}
