use super::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

// Drain both pipes while waiting: an inline screenshot can exceed the pipe
// capacity. A broken EOF implementation must fail promptly rather than hang CI.
fn run_process(command: &mut Command, input: &str) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch RustShot");
    let mut stdin = child.stdin.take().unwrap();
    let input = input.as_bytes().to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let read = |mut pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).expect("read child output");
            bytes
        })
    };
    let stdout = read(Box::new(child.stdout.take().unwrap()));
    let stderr = read(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("RustShot did not exit after stdin EOF within 15 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    writer.join().unwrap().expect("write MCP transcript");
    Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    }
}

#[when("the RustShot MCP process receives this transcript before EOF:")]
fn mcp_transcript(world: &mut RustShotWorld, step: &cucumber::gherkin::Step) {
    let process = run_process(
        Command::new(env!("CARGO_BIN_EXE_RustShot")).args(["--mcp", "--list"]),
        &format!("\n{}\n\n", step.docstring.as_ref().unwrap()),
    );
    world.workflows.replies = String::from_utf8(process.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout contains only JSON-RPC replies"))
        .collect();
    world.workflows.process = Some(process);
}

#[then(regex = r"^the MCP process exits successfully with (\d+) replies$")]
fn assert_mcp_exit(world: &mut RustShotWorld, count: usize) {
    let process = world.workflows.process.as_ref().unwrap();
    assert!(
        process.status.success(),
        "{}",
        String::from_utf8_lossy(&process.stderr)
    );
    assert_eq!(world.workflows.replies.len(), count);
}

#[then("the ordered MCP replies contain:")]
fn assert_transcript(world: &mut RustShotWorld, step: &cucumber::gherkin::Step) {
    for row in step.table.as_ref().unwrap().rows.iter().skip(1) {
        let index: usize = row[0].parse().unwrap();
        let expected: Value = serde_json::from_str(&row[2]).unwrap();
        assert_eq!(
            world.workflows.replies[index].pointer(&row[1]),
            Some(&expected),
            "{row:?}"
        );
    }
}

#[when("the executable lists its available screens and windows")]
fn list_desktop(world: &mut RustShotWorld) {
    for args in [["--list"], ["--list-windows"]] {
        let output = run_process(Command::new(env!("CARGO_BIN_EXE_RustShot")).args(args), "");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        let expected = if args[0] == "--list" {
            text.starts_with("Available screens:")
        } else {
            text.starts_with("Open windows:") || text.trim() == "No open windows found."
        };
        assert!(expected, "unexpected listing: {text}");
    }
    world.workflows.listed = Some(true);
}

#[then("both listing commands finish without starting a capture")]
fn assert_desktop_listing(world: &mut RustShotWorld) {
    assert_eq!(world.workflows.listed, Some(true));
}

#[when(regex = r#"^the MCP tool "(list_screens|list_windows)" is called$"#)]
fn call_listing_tool(world: &mut RustShotWorld, tool: String) {
    let request = json!({"jsonrpc":"2.0", "id":9, "method":"tools/call", "params":{"name":tool}});
    world.mcp_reply = Some(rustshot::mcp::handle_message(&request.to_string()));
}

#[cfg(windows)]
#[when("MCP captures screen zero and an owned changing window over stdio")]
fn capture_over_stdio(world: &mut RustShotWorld) {
    let fixture = crate::capture_fixture::CaptureWindow::new();
    let directory = support::TestDir::new("rustshot_bdd_mcp_capture");
    let screen_path = directory.file("screen.png");
    let window_path = directory.file("window.jpg");
    let screen = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
        "name":"capture_screen", "arguments":{"screen":0, "output_path":screen_path}
    }});
    let window = json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{
        "name":"capture_window", "arguments":{"id":fixture.id(), "format":"jpg", "output_path":window_path}
    }});
    let process = run_process(
        Command::new(env!("CARGO_BIN_EXE_RustShot")).arg("--mcp"),
        &format!("{screen}\n{window}\n"),
    );
    assert!(
        process.status.success(),
        "{}",
        String::from_utf8_lossy(&process.stderr)
    );
    world.workflows.replies = String::from_utf8(process.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    world.workflows.directory = Some(directory);
}

