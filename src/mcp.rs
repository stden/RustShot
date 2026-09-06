#![expect(
    clippy::needless_pass_by_value,
    reason = "JSON values are consumed during serialization"
)]
//! MCP (Model Context Protocol) server, exposing `RustShot`'s capture tools to an
//! MCP client such as an AI assistant.
//!
//! Transport is the MCP stdio transport: newline-delimited JSON-RPC 2.0 on
//! stdin/stdout, one message per line. Start it with `RustShot.exe --mcp`.
//!
//! # Tools
//!
//! - `list_screens` — enumerate displays.
//! - `list_windows` — enumerate visible, titled top-level windows (Windows only).
//! - `capture_screen` — screenshot a display; returns the saved path and the
//!   image inline.
//! - `capture_window` — screenshot one window by id; returns the saved path and
//!   the image inline.
//!
//! # Structure
//!
//! [`handle_message`] is a pure function: message string in, optional response
//! string out. All the protocol logic lives there and is unit-tested without any
//! I/O. [`run`] is the thin stdin/stdout loop around it.

use crate::capture::{self, Format};
use crate::window;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// MCP protocol revision this server implements.
const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "rustshot";
const SERVER_VERSION: &str = "0.1.0";

// JSON-RPC 2.0 error codes (subset we emit).
const PARSE_ERROR: i64 = -32700;
const METHOD_NOT_FOUND: i64 = -32601;

/// Serve MCP over stdio until stdin reaches end-of-file.
///
/// Each input line is one JSON-RPC message; each reply is written as one line
/// and flushed immediately so the client is never left waiting on a buffer.
/// Notifications produce no reply. A malformed line yields a JSON-RPC parse
/// error rather than terminating the server.
pub fn run() -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("Failed to read stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_message(&line) {
            writeln!(stdout, "{response}").map_err(|e| format!("Failed to write stdout: {e}"))?;
            stdout
                .flush()
                .map_err(|e| format!("Failed to flush stdout: {e}"))?;
        }
    }
    Ok(())
}

/// Route one JSON-RPC message.
///
/// Returns `Some(response)` for a request (a message with an `id`) and `None`
/// for a notification (no `id`), which by JSON-RPC rules must not be answered.
pub fn handle_message(input: &str) -> Option<String> {
    let message: Value = match serde_json::from_str(input) {
        Ok(value) => value,
        // Per JSON-RPC, a parse error is reported against a null id.
        Err(e) => {
            return Some(error_response(
                &Value::Null,
                PARSE_ERROR,
                &format!("Parse error: {e}"),
            ))
        }
    };

    // No `id` means this is a notification (e.g. notifications/initialized): we
    // act on nothing and reply with nothing.
    let id = message.get("id")?.clone();
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");

    let response = match method {
        "initialize" => success_response(&id, initialize_result()),
        "tools/list" => success_response(&id, json!({ "tools": tool_definitions() })),
        "tools/call" => success_response(&id, call_tool_result(message.get("params"))),
        // MCP keep-alive.
        "ping" => success_response(&id, json!({})),
        other => error_response(&id, METHOD_NOT_FOUND, &format!("Method not found: {other}")),
    };
    Some(response)
}

/// The `initialize` result: protocol version, declared capabilities, identity.
fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
    })
}

/// The tool catalogue returned by `tools/list`.
///
/// Kept as one function so the schema is defined in exactly one place and can be
/// asserted in tests.
fn tool_definitions() -> Value {
    let format_property = json!({
        "type": "string",
        "enum": ["png", "jpg", "bmp"],
        "description": "Image format (default: png)",
    });
    json!([
        {
            "name": "list_screens",
            "description": "List available displays as human-readable lines.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "list_windows",
            "description": "List open windows as '<id>  <title>' lines. The id is used by capture_window. Windows only.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "capture_screen",
            "description": "Capture a display to an image file and return the saved path and the image.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "screen": { "type": "integer", "minimum": 0, "description": "0-based display index (default: primary)" },
                    "format": format_property,
                    "output_path": { "type": "string", "description": "Explicit output file path (optional)" },
                },
            },
        },
        {
            "name": "capture_window",
            "description": "Capture one window by id (from list_windows) and return the saved path and the image. Windows only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Window id from list_windows" },
                    "format": format_property,
                    "output_path": { "type": "string", "description": "Explicit output file path (optional)" },
                },
                "required": ["id"],
            },
        },
    ])
}

