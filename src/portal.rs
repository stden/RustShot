//! Wayland screenshots through the `org.freedesktop.portal.Screenshot` D-Bus API.
//!
//! Wayland compositors do not let applications grab the screen directly, so a
//! Wayland session asks the desktop portal (`xdg-desktop-portal` plus a desktop
//! backend) to take the screenshot. The portal writes the image to a file and
//! answers with that file's `file://` URI. X11 sessions keep using the
//! `screenshots` crate path in `capture.rs`.
//!
//! The portal can only capture the whole desktop. Window and single-screen
//! requests are rejected with errors instead of being approximated.

use dbus::arg::{PropMap, RefArg, Variant};
use dbus::blocking::Connection;
use dbus::message::MatchRule;
use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Which capture path a session uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureBackend {
    /// Existing `screenshots` crate path, used for X11 sessions.
    X11,
    /// Desktop screenshot portal, used for Wayland sessions.
    WaylandPortal,
}

/// Error returned for `--window` (and the matching MCP call) on Wayland.
pub const WINDOW_CAPTURE_UNSUPPORTED: &str = "Active window capture is not available on Wayland: the desktop screenshot portal only captures the whole desktop. Run without --window, or use an X11 session.";

const PORTAL_SERVICE: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SCREENSHOT_INTERFACE: &str = "org.freedesktop.portal.Screenshot";
const REQUEST_INTERFACE: &str = "org.freedesktop.portal.Request";
/// Fixed token: every bus connection has its own sender name, so request paths never clash.
const HANDLE_TOKEN: &str = "rustshot";
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
/// How long to wait for the portal (and any compositor prompt) before giving up.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Response codes of the `org.freedesktop.portal.Request.Response` signal.
const RESPONSE_SUCCESS: u32 = 0;
const RESPONSE_CANCELLED: u32 = 1;
const RESPONSE_FAILED: u32 = 2;

const CANCELLED_MESSAGE: &str = "Screenshot cancelled";
const PORTAL_MISSING_MESSAGE: &str = "The desktop portal is not running. Install xdg-desktop-portal with a desktop backend (xdg-desktop-portal-gnome, -kde or -wlr) and make sure it runs in your session.";
const PORTAL_METHOD_MISSING_MESSAGE: &str = "The desktop portal does not provide the Screenshot interface. Update xdg-desktop-portal and its desktop backend.";

/// Choose the capture path from the session environment values.
///
/// An explicit `XDG_SESSION_TYPE` decides. Otherwise a non-empty
/// `WAYLAND_DISPLAY` means Wayland.
pub fn backend_for_session(
    session_type: Option<&str>,
    wayland_display: Option<&str>,
) -> CaptureBackend {
    let is_wayland = match session_type {
        Some("wayland") => true,
        Some("x11") => false,
        _ => wayland_display.is_some_and(|display| !display.is_empty()),
    };
    if is_wayland {
        CaptureBackend::WaylandPortal
    } else {
        CaptureBackend::X11
    }
}

/// Capture path for this process, read from the session environment.
pub fn current_backend() -> CaptureBackend {
    let session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    backend_for_session(session_type.as_deref(), wayland_display.as_deref())
}

/// Reject a single-screen request, which the whole-desktop portal cannot honour.
pub fn ensure_full_desktop(screen_index: Option<usize>) -> Result<(), String> {
    match screen_index {
        None => Ok(()),
        Some(index) => Err(format!(
            "Capturing one screen (--screen {index}) is not available on Wayland: the desktop screenshot portal only captures the whole desktop. Run without --screen, or use an X11 session."
        )),
    }
}

/// Map the `Request.Response` code and optional `uri` to the screenshot file path.
fn parse_screenshot_response(response: u32, uri: Option<&str>) -> Result<PathBuf, String> {
    match response {
        RESPONSE_SUCCESS => {
            let uri =
                uri.ok_or("The desktop portal reported success but returned no screenshot")?;
            file_uri_to_path(uri)
        }
        RESPONSE_CANCELLED => Err(CANCELLED_MESSAGE.to_string()),
        RESPONSE_FAILED => {
            Err("The desktop portal failed or denied the screenshot request".to_string())
        }
        other => Err(format!(
            "The desktop portal returned an unexpected response code {other}"
        )),
    }
}