#[cfg(windows)]
#[then("both captures return saved files and inline previews in the requested formats")]
fn assert_transport_captures(world: &mut RustShotWorld) {
    assert_eq!(world.workflows.replies.len(), 2);
    for (index, file, mime, encoding) in [
        (0, "screen.png", "image/png", image::ImageFormat::Png),
        (1, "window.jpg", "image/jpeg", image::ImageFormat::Jpeg),
    ] {
        let reply = &world.workflows.replies[index];
        assert_eq!(reply["id"], index + 1);
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        let content = &reply["result"]["content"];
        let path = world.workflows.directory.as_ref().unwrap().file(file);
        assert_eq!(content[0]["text"], format!("Saved to {}", path.display()));
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["mimeType"], mime);
        assert!(content[1]["data"].as_str().unwrap().len() > 100);
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), encoding);
        let image = image::load_from_memory(&bytes).unwrap();
        assert!(image.width() > 100 && image.height() > 100);
    }
}

#[cfg(windows)]
#[when("a small desktop region is recorded while an owned window changes")]
fn record_changing_region(world: &mut RustShotWorld) {
    let _fixture = crate::capture_fixture::CaptureWindow::new();
    let directory = support::TestDir::new("rustshot_bdd_recording");
    let screen = screenshots::Screen::all().unwrap()[0].display_info;
    let region = rustshot::selection::Region {
        physical_x: 20,
        physical_y: 20,
        physical_width: 64,
        physical_height: 64,
    };
    let session = record::RecordingSession::start_in_dir(
        screen.x,
        screen.y,
        region,
        rustshot::settings::Fps::new(10).unwrap(),
        directory.path().to_owned(),
    )
    .unwrap();
    // The fixture continuously repaints, including while the native encoder
    // starts, so this exercises live recording instead of an idle desktop.
    let deadline = Instant::now() + Duration::from_secs(2);
    while !session.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    world.workflows.path = Some(session.stop().expect("stop and finalize native recording"));
    world.workflows.directory = Some(directory);
}

#[cfg(windows)]
#[then("stopping produces an MP4 with encoded media and a finalized movie index")]
fn assert_finalized_recording(world: &mut RustShotWorld) {
    let path = world.workflows.path.as_ref().unwrap();
    assert_eq!(
        path.parent(),
        Some(world.workflows.directory.as_ref().unwrap().path())
    );
    assert_eq!(path.extension().unwrap(), "mp4");
    let bytes = std::fs::read(path).unwrap();
    let mut offset = 0;
    let mut boxes = std::collections::BTreeSet::new();
    while offset < bytes.len() {
        assert!(bytes.len() - offset >= 8, "truncated MP4 box header");
        let short_size = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let (size, header) = match short_size {
            0 => (bytes.len() - offset, 8),
            1 => {
                assert!(
                    bytes.len() - offset >= 16,
                    "truncated extended MP4 box header"
                );
                (
                    usize::try_from(u64::from_be_bytes(
                        bytes[offset + 8..offset + 16].try_into().unwrap(),
                    ))
                    .unwrap(),
                    16,
                )
            }
            size => (size as usize, 8),
        };
        assert!(
            size >= header && size <= bytes.len() - offset,
            "invalid MP4 box length"
        );
        let name = &bytes[offset + 4..offset + 8];
        if name == b"mdat" || name == b"moov" {
            assert!(size > header, "empty media or index");
        }
        boxes.insert(name);
        offset += size;
    }
    for required in [b"ftyp", b"mdat", b"moov"] {
        assert!(
            boxes.contains(required.as_slice()),
            "missing MP4 box {required:?}"
        );
    }
    assert!(
        bytes.windows(4).any(|tag| tag == b"avc1"),
        "the video track must use H.264"
    );
}