/// Build the `tools/call` result. Tool failures are reported the MCP way — a
/// successful JSON-RPC response carrying `isError: true` — so the client can
/// show the message to the model rather than treating it as a transport fault.
fn call_tool_result(params: Option<&Value>) -> Value {
    let params = params.cloned().unwrap_or(Value::Null);
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match call_tool(name, &arguments) {
        Ok(content) => json!({ "content": content, "isError": false }),
        Err(message) => json!({ "content": [text_content(&message)], "isError": true }),
    }
}

/// Execute a tool, returning its MCP content blocks on success.
fn call_tool(name: &str, args: &Value) -> Result<Vec<Value>, String> {
    match name {
        "list_screens" => Ok(vec![text_content(&crate::cli::format_screen_list_output(
            capture::list_screens(),
        ))]),
        "list_windows" => Ok(vec![text_content(&window::format_window_list(
            &window::list_windows(),
        ))]),
        "capture_screen" => {
            let screen = args
                .get("screen")
                .and_then(Value::as_u64)
                .map(|n| n as usize);
            let format = format_arg(args);
            let output = output_path_arg(args);
            let path = capture::capture_screenshot(output, format, screen)?;
            saved_content(&path, format)
        }
        "capture_window" => {
            let id = args
                .get("id")
                .and_then(Value::as_u64)
                .ok_or("capture_window requires an integer 'id'")?;
            let format = format_arg(args);
            let output = output_path_arg(args);
            let path = window::capture_window(id, output, format)?;
            saved_content(&path, format)
        }
        other => Err(format!("Unknown tool: {other}")),
    }
}

/// Read `format` from tool arguments, defaulting to PNG for anything missing or
/// unrecognised (same forgiving rule as settings).
fn format_arg(args: &Value) -> Format {
    args.get("format")
        .and_then(Value::as_str)
        .map(Format::from_settings_str)
        .unwrap_or(Format::Png)
}

fn output_path_arg(args: &Value) -> Option<std::path::PathBuf> {
    args.get("output_path")
        .and_then(Value::as_str)
        .map(std::path::PathBuf::from)
}

/// Content blocks for a successful capture: the saved path as text, plus the
/// image itself inline so an MCP client can view it directly.
fn saved_content(path: &std::path::Path, format: Format) -> Result<Vec<Value>, String> {
    let mut content = vec![text_content(&format!("Saved to {}", path.display()))];
    match std::fs::read(path) {
        Ok(bytes) => content.push(image_content(&bytes, mime_type(format))),
        // The file was saved; only the inline preview failed. Say so rather than
        // failing the whole call.
        Err(e) => content.push(text_content(&format!("(could not inline the image: {e})"))),
    }
    Ok(content)
}

fn text_content(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn image_content(bytes: &[u8], mime: &str) -> Value {
    json!({ "type": "image", "data": base64_encode(bytes), "mimeType": mime })
}

fn mime_type(format: Format) -> &'static str {
    match format {
        Format::Png => "image/png",
        Format::Jpg => "image/jpeg",
        Format::Bmp => "image/bmp",
    }
}

fn success_response(id: &Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error_response(id: &Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

/// Standard base64 (RFC 4648) with `=` padding.
///
/// Hand-rolled to avoid a dependency for ~20 lines: MCP image blocks need the
/// screenshot bytes as base64, and the encoding is small and fully specified.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let triple = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);

        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
#[path = "../tests/unit/mcp/mod.rs"]
mod tests;
