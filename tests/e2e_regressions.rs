//! End-to-end regression coverage for validated public API boundaries.

use rustshot::{record, settings};

#[cfg(windows)]
#[path = "support/capture_window.rs"]
mod capture_fixture;

// ---------------------------------------------------------------------------
// CLI process helpers
//
// The tests below run the real executable rather than calling into the library,
// because the behaviour they pin *is* the process contract: which mode a flag
// selects, what lands on stdout versus stderr, and the exit code a script sees.
// None of that is observable from inside the library.
// ---------------------------------------------------------------------------

/// Output of one CLI invocation, decoded as text.
struct CliRun {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_cli(args: &[&str]) -> CliRun {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_RustShot"))
        .args(args)
        .output()
        .expect("the RustShot binary must run");
    CliRun {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// A unique temp directory for one test, removed on drop.
struct TempOut(std::path::PathBuf);

impl TempOut {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is set after the UNIX epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rustshot_e2e_{name}_{nanos}"));
        std::fs::create_dir_all(&path).expect("test output directory");
        Self(path)
    }

    fn file(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempOut {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cli_list_screens_prints_the_screen_table_and_exits_successfully() {
    let run = run_cli(&["--list"]);

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(
        run.stdout.starts_with("Available screens:"),
        "stdout: {:?}",
        run.stdout
    );
    // `--list` is one of the flags that implies CLI mode, so it must never fall
    // through to the tray and leave the process running.
    assert!(run.stdout.contains("Screen 0:"), "stdout: {:?}", run.stdout);
}

#[test]
fn cli_list_windows_prints_the_window_table_and_exits_successfully() {
    let run = run_cli(&["--list-windows"]);

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(
        run.stdout.starts_with("Open windows:") || run.stdout.starts_with("No open windows found."),
        "stdout: {:?}",
        run.stdout
    );
}

#[test]
fn cli_rejects_an_unsupported_output_extension_before_capturing() {
    // The filename must not be able to disagree with the encoder: without an
    // explicit --format, an unknown extension is refused rather than silently
    // written as a PNG called ".tiff".
    let run = run_cli(&["--cli", "-o", "screenshot.tiff"]);

    assert_eq!(run.code, Some(1), "stdout: {}", run.stdout);
    assert!(
        run.stderr.contains("Unsupported output extension '.tiff'"),
        "stderr: {:?}",
        run.stderr
    );
    assert!(
        run.stderr.contains("--format"),
        "the error must name the escape hatch: {:?}",
        run.stderr
    );
}

#[test]
fn cli_reports_a_platform_specific_window_error_and_exits_nonzero() {
    let run = run_cli(&["--cli", "--window-id", "18446744073709551615"]);

    assert_eq!(run.code, Some(1), "stdout: {}", run.stdout);
    let expected_error = if cfg!(target_os = "windows") {
        "No window with id 18446744073709551615"
    } else {
        "Window capture is only supported on Windows"
    };
    assert!(
        run.stderr.contains(expected_error),
        "expected {expected_error:?} in stderr: {:?}",
        run.stderr
    );
}

#[test]
fn cli_capture_writes_a_decodable_image_to_the_requested_path() {
    let out = TempOut::new("cli_capture");
    let target = out.file("shot.png");
    let run = run_cli(&["--cli", "-o", target.to_str().expect("utf-8 temp path")]);

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(
        run.stdout.contains("Screenshot saved to:"),
        "stdout: {:?}",
        run.stdout
    );
    let image = image::open(&target).expect("the saved screenshot must decode");
    assert!(image.width() > 0 && image.height() > 0);
}

#[cfg(windows)]
#[test]
fn cli_active_window_capture_writes_a_decodable_image() {
    // `--window` routes to a different backend than a plain screen capture, and
    // the CLI wires it through its own closure. Whichever window happens to be
    // in the foreground while the suite runs is a valid subject; the contract is
    // the saved, decodable file and the success message.
    let out = TempOut::new("cli_active_window");
    let target = out.file("active.png");
    let run = run_cli(&[
        "--cli",
        "--window",
        "-o",
        target.to_str().expect("utf-8 temp path"),
    ]);

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(
        run.stdout.contains("Screenshot saved to:"),
        "stdout: {:?}",
        run.stdout
    );
    let image = image::open(&target).expect("the saved window screenshot must decode");
    assert!(image.width() > 0 && image.height() > 0);
}

#[cfg(windows)]
#[test]
fn cli_captures_a_listed_window_to_the_requested_path() {
    // Chains the two documented steps: --list-windows hands out ids, and
    // --window-id consumes one. A drift between the two formats would break the
    // only workflow a user has for capturing a specific window.
    let listed = run_cli(&["--list-windows"]);
    assert_eq!(listed.code, Some(0), "stderr: {}", listed.stderr);

    let Some(id) = listed
        .stdout
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().next())
        .find_map(|token| token.parse::<u64>().ok())
    else {
        eprintln!("skipping: no listed windows to capture on this desktop");
        return;
    };

    let out = TempOut::new("cli_window");
    let target = out.file("window.png");
    let run = run_cli(&[
        "--cli",
        "--window-id",
        &id.to_string(),
        "-o",
        target.to_str().expect("utf-8 temp path"),
    ]);

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    let image = image::open(&target).expect("the saved window screenshot must decode");
    assert!(image.width() > 0 && image.height() > 0);
}

#[cfg(windows)]
#[test]
fn oversized_recording_region_is_rejected_before_a_worker_starts() {
    let region = rustshot::selection::Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: u32::MAX,
        physical_height: 2,
    };
    let result = record::RecordingSession::start_in_dir(
        0,
        0,
        region,
        settings::Fps::new(30).unwrap(),
        std::env::temp_dir().join("rustshot-e2e-invalid-recording"),
    );

    assert!(result.is_err());
}

#[test]
fn malformed_recording_buffers_return_errors_without_panicking() {
    let result = std::panic::catch_unwind(|| {
        let mut output = [0u8; 4];
        record::rgba_to_bgra_bottom_up_into(&[], &mut output, 1, 1)
    });

    assert!(result.is_ok(), "public conversion API must not panic");
    assert!(result.unwrap().is_err());
}

/// The `--mcp` stdio transport, driven through a real process.
///
/// `handle_message` is unit-tested, but nothing covered the loop around it:
/// newline framing, skipping blank lines, staying silent for notifications, and
/// terminating at EOF instead of hanging. It also pins something no in-process
/// test can see — on Windows the binary is built with
/// `windows_subsystem = "windows"` and so owns no console, yet it must still
/// serve JSON-RPC when a client hands it pipes.
#[test]
fn mcp_stdio_transport_answers_one_line_per_request_and_exits_at_eof() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_RustShot"))
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the MCP server must start");

    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        // A blank line is skipped; a notification is acted on but never
        // answered; only the two remaining messages may produce output.
        stdin.write_all(b"\n").expect("write blank line");
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .expect("write notification");
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n")
            .expect("write request");
        stdin
            .write_all(b"{ not json\n")
            .expect("write malformed line");
        // Dropping stdin closes the pipe: EOF must end the serve loop.
    }

