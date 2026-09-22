use super::*;
use std::cell::RefCell;

#[path = "tray_behavior.rs"]
mod behavior;

#[test]
fn exit_signals_recording_before_closing_selection() {
    let effects = RefCell::new(Vec::new());
    let path = Path::new("recording.stop");
    prepare_tray_exit_with(
        Some(path),
        |actual| {
            assert_eq!(actual, path);
            effects.borrow_mut().push("stop recording");
            Ok(())
        },
        || {
            effects.borrow_mut().push("close selection");
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(*effects.borrow(), ["stop recording", "close selection"]);
}

#[test]
fn exit_is_cancelled_when_recording_cannot_be_signalled() {
    let result = prepare_tray_exit_with(
        Some(Path::new("blocked.stop")),
        |_| Err("stop marker denied".to_string()),
        || panic!("shutdown must stop when the recorder cannot be signalled"),
    );
    assert_eq!(result, Err("stop marker denied".to_string()));
}

#[test]
fn exit_without_recording_still_closes_selection_and_reports_failure() {
    let result = prepare_tray_exit_with(
        None,
        |_| panic!("an absent recording must not be started or signalled"),
        || Err("selection could not close".to_string()),
    );
    assert_eq!(result, Err("selection could not close".to_string()));
}
