//! Prevent an advertised command or separately hosted feature losing its BDD coverage.
use cucumber::gherkin::Feature;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn feature_paths(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(feature_paths(&path));
        } else if path
            .extension()
            .is_some_and(|extension| extension == "feature")
        {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn features() -> Vec<Feature> {
    feature_paths(Path::new("tests/features"))
        .into_iter()
        .map(|path| Feature::parse_path(path, Default::default()).unwrap())
        .collect()
}

#[test]
fn every_tray_command_and_mcp_tool_has_a_behavioral_specification() {
    let mut tags = BTreeSet::new();
    for feature in features() {
        assert!(!feature.scenarios.is_empty() || !feature.rules.is_empty());
        tags.extend(feature.tags);
        for scenario in feature.scenarios {
            tags.extend(scenario.tags);
        }
        for rule in feature.rules {
            tags.extend(rule.tags);
            for scenario in rule.scenarios {
                tags.extend(scenario.tags);
            }
        }
    }
    for command in rustshot::tray::MenuAction::ALL {
        assert!(
            tags.contains(&format!("tray_{}", command.id())),
            "missing BDD scenario for {}",
            command.id()
        );
    }
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    let reply: serde_json::Value =
        serde_json::from_str(&rustshot::mcp::handle_message(request).unwrap()).unwrap();
    for tool in reply["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        assert!(
            tags.contains(&format!("mcp_{name}")),
            "missing BDD scenario for MCP tool {name}"
        );
    }
}

#[test]
fn every_in_process_feature_has_an_explicit_private_runner() {
    let runners = BTreeMap::from([
        ("settings_files.feature", "tests/unit/settings/behavior.rs"),
        (
            "windows_settings.feature",
            "tests/unit/settings_dialog/behavior.rs",
        ),
        ("tray_commands.feature", "tests/unit/app/tray_behavior.rs"),
        (
            "selection_interaction.feature",
            "tests/unit/selection/behavior.rs",
        ),
        (
            "recording_lifecycle.feature",
            "tests/unit/record/behavior.rs",
        ),
        ("tray_runtime.feature", "tests/unit/tray/behavior.rs"),
        ("mcp_previews.feature", "tests/unit/mcp/behavior.rs"),
    ]);
    let mut hosted = BTreeSet::new();
    for path in feature_paths(Path::new("tests/features")) {
        let feature = Feature::parse_path(&path, Default::default()).unwrap();
        if !feature.tags.iter().any(|tag| tag == "in_process") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap();
        let runner = runners
            .get(name)
            .unwrap_or_else(|| panic!("no private BDD runner for {name}"));
        let source = std::fs::read_to_string(runner).unwrap();
        assert!(
            source.contains(&format!("tests/features/in_process/{name}")),
            "{runner} does not run {name}"
        );
        assert!(
            source.contains("fail_on_skipped()"),
            "undefined steps must fail {runner}"
        );
        hosted.insert(name.to_string());
    }
    assert_eq!(
        hosted,
        runners.keys().map(|name| (*name).to_string()).collect()
    );
}
