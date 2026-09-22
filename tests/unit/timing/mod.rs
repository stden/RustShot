use super::*;

/// Every milestone the timeline can report, for exhaustiveness in tests.
const ALL: [CaptureMilestone; 7] = [
    CaptureMilestone::ScreensEnumerated,
    CaptureMilestone::MonitorCaptured,
    CaptureMilestone::OverlayWindowCreated,
    CaptureMilestone::OverlayFirstFrame,
    CaptureMilestone::RegionSelected,
    CaptureMilestone::ImageCropped,
    CaptureMilestone::ClipboardWritten,
];

#[test]
fn every_milestone_has_a_distinct_label() {
    // Two milestones sharing a label would silently merge two stages in the
    // log, which is worse than having no timing at all.
    let mut labels: Vec<&str> = ALL.iter().map(|m| m.label()).collect();
    let count = labels.len();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(
        labels.len(),
        count,
        "duplicate milestone labels: {labels:?}"
    );
}

#[test]
fn milestone_labels_are_greppable() {
    // Lowercase, underscore-separated, no spaces: the log is filtered with
    // plain text tools.
    for milestone in ALL {
        let label = milestone.label();
        assert!(!label.is_empty());
        assert!(
            label.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{label} is not a greppable label"
        );
    }
}

#[test]
fn milestone_line_keeps_microsecond_resolution() {
    assert_eq!(
        format_milestone(
            CaptureMilestone::OverlayFirstFrame,
            Duration::from_micros(18_204)
        ),
        "[timing] overlay_first_frame +18.204 ms"
    );
    // A sub-millisecond stage must not render as +0.000 ms.
    assert_eq!(
        format_milestone(CaptureMilestone::ImageCropped, Duration::from_micros(87)),
        "[timing] image_cropped +0.087 ms"
    );
    assert_eq!(
        format_milestone(CaptureMilestone::ClipboardWritten, Duration::ZERO),
        "[timing] clipboard_written +0.000 ms"
    );
}

/// Both halves of `mark` live in one test on purpose: [`ORIGIN`] is a
/// process-global `OnceLock`, so a separate test that started the timeline
/// would race this one's "no timeline yet" precondition under `cargo test`'s
/// parallelism. Ordering them here makes the sequence deterministic.
#[test]
fn marking_is_silent_without_a_timeline_and_reports_offsets_with_one() {
    // The tray process never starts a timeline; marking must not panic or
    // implicitly start one.
    mark(CaptureMilestone::ScreensEnumerated);
    assert!(ORIGIN.get().is_none(), "marking must not start a timeline");

    // Every `--select` child starts one as its first act, and each milestone
    // afterwards reports an offset from it.
    start_capture_timeline();
    let origin = *ORIGIN.get().expect("the timeline is started");
    for milestone in ALL {
        mark(milestone);
    }

    // A second start is ignored rather than resetting the origin: two
    // origins in one process would make the offsets incomparable.
    start_capture_timeline();
    assert_eq!(
        *ORIGIN.get().expect("the timeline stays started"),
        origin,
        "the capture timeline origin must not be reset"
    );
}

#[test]
fn action_records_measure_each_step_and_keep_request_identity() {
    let origin = Instant::now();
    let timer = ActionTimer {
        action: "settings",
        process_id: 123,
        request_id: 45,
        origin,
        previous: std::cell::Cell::new(origin),
        finished: false,
    };

    assert_eq!(
        timer.record_at(ActionEvent::Started, origin),
        "[action] pid=123 request=45 action=settings stage=started step_ms=0.000 total_ms=0.000"
    );
    assert_eq!(
        timer.record_at(
            ActionEvent::Stage("created"),
            origin + Duration::from_micros(12_500)
        ),
        "[action] pid=123 request=45 action=settings stage=created step_ms=12.500 total_ms=12.500"
    );
    assert_eq!(
        timer.record_at(
            ActionEvent::Stage("visible"),
            origin + Duration::from_micros(20_750)
        ),
        "[action] pid=123 request=45 action=settings stage=visible step_ms=8.250 total_ms=20.750"
    );
    assert_eq!(
        timer.record_at(ActionEvent::Finished("ok"), origin + Duration::from_micros(21_000)),
        "[action] pid=123 request=45 action=settings stage=finished outcome=ok step_ms=0.250 total_ms=21.000"
    );
}

#[test]
fn action_timers_are_send_and_request_ids_are_distinct() {
    fn assert_send<T: Send>() {}
    assert_send::<ActionTimer>();

    let first = ActionTimer::new("first");
    let second = ActionTimer::new("second");
    assert_eq!(first.process_id, std::process::id());
    assert_eq!(first.process_id, second.process_id);
    assert_ne!(first.request_id, second.request_id);
    first.finish("ok");
    second.finish("error");
}

#[test]
fn time_result_preserves_success_error_and_calls_the_operation_once() {
    // Neither value is Clone or Display: instrumentation must not inspect or
    // reconstruct the application's payloads just to determine its status.
    struct Payload(u8);
    let value = Box::new(Payload(7));
    let expected = std::ptr::from_ref(value.as_ref());
    let mut calls = 0;
    let result = time_result("success", || {
        calls += 1;
        Ok::<_, Payload>(value)
    });
    let Ok(value) = result else {
        panic!("successful result changed")
    };
    assert_eq!(std::ptr::from_ref(value.as_ref()), expected);
    assert_eq!(value.0, 7);
    assert_eq!(calls, 1);

    let failure = Box::new(Payload(9));
    let expected = std::ptr::from_ref(failure.as_ref());
    let result = time_result("failure", || {
        calls += 1;
        Err::<Payload, _>(failure)
    });
    let Err(failure) = result else {
        panic!("error result changed")
    };
    assert_eq!(std::ptr::from_ref(failure.as_ref()), expected);
    assert_eq!(failure.0, 9);
    assert_eq!(calls, 2);
}

#[test]
fn capture_record_distinguishes_user_wait_from_its_total_elapsed_time() {
    assert_eq!(
        format_capture_record(
            CaptureMilestone::RegionSelected,
            123,
            45,
            Duration::from_millis(800),
            Duration::from_millis(825),
        ),
        "[timing] region_selected +825.000 ms pid=123 request=45 stage=user_selection_wait step_ms=800.000 total_ms=825.000"
    );
}

#[test]
fn only_unfinished_actions_produce_an_abandoned_record() {
    let origin = Instant::now();
    let mut timer = ActionTimer {
        action: "worker",
        process_id: 123,
        request_id: 45,
        origin,
        previous: std::cell::Cell::new(origin),
        finished: false,
    };
    assert_eq!(
        timer.abandonment_record_at(origin + Duration::from_millis(5)),
        Some("[action] pid=123 request=45 action=worker stage=abandoned outcome=abandoned step_ms=5.000 total_ms=5.000".to_string())
    );
    timer.finished = true;
    assert_eq!(
        timer.abandonment_record_at(origin + Duration::from_millis(6)),
        None
    );
}
