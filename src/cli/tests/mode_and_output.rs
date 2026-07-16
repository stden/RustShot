//! Tests for app-mode policy and output/error-reporting helpers.

#[allow(unused_imports)]
use super::*;
use crate::cli::*;
use std::io::{self, Write};

// =========================================================================
// should_run_cli_mode
// =========================================================================

#[test]
fn cli_mode_all_false() {
    assert!(!should_run_cli_mode(
        false, false, false, false, false, false
    ));
}

#[test]
fn cli_mode_cli_flag() {
    assert!(should_run_cli_mode(true, false, false, false, false, false));
}

#[test]
fn cli_mode_list_flag() {
    assert!(should_run_cli_mode(false, true, false, false, false, false));
}

#[test]
fn cli_mode_window_flag() {
    assert!(should_run_cli_mode(false, false, true, false, false, false));
}

#[test]
fn cli_mode_has_output() {
    assert!(should_run_cli_mode(false, false, false, true, false, false));
}

#[test]
fn cli_mode_has_screen() {
    assert!(should_run_cli_mode(false, false, false, false, true, false));
}

#[test]
fn cli_mode_format_explicit() {
    assert!(should_run_cli_mode(false, false, false, false, false, true));
}

#[test]
fn cli_mode_multiple_flags() {
    assert!(should_run_cli_mode(true, true, true, true, true, true));
}

// =========================================================================
// format_screen_list_output
// =========================================================================

#[test]
fn screen_list_empty() {
    let output = format_screen_list_output(Vec::<&str>::new());
    assert_eq!(output, "Available screens:");
}

#[test]
fn screen_list_single() {
    let output = format_screen_list_output(vec!["Screen 0: 1920x1080"]);
    assert_eq!(output, "Available screens:\n  Screen 0: 1920x1080");
}

#[test]
fn screen_list_multiple() {
    let output = format_screen_list_output(vec!["Screen 0: 1920x1080", "Screen 1: 2560x1440"]);
    assert!(output.starts_with("Available screens:"));
    assert!(output.contains("  Screen 0: 1920x1080"));
    assert!(output.contains("  Screen 1: 2560x1440"));
}

#[test]
fn screen_list_with_string_vec() {
    let screens = vec!["A".to_string(), "B".to_string()];
    let output = format_screen_list_output(screens);
    assert!(output.contains("  A"));
    assert!(output.contains("  B"));
}

// =========================================================================
// selection_stderr_message
// =========================================================================

#[test]
fn selection_stderr_cancelled_returns_none() {
    assert!(selection_stderr_message("Selection cancelled").is_none());
}

#[test]
fn selection_stderr_other_error_returns_some() {
    let msg = selection_stderr_message("Something went wrong");
    assert_eq!(msg, Some("Something went wrong".to_string()));
}

#[test]
fn selection_stderr_empty_error_returns_some() {
    assert_eq!(selection_stderr_message(""), Some(String::new()));
}

#[test]
fn selection_stderr_similar_to_cancelled_returns_some() {
    // Must be exact match, not prefix
    assert!(selection_stderr_message("Selection cancelled!").is_some());
}

// =========================================================================
// write_error_line
// =========================================================================

#[test]
fn write_error_line_appends_newline() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "hello").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "hello\n");
}

#[test]
fn write_error_line_empty_message() {
    let mut buf = Vec::new();
    write_error_line(&mut buf, "").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "\n");
}

#[test]
fn write_error_line_returns_error_on_broken_writer() {
    struct BrokenWriter;
    impl Write for BrokenWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut writer = BrokenWriter;
    assert!(writer.flush().is_ok());
    assert!(write_error_line(writer, "test").is_err());
}