    let output = child
        .wait_with_output()
        .expect("the server must exit once stdin reaches EOF");
    assert!(output.status.success(), "exit status: {:?}", output.status);

    let stdout = String::from_utf8(output.stdout).expect("responses must be UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "unexpected transcript: {stdout:?}");

    let initialize: serde_json::Value =
        serde_json::from_str(lines[0]).expect("the first response must be one JSON object");
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["protocolVersion"], "2024-11-05");

    // A malformed line must be reported and survived, not terminate the server.
    let parse_error: serde_json::Value =
        serde_json::from_str(lines[1]).expect("the second response must be one JSON object");
    assert!(parse_error["id"].is_null());
    assert_eq!(parse_error["error"]["code"], -32700);
}

/// The two capture tools, driven over the real transport.
///
/// Everything else about `tools/call` is unit-tested with stubs, but the branch
/// that runs after a capture actually succeeds — read the file back and hand the
/// client the bytes inline — needs a real capture. An MCP client renders that
/// image block, so "the file was saved" is only half the contract.
#[cfg(windows)]
#[test]
fn mcp_capture_tools_return_the_saved_path_and_the_image_inline() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let out = TempOut::new("mcp_capture");
    let screen_target = out.file("screen.png");
    let window_target = out.file("window.jpg");