/// Convert a `file://` URI from the portal into a local path, decoding `%XX` escapes.
fn file_uri_to_path(uri: &str) -> Result<PathBuf, String> {
    let encoded = uri
        .strip_prefix("file://")
        .filter(|path| path.starts_with('/'))
        .ok_or_else(|| {
            format!("The desktop portal returned a non-local screenshot location: {uri}")
        })?;
    let decoded = percent_decode_str(encoded)
        .decode_utf8()
        .map_err(|_| format!("The screenshot location is not valid UTF-8: {uri}"))?;
    Ok(PathBuf::from(decoded.as_ref()))
}

/// Turn a failed D-Bus call into a message that tells the user what to fix.
fn portal_call_error(name: Option<&str>, message: Option<&str>) -> String {
    match name {
        Some("org.freedesktop.portal.Error.NotAllowed") => {
            "Screenshot permission was denied by the desktop portal".to_string()
        }
        Some("org.freedesktop.portal.Error.Cancelled") => CANCELLED_MESSAGE.to_string(),
        Some(
            "org.freedesktop.DBus.Error.ServiceUnknown"
            | "org.freedesktop.DBus.Error.NameHasNoOwner",
        ) => PORTAL_MISSING_MESSAGE.to_string(),
        Some(
            "org.freedesktop.DBus.Error.UnknownMethod"
            | "org.freedesktop.DBus.Error.UnknownInterface"
            | "org.freedesktop.DBus.Error.UnknownObject",
        ) => PORTAL_METHOD_MISSING_MESSAGE.to_string(),
        _ => format!(
            "Screenshot request to the desktop portal failed: {}",
            message.or(name).unwrap_or("unknown error")
        ),
    }
}

/// Object path the portal uses for a request: `.../request/<sender>/<token>`.
///
/// The sender is the bus name without its leading `:`, with dots replaced by
/// underscores, as the portal specification defines it.
fn request_handle_path(unique_name: &str, token: &str) -> String {
    let sender = unique_name.trim_start_matches(':').replace('.', "_");
    format!("{PORTAL_PATH}/request/{sender}/{token}")
}

/// Ask the desktop portal for a full-desktop screenshot and return the file it wrote.
///
/// The response listener is registered before the call, so an answer that
/// arrives early is not lost. Cancellation, denial and missing portals come
/// back as `Err` with a readable message.
pub fn request_screenshot() -> Result<PathBuf, String> {
    let connection = Connection::new_session()
        .map_err(|e| format!("Cannot reach the desktop session bus: {e}"))?;
    let handle = request_handle_path(&connection.unique_name(), HANDLE_TOKEN);
    let slot: AnswerSlot = Arc::new(Mutex::new(None));
    listen_for_response(&connection, &handle, Arc::clone(&slot))?;
    call_screenshot(&connection, &handle)?;
    let (code, uri) = wait_for_response(&connection, &slot)?;
    parse_screenshot_response(code, uri.as_deref())
}

/// Response code and the optional `uri` taken from the `Response` signal.
type Answer = (u32, Option<String>);
/// Shared with the D-Bus callback, which must be `Send`; hence `Arc` and plain data.
type AnswerSlot = Arc<Mutex<Option<Answer>>>;

fn listen_for_response(
    connection: &Connection,
    handle: &str,
    slot: AnswerSlot,
) -> Result<(), String> {
    // Only the portal may answer: a same-user process could otherwise forge a Response on our path.
    let rule = MatchRule::new_signal(REQUEST_INTERFACE, "Response")
        .with_sender(PORTAL_SERVICE)
        .with_path(handle.to_string());
    connection
        .add_match(rule, move |(code, results): (u32, PropMap), _, _| {
            let uri = results
                .get("uri")
                .and_then(|value| value.as_str())
                .map(str::to_owned);
            if let Ok(mut answer) = slot.lock() {
                *answer = Some((code, uri));
            }
            // A request answers exactly once, so the match can be dropped.
            false
        })
        .map(|_| ())
        .map_err(|e| format!("Cannot listen for the screenshot response: {e}"))
}

