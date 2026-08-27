use crate::{capture, cell, settings, table_rows, RustShotWorld};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use std::path::PathBuf;

#[given("application settings:")]
fn application_settings(world: &mut RustShotWorld, step: &Step) {
    let rows = table_rows(step);
    assert_eq!(rows.len(), 1, "the settings table needs exactly one row");
    let row = &rows[0];
    world.settings_input = Some(settings::Settings {
        screenshots_dir: PathBuf::from(&row[0]),
        recordings_dir: PathBuf::from(&row[1]),
        default_format: capture::Format::from_settings_str(&row[2]),
        recording_fps: settings::Fps::new(cell(row, 3)).expect("invalid FPS in feature"),
        auto_start: cell(row, 4),
    });
}

#[given("default application settings")]
fn default_settings(world: &mut RustShotWorld) {
    world.settings_input = Some(settings::Settings::default());
}

#[when("the settings are serialized and parsed")]
fn serialize_and_parse_settings(world: &mut RustShotWorld) {
    let serialized = settings::format_settings_json(
        world
            .settings_input
            .as_ref()
            .expect("application settings were not provided"),
    );
    world.parsed_settings = Some(settings::parse_settings_json(&serialized));
    world.serialized_settings = Some(serialized);
}

#[when("the settings are serialized")]
fn serialize_settings(world: &mut RustShotWorld) {
    world.serialized_settings = Some(settings::format_settings_json(
        world
            .settings_input
            .as_ref()
            .expect("application settings were not provided"),
    ));
}

#[then("every settings field matches the original")]
fn assert_settings_roundtrip(world: &mut RustShotWorld) {
    assert_eq!(world.parsed_settings, world.settings_input);
}

#[then("the serialized settings are valid JSON")]
fn assert_valid_settings_json(world: &mut RustShotWorld) {
    let serialized = world
        .serialized_settings
        .as_deref()
        .expect("settings were not serialized");
    serde_json::from_str::<serde_json::Value>(serialized)
        .unwrap_or_else(|error| panic!("settings JSON is invalid: {error}\n{serialized}"));
}

#[then("every settings schema key occurs exactly once")]
fn assert_settings_keys_once(world: &mut RustShotWorld) {
    let serialized = world
        .serialized_settings
        .as_deref()
        .expect("settings were not serialized");
    for key in [
        "screenshots_dir",
        "recordings_dir",
        "default_format",
        "recording_fps",
        "auto_start",
    ] {
        assert_eq!(
            serialized.matches(&format!("\"{key}\"")).count(),
            1,
            "key {key:?} in {serialized}"
        );
    }
}

#[when("settings JSON is parsed:")]
fn parse_settings_json(world: &mut RustShotWorld, step: &Step) {
    let json = step
        .docstring
        .as_deref()
        .expect("this step requires a JSON docstring")
        .trim();
    world.parsed_settings = Some(settings::parse_settings_json(json));
}

fn parsed_settings(world: &RustShotWorld) -> &settings::Settings {
    world
        .parsed_settings
        .as_ref()
        .expect("settings JSON was not parsed")
}

#[then(regex = r#"^the parsed settings format is \"([a-z]+)\"$"#)]
fn assert_parsed_format(world: &mut RustShotWorld, expected: String) {
    assert_eq!(parsed_settings(world).default_format.extension(), expected);
}

#[then("the other parsed settings fields equal their defaults")]
fn assert_other_fields_default(world: &mut RustShotWorld) {
    let actual = parsed_settings(world);
    let expected = settings::Settings::default();
    assert_eq!(actual.screenshots_dir, expected.screenshots_dir);
    assert_eq!(actual.recordings_dir, expected.recordings_dir);
    assert_eq!(actual.recording_fps, expected.recording_fps);
    assert_eq!(actual.auto_start, expected.auto_start);
}

#[then("all parsed settings fields equal their defaults")]
fn assert_all_fields_default(world: &mut RustShotWorld) {
    assert_eq!(parsed_settings(world), &settings::Settings::default());
}

#[then(regex = r"^the parsed recording FPS is (\d+)$")]
fn assert_parsed_fps(world: &mut RustShotWorld, expected: u32) {
    assert_eq!(parsed_settings(world).recording_fps.get(), expected);
}

#[then(regex = r"^parsed auto-start is (true|false)$")]
fn assert_parsed_auto_start(world: &mut RustShotWorld, expected: bool) {
    assert_eq!(parsed_settings(world).auto_start, expected);
}
