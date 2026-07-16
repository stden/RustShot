//! Unit tests for the MCP server: protocol routing via [`handle_message`] and
//! the pure argument/content helpers.

use super::*;

fn parse(response: &str) -> Value {
    serde_json::from_str(response).expect("response must be valid JSON")
}

#[test]
fn base64_matches_known_vectors() {
    // RFC 4648 test vectors.
    assert_eq!(base64_encode(b""), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
}

#[test]
fn initialize_reports_protocol_and_identity() {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let res = parse(&handle_message(req).unwrap());
    assert_eq!(res["id"], 1);
    assert_eq!(res["result"]["protocolVersion"], PROTOCOL_VERSION);
    assert_eq!(res["result"]["serverInfo"]["name"], SERVER_NAME);
    assert!(res["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn tools_list_advertises_all_four_tools() {
    let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    let res = parse(&handle_message(req).unwrap());
    let names: Vec<&str> = res["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "list_screens",
            "list_windows",
            "capture_screen",
            "capture_window"
        ]
    );
    // capture_window must require an id.
    let capture_window = res["result"]["tools"].as_array().unwrap()[3].clone();
    assert_eq!(capture_window["inputSchema"]["required"][0], "id");
}

#[test]
fn notification_gets_no_response() {
    let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    assert!(handle_message(notif).is_none());
}

#[test]
fn unknown_method_is_a_jsonrpc_error() {
    let req = r#"{"jsonrpc":"2.0","id":9,"method":"does/not/exist"}"#;
    let res = parse(&handle_message(req).unwrap());
    assert_eq!(res["id"], 9);
    assert_eq!(res["error"]["code"], METHOD_NOT_FOUND);
}

#[test]
fn malformed_json_is_a_parse_error_not_a_crash() {
    let res = parse(&handle_message("{ this is not json").unwrap());
    assert!(res["id"].is_null());
    assert_eq!(res["error"]["code"], PARSE_ERROR);
}

#[test]
fn list_windows_tool_returns_text_content() {
    let req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_windows","arguments":{}}}"#;
    let res = parse(&handle_message(req).unwrap());
    assert_eq!(res["result"]["isError"], false);
    assert_eq!(res["result"]["content"][0]["type"], "text");
    // Populated on a desktop, empty on headless CI — both are non-crashing.
    assert!(res["result"]["content"][0]["text"].is_string());
}

#[test]
fn list_screens_tool_returns_text_content() {
    let req = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_screens","arguments":{}}}"#;
    let res = parse(&handle_message(req).unwrap());
    assert_eq!(res["result"]["isError"], false);
    assert_eq!(res["result"]["content"][0]["type"], "text");
}

#[test]
fn unknown_tool_reports_iserror_not_transport_error() {
    let req =
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#;
    let res = parse(&handle_message(req).unwrap());
    // Tool errors are a normal result with isError, not a JSON-RPC error.
    assert!(res["error"].is_null());
    assert_eq!(res["result"]["isError"], true);
    assert!(res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Unknown tool"));
}

#[test]
fn capture_window_without_id_is_a_tool_error() {
    let req = r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"capture_window","arguments":{}}}"#;
    let res = parse(&handle_message(req).unwrap());
    assert_eq!(res["result"]["isError"], true);
    assert!(res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("id"));
}

#[test]
fn string_id_is_preserved_in_response() {
    let req = r#"{"jsonrpc":"2.0","id":"abc","method":"ping"}"#;
    let res = parse(&handle_message(req).unwrap());
    assert_eq!(res["id"], "abc");
}

// ---------------------------------------------------------------------------
// format_arg — tool-argument format parsing with the forgiving PNG fallback
// ---------------------------------------------------------------------------

#[test]
fn format_arg_parses_each_supported_format() {
    for format in Format::ALL {
        let args = json!({ "format": format.extension() });
        assert_eq!(format_arg(&args), format);
    }
}

#[test]
fn format_arg_falls_back_to_png_for_unknown_format() {
    // Same forgiving rule as settings: unrecognised input means PNG.
    assert_eq!(format_arg(&json!({ "format": "webp" })), Format::Png);
}

#[test]
fn format_arg_is_case_sensitive_like_settings() {
    assert_eq!(format_arg(&json!({ "format": "JPG" })), Format::Png);
}

#[test]
fn format_arg_defaults_to_png_when_missing() {
    assert_eq!(format_arg(&json!({})), Format::Png);
}

#[test]
fn format_arg_defaults_to_png_for_non_string_value() {
    assert_eq!(format_arg(&json!({ "format": 42 })), Format::Png);
}

// ---------------------------------------------------------------------------
// output_path_arg — optional explicit output path
// ---------------------------------------------------------------------------

#[test]
fn output_path_arg_reads_the_provided_path() {
    let args = json!({ "output_path": "shots/out.png" });
    assert_eq!(
        output_path_arg(&args),
        Some(std::path::PathBuf::from("shots/out.png"))
    );
}

#[test]
fn output_path_arg_is_none_when_absent() {
    assert_eq!(output_path_arg(&json!({})), None);
}

#[test]
fn output_path_arg_is_none_for_non_string_value() {
    assert_eq!(output_path_arg(&json!({ "output_path": 7 })), None);
}

// ---------------------------------------------------------------------------
// mime_type — one MIME string per capture format
// ---------------------------------------------------------------------------

#[test]
fn mime_type_covers_every_format() {
    assert_eq!(mime_type(Format::Png), "image/png");
    assert_eq!(mime_type(Format::Jpg), "image/jpeg");
    assert_eq!(mime_type(Format::Bmp), "image/bmp");
}

#[test]
fn mime_type_is_consistent_with_format_extension() {
    for format in Format::ALL {
        let subtype = mime_type(format)
            .strip_prefix("image/")
            .expect("every capture format must map to an image/* MIME type");
        // "jpg" files carry the standard "jpeg" MIME subtype; every other
        // format's subtype matches its file extension exactly.
        match format {
            Format::Jpg => assert_eq!(subtype, "jpeg"),
            _ => assert_eq!(subtype, format.extension()),
        }
    }
}

// ---------------------------------------------------------------------------
// saved_content — capture result blocks, including the unreadable-file path
// ---------------------------------------------------------------------------

#[test]
fn saved_content_inlines_the_image_when_the_file_is_readable() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rustshot_mcp_saved_{unique}.png"));
    std::fs::write(&path, b"fake image bytes").unwrap();

    let content = saved_content(&path, Format::Png).unwrap();

    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"], "text");
    assert!(content[0]["text"]
        .as_str()
        .unwrap()
        .contains(&path.display().to_string()));
    assert_eq!(content[1]["type"], "image");
    assert_eq!(content[1]["mimeType"], "image/png");
    assert_eq!(content[1]["data"], base64_encode(b"fake image bytes"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn saved_content_degrades_to_text_when_the_file_is_unreadable() {
    let path = std::env::temp_dir().join("rustshot_mcp_saved_missing_never_written.png");
    let _ = std::fs::remove_file(&path);

    // The file was saved by the capture layer; only the inline preview can
    // fail. That must stay Ok with a textual note, not fail the whole call.
    let content = saved_content(&path, Format::Png).unwrap();

    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"], "text");
    assert!(content[0]["text"]
        .as_str()
        .unwrap()
        .starts_with("Saved to "));
    assert_eq!(content[1]["type"], "text");
    assert!(content[1]["text"]
        .as_str()
        .unwrap()
        .contains("could not inline the image"));
}