fn validate_request_handle(expected: &str, actual: &str) -> Result<(), String> {
    if expected == actual {
        return Ok(());
    }
    Err("The desktop portal returned an unexpected request handle".to_string())
}

fn call_screenshot(connection: &Connection, expected_handle: &str) -> Result<(), String> {
    let mut options: PropMap = HashMap::new();
    options.insert(
        "handle_token".to_string(),
        Variant(Box::new(HANDLE_TOKEN.to_string())),
    );
    options.insert("interactive".to_string(), Variant(Box::new(false)));
    let proxy = connection.with_proxy(PORTAL_SERVICE, PORTAL_PATH, CALL_TIMEOUT);
    let reply: Result<(dbus::Path<'static>,), dbus::Error> =
        proxy.method_call(SCREENSHOT_INTERFACE, "Screenshot", ("", options));
    let (handle,) = reply.map_err(|e| portal_call_error(e.name(), e.message()))?;
    validate_request_handle(expected_handle, &handle)
}

fn wait_for_response(connection: &Connection, slot: &AnswerSlot) -> Result<Answer, String> {
    let deadline = Instant::now() + RESPONSE_TIMEOUT;
    loop {
        let answer = slot
            .lock()
            .map_err(|_| "Internal error: the screenshot response state is poisoned".to_string())?
            .take();
        if let Some(answer) = answer {
            return Ok(answer);
        }
        if Instant::now() >= deadline {
            return Err(
                "Timed out waiting for the desktop portal to answer the screenshot request"
                    .to_string(),
            );
        }
        connection
            .process(POLL_INTERVAL)
            .map_err(|e| format!("Lost the connection to the desktop session bus: {e}"))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_wayland_session_type_selects_portal() {
        assert_eq!(
            backend_for_session(Some("wayland"), None),
            CaptureBackend::WaylandPortal
        );
    }

    #[test]
    fn backend_x11_session_type_stays_on_x11_even_with_wayland_display() {
        assert_eq!(
            backend_for_session(Some("x11"), Some("wayland-0")),
            CaptureBackend::X11
        );
    }

    #[test]
    fn backend_missing_session_type_uses_wayland_display() {
        assert_eq!(
            backend_for_session(None, Some("wayland-0")),
            CaptureBackend::WaylandPortal
        );
    }

    #[test]
    fn backend_other_session_type_uses_wayland_display() {
        assert_eq!(
            backend_for_session(Some("tty"), Some("wayland-0")),
            CaptureBackend::WaylandPortal
        );
    }

    #[test]
    fn backend_without_any_wayland_signal_is_x11() {
        assert_eq!(backend_for_session(None, None), CaptureBackend::X11);
        assert_eq!(backend_for_session(Some("x11"), None), CaptureBackend::X11);
    }

    #[test]
    fn backend_empty_wayland_display_is_x11() {
        assert_eq!(backend_for_session(None, Some("")), CaptureBackend::X11);
    }

    #[test]
    fn full_desktop_request_without_screen_is_accepted() {
        assert!(ensure_full_desktop(None).is_ok());
    }

    #[test]
    fn full_desktop_request_with_screen_index_is_rejected_with_clear_message() {
        let err = ensure_full_desktop(Some(1)).unwrap_err();
        assert!(err.contains("--screen 1"), "message was: {err}");
        assert!(err.contains("Wayland"), "message was: {err}");
    }

    #[test]
    fn response_success_returns_local_path_from_uri() {
        let path = parse_screenshot_response(0, Some("file:///tmp/shot.png")).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/shot.png"));
    }

    #[test]
    fn response_success_without_uri_is_error() {
        let err = parse_screenshot_response(0, None).unwrap_err();
        assert!(err.contains("no screenshot"), "message was: {err}");
    }

    #[test]
    fn response_cancelled_is_error_without_path() {
        let err = parse_screenshot_response(1, None).unwrap_err();
        assert_eq!(err, CANCELLED_MESSAGE);
    }

    #[test]
    fn response_failed_or_denied_is_error() {
        let err = parse_screenshot_response(2, None).unwrap_err();
        assert!(err.contains("denied"), "message was: {err}");
    }

    #[test]
    fn response_unknown_code_is_error_naming_the_code() {
        let err = parse_screenshot_response(9, None).unwrap_err();
        assert!(err.contains("code 9"), "message was: {err}");
    }

    #[test]
    fn file_uri_decodes_percent_escapes() {
        let path =
            file_uri_to_path("file:///home/user/Pictures/Screenshot%20from%202026.png").unwrap();
        assert_eq!(
            path,
            PathBuf::from("/home/user/Pictures/Screenshot from 2026.png")
        );
    }

    #[test]
    fn file_uri_rejects_remote_location() {
        let err = file_uri_to_path("https://example.com/shot.png").unwrap_err();
        assert!(err.contains("non-local"), "message was: {err}");
    }

    #[test]
    fn file_uri_rejects_host_part() {
        let err = file_uri_to_path("file://server/share/shot.png").unwrap_err();
        assert!(err.contains("non-local"), "message was: {err}");
    }

    #[test]
    fn file_uri_rejects_non_utf8_path() {
        let err = file_uri_to_path("file:///tmp/%FF.png").unwrap_err();
        assert!(err.contains("not valid UTF-8"), "message was: {err}");
    }

    #[test]
    fn portal_error_not_allowed_reports_denied_permission() {
        let msg = portal_call_error(Some("org.freedesktop.portal.Error.NotAllowed"), None);
        assert!(msg.contains("permission was denied"), "message was: {msg}");
    }

    #[test]
    fn portal_error_cancelled_uses_cancellation_message() {
        let msg = portal_call_error(Some("org.freedesktop.portal.Error.Cancelled"), None);
        assert_eq!(msg, CANCELLED_MESSAGE);
    }

    #[test]
    fn portal_error_service_unknown_names_missing_portal() {
        let msg = portal_call_error(Some("org.freedesktop.DBus.Error.ServiceUnknown"), None);
        assert_eq!(msg, PORTAL_MISSING_MESSAGE);
    }

    #[test]
    fn portal_error_unknown_method_names_missing_interface() {
        let msg = portal_call_error(Some("org.freedesktop.DBus.Error.UnknownMethod"), None);
        assert_eq!(msg, PORTAL_METHOD_MISSING_MESSAGE);
    }

    #[test]
    fn portal_error_other_keeps_bus_message() {
        let msg = portal_call_error(Some("org.example.Failure"), Some("disk on fire"));
        assert!(msg.contains("disk on fire"), "message was: {msg}");
    }

    #[test]
    fn portal_error_without_name_or_message_is_still_readable() {
        let msg = portal_call_error(None, None);
        assert!(msg.contains("unknown error"), "message was: {msg}");
    }

    #[test]
    fn request_handle_path_strips_colon_and_replaces_dots() {
        assert_eq!(
            request_handle_path(":1.57", "rustshot"),
            "/org/freedesktop/portal/desktop/request/1_57/rustshot"
        );
    }

    #[test]
    fn request_handle_accepts_the_registered_response_path() {
        let expected = "/org/freedesktop/portal/desktop/request/1_57/rustshot";
        assert_eq!(validate_request_handle(expected, expected), Ok(()));
    }

    #[test]
    fn request_handle_rejects_an_unregistered_response_path() {
        let error = validate_request_handle("/expected", "/unexpected").unwrap_err();
        assert!(error.contains("unexpected request handle"));
    }
}