    // A window belonging to another application can close between enumeration
    // and capture. Keep our target alive and changing through both requests.
    let fixture = capture_fixture::CaptureWindow::new();

    let listed = run_cli(&["--list-windows"]);
    let window_id = Some(fixture.id());
    assert!(
        listed
            .stdout
            .lines()
            .skip(1)
            .filter_map(|line| line.split_whitespace().next())
            .filter_map(|token| token.parse::<u64>().ok())
            .any(|id| id == fixture.id()),
        "the owned window must be listed"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_RustShot"))
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the MCP server must start");

    let mut expected_lines = 1;
    {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let screen_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "capture_screen",
                "arguments": { "screen": 0, "output_path": screen_target },
            },
        });
        writeln!(stdin, "{screen_request}").expect("write capture_screen request");

        if let Some(id) = window_id {
            let window_request = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "capture_window",
                    "arguments": { "id": id, "format": "jpg", "output_path": window_target },
                },
            });
            writeln!(stdin, "{window_request}").expect("write capture_window request");
            expected_lines = 2;
        }
    }

    let output = child
        .wait_with_output()
        .expect("the server must exit at EOF");
    assert!(output.status.success(), "exit status: {:?}", output.status);
    let stdout = String::from_utf8(output.stdout).expect("responses must be UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), expected_lines, "transcript: {stdout:?}");

    let screen: serde_json::Value = serde_json::from_str(lines[0]).expect("response 1 is JSON");
    assert_eq!(
        screen["result"]["isError"], false,
        "capture_screen failed: {}",
        screen["result"]["content"][0]["text"]
    );
    assert_eq!(screen["result"]["content"][0]["type"], "text");
    assert!(screen["result"]["content"][0]["text"]
        .as_str()
        .expect("a text block")
        .starts_with("Saved to "));
    assert_eq!(screen["result"]["content"][1]["type"], "image");
    assert_eq!(screen["result"]["content"][1]["mimeType"], "image/png");
    assert!(
        screen["result"]["content"][1]["data"]
            .as_str()
            .expect("base64 data")
            .len()
            > 100,
        "the inline image must carry the encoded screenshot"
    );
    assert!(image::open(&screen_target).is_ok(), "saved PNG must decode");

    if window_id.is_some() {
        let window: serde_json::Value = serde_json::from_str(lines[1]).expect("response 2 is JSON");
        assert_eq!(
            window["result"]["isError"], false,
            "capture_window failed: {}",
            window["result"]["content"][0]["text"]
        );
        // The requested format reaches both the encoder and the MIME type.
        assert_eq!(window["result"]["content"][1]["mimeType"], "image/jpeg");
        assert!(
            image::open(&window_target).is_ok(),
            "saved JPEG must decode"
        );
    }
}

#[test]
fn settings_with_control_characters_are_valid_json_and_roundtrip() {
    let original = settings::Settings {
        screenshots_dir: "shots\0archive".into(),
        recordings_dir: "records\u{1}archive".into(),
        ..settings::Settings::default()
    };

    let encoded = settings::format_settings_json(&original);
    serde_json::from_str::<serde_json::Value>(&encoded).expect("settings must be valid JSON");
    let decoded = settings::parse_settings_json(&encoded);

    assert_eq!(decoded.screenshots_dir, original.screenshots_dir);
    assert_eq!(decoded.recordings_dir, original.recordings_dir);
}

#[cfg(unix)]
#[test]
fn non_unicode_settings_paths_roundtrip_losslessly() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let mut original = settings::Settings::default();
    original.screenshots_dir = OsString::from_vec(vec![b's', b'h', b'o', b't', 0xff]).into();

    let encoded = settings::format_settings_json(&original);
    let decoded = settings::parse_settings_json(&encoded);
    assert_eq!(decoded.screenshots_dir, original.screenshots_dir);
}
