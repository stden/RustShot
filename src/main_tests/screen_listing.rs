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

#[test]
fn format_screen_list_output_handles_single_entry() {
    let output = format_screen_list_output(vec!["Screen 0: 800x600".to_string()]);
    assert_eq!(output, "Available screens:\n  Screen 0: 800x600");
}

#[test]
fn run_cli_with_capture_list_does_not_call_capture_fn() {
    let args = Args {
        list: true,
        ..base_args()
    };
    // The capture fn should never be called when list=true
    let result = run_cli_with_capture_test(args, |_, _, _| {
        panic!("capture fn must not be called when listing screens")
    });
    assert!(result.is_ok());
}

#[test]
fn run_cli_with_capture_list_fn_call_count_is_zero() {
    let count = Cell::new(0u32);
    let args = Args {
        list: true,
        ..base_args()
    };
    let _ = run_cli_with_capture_test(args, |_, _, _| {
        count.set(count.get() + 1);
        Ok(PathBuf::from("x.png"))
    });
    assert_eq!(
        count.get(),
        0,
        "capture fn must not be called when list=true"
    );
}

#[test]
fn format_screen_list_output_three_entries() {
    let output = format_screen_list_output(vec![
        "Screen 0: 1920x1080".to_string(),
        "Screen 1: 2560x1440".to_string(),
        "Screen 2: 3840x2160".to_string(),
    ]);
    assert_eq!(output.lines().count(), 4); // header + 3 screens
    assert!(output.contains("Screen 2: 3840x2160"));
}

// --- format_screen_list_output edge cases ---

#[test]
fn format_screen_list_output_three_short_entries() {
    let output = format_screen_list_output(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
    assert_eq!(output, "Available screens:\n  A\n  B\n  C");
}

#[test]
fn format_screen_list_output_entries_are_indented() {
    let output = format_screen_list_output(vec!["Screen 0: 800x600".to_string()]);
    assert!(output.contains("  Screen 0"));
}

// -------------------------------------------------------------------------
// list_screens — smoke test
// -------------------------------------------------------------------------

#[test]
fn list_screens_returns_vec() {
    let screens = capture::list_screens();
    // On CI (no display) this may be empty, but should not panic
    assert!(screens.len() <= 100, "unreasonable number of screens");
}

#[test]
fn list_screens_entries_contain_screen_prefix() {
    let screens = capture::list_screens();
    for s in &screens {
        assert!(
            s.starts_with("Screen "),
            "each entry should start with 'Screen '"
        );
    }
}

// --- format_screen_list_output edge cases ---

#[test]
fn format_screen_list_output_special_characters() {
    let output = format_screen_list_output(vec!["Screen 0: 1920×1080 (Primary)".to_string()]);
    assert!(output.contains("1920×1080"));
    assert!(output.contains("(Primary)"));
}

#[test]
fn format_screen_list_output_many_entries() {
    let entries: Vec<String> = (0..10)
        .map(|i| format!("Screen {}: 1920x1080", i))
        .collect();
    let output = format_screen_list_output(entries);
    assert_eq!(output.lines().count(), 11); // header + 10 screens
}

#[test]
fn format_screen_list_output_preserves_entry_order() {
    let output = format_screen_list_output(vec![
        "first".to_string(),
        "second".to_string(),
        "third".to_string(),
    ]);
    let first_pos = output.find("first").unwrap();
    let second_pos = output.find("second").unwrap();
    let third_pos = output.find("third").unwrap();
    assert!(first_pos < second_pos);
    assert!(second_pos < third_pos);
}

// --- format_screen_list_output edge cases ---

#[test]
fn format_screen_list_output_single_entry() {
    let output = format_screen_list_output(vec!["Screen 0: 1920x1080".to_string()]);
    assert_eq!(output, "Available screens:\n  Screen 0: 1920x1080");
}

// -------------------------------------------------------------------------
// format_screen_list_output — large count
// -------------------------------------------------------------------------

#[test]
fn format_screen_list_output_ten_entries_all_indented() {
    let entries: Vec<String> = (0..10).map(|i| format!("Screen {i}: 1920x1080")).collect();
    let output = format_screen_list_output(entries);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 11); // header + 10 entries
    for line in &lines[1..] {
        assert!(line.starts_with("  "));
    }
}
