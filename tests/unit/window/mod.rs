use super::*;

#[test]
fn listable_requires_visible_and_titled() {
    assert!(is_listable_window(true, "Editor"));
    assert!(!is_listable_window(false, "Editor"));
    assert!(!is_listable_window(true, ""));
    assert!(!is_listable_window(true, "\t \n"));
}

#[test]
fn format_window_list_empty() {
    assert_eq!(format_window_list(&[]), "No open windows found.");
}

#[test]
fn format_window_list_lines() {
    let windows = vec![
        WindowInfo {
            id: 10,
            title: "A".to_string(),
        },
        WindowInfo {
            id: 20,
            title: "B".to_string(),
        },
    ];
    assert_eq!(
        format_window_list(&windows),
        "Open windows:\n  10  A\n  20  B"
    );
}

#[cfg(not(windows))]
#[test]
fn non_windows_returns_empty_and_errors() {
    assert!(list_windows().is_empty());
    assert!(capture_window_image(1).is_err());
}

/// A `--window-id` from a stale window list must be rejected by name before
/// any capture backend is started, rather than surfacing as a WGC or GDI
/// failure further down.
#[cfg(windows)]
#[test]
fn unknown_window_id_is_rejected_before_capture() {
    assert_eq!(
        capture_window_image(u64::MAX),
        Err(format!("No window with id {}", u64::MAX))
    );
}

/// The default-directory wrapper must reject the same ids as
/// [`capture_window_in_dir`], and reject them before it creates or writes
/// anything under the screenshots directory.
#[cfg(windows)]
#[test]
fn capture_window_with_the_default_directory_rejects_unknown_ids() {
    assert_eq!(
        capture_window(u64::MAX, None, Format::Png),
        Err(format!("No window with id {}", u64::MAX))
    );
}
