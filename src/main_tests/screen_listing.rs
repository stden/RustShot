//! Screen-list output formatting and `--list` CLI flow tests.

use super::*;

#[test]
fn run_cli_with_capture_lists_screens() {
    let args = Args {
        list: true,
        ..base_args()
    };

    let output =
        run_cli_with_capture_test(args, |_, _, _| unreachable!("capture fn should not run"))
            .unwrap();
    assert!(output.starts_with("Available screens:"));
}

#[test]
fn format_screen_list_output_formats_multiple_entries() {
    let output = format_screen_list_output(vec![
        "Screen 0: 1920x1080".to_string(),
        "Screen 1: 2560x1440".to_string(),
    ]);

    assert_eq!(
        output,
        "Available screens:\n  Screen 0: 1920x1080\n  Screen 1: 2560x1440"
    );
}

#[test]
fn format_screen_list_output_handles_empty_list() {
    assert_eq!(
        format_screen_list_output(Vec::<String>::new()),
        "Available screens:"
    );
}

// --- format_screen_list_output edge cases ---

#[test]
fn format_screen_list_output_special_characters() {
    let output = format_screen_list_output(vec!["Screen 0: 1920×1080 (Primary)".to_string()]);
    assert!(output.contains("1920×1080"));
    assert!(output.contains("(Primary)"));
}
