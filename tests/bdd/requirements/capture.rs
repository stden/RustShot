use crate::{capture, known_format, RustShotWorld};
use cucumber::{then, when};

#[when(regex = r#"^the screenshot pipeline is run with \"([^\"]+)\" failing as \"([^\"]+)\"$"#)]
fn run_capture_pipeline(world: &mut RustShotWorld, stage: String, message: String) {
    let (result, stages) = capture::benchmark_capture_pipeline(&stage, &message);
    world.capture_pipeline_result = Some(result);
    world.capture_pipeline_stages = stages;
}

#[when("the screenshot pipeline is run with every stage succeeding")]
fn run_capture_pipeline_successfully(world: &mut RustShotWorld) {
    // No stage name matches, so every injected stage returns Ok.
    let (result, stages) = capture::benchmark_capture_pipeline("none", "unused");
    world.capture_pipeline_result = Some(result);
    world.capture_pipeline_stages = stages;
}

#[then(regex = r#"^the screenshot pipeline succeeds with \"([^\"]+)\"$"#)]
fn assert_capture_pipeline_success(world: &mut RustShotWorld, expected: String) {
    let result = world
        .capture_pipeline_result
        .as_ref()
        .expect("the screenshot pipeline did not run");
    assert_eq!(
        result.as_ref().ok().map(|path| path.display().to_string()),
        Some(expected)
    );
}

#[then(regex = r#"^the screenshot pipeline fails with \"([^\"]+)\"$"#)]
fn assert_capture_pipeline_error(world: &mut RustShotWorld, expected: String) {
    let result = world
        .capture_pipeline_result
        .as_ref()
        .expect("the screenshot pipeline did not run");
    assert_eq!(
        result.as_ref().err().map(String::as_str),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the screenshot pipeline stages are \"([^\"]+)\"$"#)]
fn assert_capture_pipeline_stages(world: &mut RustShotWorld, expected: String) {
    let expected: Vec<&str> = expected.split(',').collect();
    let actual: Vec<&str> = world
        .capture_pipeline_stages
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected);
}

#[then("the generated screenshot timestamp has the canonical safe shape")]
fn assert_generated_timestamp_shape(world: &mut RustShotWorld) {
    let path = world
        .resolved_path
        .as_ref()
        .expect("no output path was generated");
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("generated path has no UTF-8 file stem");
    let timestamp = stem
        .strip_prefix("screenshot_")
        .expect("generated screenshot name lacks its prefix");
    let bytes = timestamp.as_bytes();

    assert_eq!(bytes.len(), 25, "timestamp: {timestamp}");
    assert_eq!(bytes[8], b'_', "timestamp: {timestamp}");
    assert_eq!(bytes[15], b'_', "timestamp: {timestamp}");
    assert!(
        bytes[..8]
            .iter()
            .chain(&bytes[9..15])
            .chain(&bytes[16..])
            .all(u8::is_ascii_digit),
        "timestamp: {timestamp}"
    );
}

#[when(regex = r#"^a default output path is generated for format \"([a-z]+)\"$"#)]
fn generate_default_output_path(world: &mut RustShotWorld, format: String) {
    world.resolved_path = Some(capture::resolve_output_path(None, known_format(&format)));
}

#[then(regex = r#"^the generated file parent directory is named \"([^\"]+)\"$"#)]
fn assert_generated_parent_name(world: &mut RustShotWorld, expected: String) {
    let parent_name = world
        .resolved_path
        .as_ref()
        .expect("no output path was generated")
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str());
    assert_eq!(parent_name, Some(expected.as_str()));
}

#[when("the same nested screenshot parent directory is prepared three times")]
fn prepare_parent_three_times(world: &mut RustShotWorld) {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "rustshot-bdd-parent-{}-{unique}",
        std::process::id()
    ));
    let output = root.join("a").join("b").join("shot.png");
    let result = (|| {
        for _ in 0..3 {
            capture::benchmark_ensure_parent_directory(&output)?;
        }
        if !output.parent().is_some_and(std::path::Path::exists) {
            return Err("nested parent directory was not created".to_string());
        }
        Ok(())
    })();
    world.screenshot_parent_root = Some(root);
    world.screenshot_parent_result = Some(result);
}

#[then("all screenshot parent directory preparations succeed")]
fn assert_parent_preparations(world: &mut RustShotWorld) {
    let result = world
        .screenshot_parent_result
        .as_ref()
        .expect("the parent directory was not prepared");
    assert!(result.is_ok(), "parent preparation failed: {result:?}");
    if let Some(root) = world.screenshot_parent_root.take() {
        std::fs::remove_dir_all(&root)
            .unwrap_or_else(|error| panic!("failed to clean {}: {error}", root.display()));
    }
}

#[when(regex = r#"^capture format \"([a-z]+)\" is mapped to an image encoder$"#)]
fn map_image_encoder(world: &mut RustShotWorld, format: String) {
    world.image_encoder = Some(format!("{:?}", known_format(&format).to_image_format()));
}

#[then(regex = r#"^the image encoder is \"([A-Za-z]+)\"$"#)]
fn assert_image_encoder(world: &mut RustShotWorld, expected: String) {
    assert_eq!(world.image_encoder.as_deref(), Some(expected.as_str()));
}
