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
