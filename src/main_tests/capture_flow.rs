//! `run_cli_with_capture` tests: format/screen/output forwarding and error paths.

use super::*;

#[test]
fn run_cli_with_capture_uses_selected_output_and_format() {
    let output_path = PathBuf::from("custom.png");
    let args = Args {
        output: Some(output_path.clone()),
        format: CliFormat::Jpg,
        screen: Some(1),
        ..base_args()
    };
    let call_count = Cell::new(0);
    let path_for_assert = output_path.clone();

    let result = run_cli_with_capture_test(args, |actual_output, actual_format, actual_screen| {
        call_count.set(call_count.get() + 1);
        assert_eq!(actual_output, Some(path_for_assert.clone()));
        assert!(matches!(actual_format, capture::Format::Jpg));
        assert_eq!(actual_screen, Some(1));
        Ok(output_path.clone())
    })
    .unwrap();

    assert_eq!(
        result,
        format!("Screenshot saved to: {}", output_path.display())
    );
    assert_eq!(call_count.get(), 1);
}

#[test]
fn run_cli_with_capture_reports_capture_error() {
    let args = base_args();

    let result = run_cli_with_capture_test(args, |_, _, _| Err("capture failed".to_string()));
    assert_eq!(result, Err("capture failed".to_string()));
}

#[test]
fn run_cli_with_capture_screen_none_passes_none() {
    let args = Args {
        screen: None,
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |_output, _format, actual_screen| {
        called.set(true);
        assert_eq!(actual_screen, None);
        Ok(PathBuf::from("shot.png"))
    })
    .unwrap();
    assert!(called.get());
}

#[test]
fn run_cli_with_capture_output_none_passes_none() {
    let args = Args {
        cli: true,
        output: None,
        ..base_args()
    };
    let called = Cell::new(false);
    run_cli_with_capture_test(args, |output, _, _| {
        called.set(true);
        assert!(output.is_none());
        Ok(PathBuf::from("shot.png"))
    })
    .unwrap();
    assert!(called.get());
}
