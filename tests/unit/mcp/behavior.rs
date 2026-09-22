use super::*;
use crate::test_support::TestDir;
use cucumber::{then, when, World};

#[derive(Debug, Default, World)]
struct PreviewWorld {
    directory: Option<TestDir>,
    path: Option<std::path::PathBuf>,
    content: Option<Vec<Value>>,
    bytes: Vec<u8>,
}

#[when(regex = r#"^a saved "(png|jpg|bmp)" screenshot is prepared for an MCP reply$"#)]
fn prepare_preview(world: &mut PreviewWorld, extension: String) {
    let directory = TestDir::new("rustshot_bdd_mcp_preview");
    let path = directory.file(&format!("shot.{extension}"));
    let format = Format::from_settings_str(&extension);
    let image = image::RgbaImage::from_pixel(4, 3, image::Rgba([12, 34, 56, 255]));
    crate::capture::save_image_optimized(&image, &path, format).unwrap();
    world.bytes = std::fs::read(&path).unwrap();
    world.content = Some(saved_content(&path, format).unwrap());
    world.directory = Some(directory);
    world.path = Some(path);
}

// Decode, rather than re-encode with the production helper: a broken alphabet
// or padding rule must not become a self-consistent but invalid preview.
fn decode_base64(text: &str) -> Vec<u8> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    assert_eq!(text.len() % 4, 0);
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in text.trim_end_matches('=').bytes() {
        let value = alphabet
            .iter()
            .position(|candidate| *candidate == byte)
            .unwrap() as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    assert_eq!(buffer, 0, "base64 padding bits must be zero");
    output
}

#[then(regex = r#"^the MCP preview has MIME type "([^"]+)" and exactly the saved bytes$"#)]
fn assert_preview(world: &mut PreviewWorld, mime: String) {
    let content = world.content.as_ref().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["type"], "text");
    assert_eq!(
        content[0]["text"],
        format!("Saved to {}", world.path.as_ref().unwrap().display())
    );
    assert_eq!(content[1]["type"], "image");
    assert_eq!(content[1]["mimeType"], mime);
    let decoded = decode_base64(content[1]["data"].as_str().unwrap());
    assert_eq!(decoded, world.bytes);
    assert_eq!(
        image::load_from_memory(&decoded)
            .unwrap()
            .to_rgba8()
            .dimensions(),
        (4, 3)
    );
}

#[when("a screenshot file disappears before its MCP preview is read")]
fn missing_preview(world: &mut PreviewWorld) {
    let directory = TestDir::new("rustshot_bdd_missing_preview");
    let path = directory.file("saved.png");
    world.content = Some(saved_content(&path, Format::Png).unwrap());
    world.directory = Some(directory);
    world.path = Some(path);
}

#[then("MCP still reports the saved path and explains why the preview is unavailable")]
fn assert_partial_preview(world: &mut PreviewWorld) {
    let content = world.content.as_ref().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(
        content[0]["text"],
        format!("Saved to {}", world.path.as_ref().unwrap().display())
    );
    assert_eq!(content[1]["type"], "text");
    assert!(content[1]["text"]
        .as_str()
        .unwrap()
        .contains("could not inline the image"));
}

#[test]
fn bdd_mcp_image_previews() {
    futures::executor::block_on(
        PreviewWorld::cucumber()
            .with_default_cli()
            .fail_on_skipped()
            .run_and_exit("tests/features/in_process/mcp_previews.feature"),
    );
}
