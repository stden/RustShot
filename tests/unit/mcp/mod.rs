//! Unit tests for the MCP server: protocol routing via [`handle_message`] and
//! the pure argument/content helpers.

use super::*;

mod behavior;

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
fn base64_encodes_the_last_two_alphabet_entries() {
    // The RFC 4648 vectors above never reach indices 62 and 63, so a typo in the
    // tail of the alphabet ("+/" — the only two non-alphanumeric characters)
    // would pass every one of them while corrupting real screenshot bytes.
    assert_eq!(base64_encode(&[0xFB, 0xEF, 0xBE]), "++++");
    assert_eq!(base64_encode(&[0xFF, 0xFF, 0xFF]), "////");
    // Both entries in one value, with padding.
    assert_eq!(base64_encode(&[0xFB, 0xFF]), "+/8=");
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

#[test]
fn message_without_a_method_is_an_invalid_request() {
    // A request carrying an id but no method is well-formed JSON, so it gets past
    // the parse-error branch. It must be answered, not treated as a notification
    // and silently dropped, or the client waits for a reply that never comes.
    let res = parse(&handle_message(r#"{"jsonrpc":"2.0","id":11}"#).unwrap());
    assert_eq!(res["id"], 11);
    assert_eq!(res["error"]["code"], -32600);
}

#[test]
fn invalid_request_envelopes_are_rejected_before_dispatch() {
    for request in [
        "null",
        "42",
        "{}",
        r#"{"id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","method":5}"#,
        r#"{"jsonrpc":"2.0","id":true,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":{},"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":null}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":7}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":"bad"}"#,
    ] {
        let response = handle_message(request).expect("invalid requests need an error reply");
        assert_eq!(parse(&response)["error"]["code"], -32600, "{request}");
    }
}

#[test]
fn invalid_request_ids_are_replaced_with_null() {
    for id in [json!(true), json!({}), json!([])] {
        let request = json!({"jsonrpc":"2.0", "id":id, "method":"ping"});
        let response = parse(&handle_message(&request.to_string()).unwrap());
        assert!(response["id"].is_null());
        assert_eq!(response["error"]["code"], INVALID_REQUEST);
    }
}

#[test]
fn negative_screen_index_is_rejected_instead_of_capturing_primary() {
    // A directory cannot be replaced by the image writer if validation regresses.
    let result = call_tool(
        "capture_screen",
        &json!({
            "screen": -1,
            "output_path": std::env::temp_dir(),
        }),
    );
    assert!(result.unwrap_err().contains("screen"));
}

#[test]
fn tool_arguments_must_be_an_object() {
    for arguments in [Value::Null, json!([]), json!(7), json!("bad")] {
        let result = call_tool("capture_window", &arguments);
        assert!(result.unwrap_err().contains("arguments"));
    }
}

#[test]
fn screen_argument_defaults_only_when_absent() {
    assert_eq!(screen_arg(&json!({})), Ok(None));
    assert_eq!(screen_arg(&json!({"screen": 0})), Ok(Some(0)));
    for screen in [Value::Null, json!(-1), json!(1.5), json!("0"), json!(true)] {
        assert!(screen_arg(&json!({"screen": screen})).is_err());
    }
}

#[test]
fn explicit_null_id_is_answered_like_any_other_request() {
    // `"id": null` is a request with a null id; only an *absent* id makes a
    // notification. Conflating the two would leave the client unanswered.
    let res = parse(&handle_message(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#).unwrap());
    assert!(res["id"].is_null());
    assert!(res["result"].is_object());
    assert!(res["error"].is_null());
}

#[test]
fn tools_call_without_params_is_a_tool_error_not_a_panic() {
    // `params` is optional in the wire format, so a client can omit it entirely.
    // That must reach the same "unknown tool" result as any other bad name
    // rather than unwrapping a missing field.
    let res = parse(&handle_message(r#"{"jsonrpc":"2.0","id":12,"method":"tools/call"}"#).unwrap());
    assert!(res["error"].is_null());
    assert_eq!(res["result"]["isError"], true);
    assert!(res["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .starts_with("Unknown tool:"));
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
