//! Dependency rules are executable so shared code cannot grow UI dependencies again.

use std::path::{Path, PathBuf};

#[path = "architecture/dependencies.rs"]
mod dependencies;
#[path = "architecture/policy.rs"]
mod policy;

use policy::{allowed_dependencies, source_layer, violations_at};

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).expect("source directory exists") {
        let path = entry.expect("source entry is readable").path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

#[test]
fn unit_test_bodies_live_outside_production_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let offenders: Vec<_> = rust_sources(&root.join("src"))
        .into_iter()
        .filter(|path| {
            let source = std::fs::read_to_string(path).expect("source is readable");
            source.lines().any(|line| {
                let line = line.trim();
                line == "#[test]" || line == "#[test_case]" || line == "#[test_case()]"
            })
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "unit tests belong in tests/unit: {offenders:?}"
    );
}

#[test]
fn shared_layers_have_explicit_module_roots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for layer in ["domain", "pixels", "platform", "application"] {
        assert!(
            root.join(format!("src/{layer}.rs")).is_file(),
            "missing {layer} layer"
        );
    }
}

#[test]
fn shared_layers_do_not_depend_on_their_consumers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();
    for path in rust_sources(&root.join("src")) {
        let relative = path.strip_prefix(root.join("src")).unwrap();
        let source = std::fs::read_to_string(&path).expect("source is readable");
        violations.extend(violations_at(&relative.to_string_lossy(), &source));
    }
    assert!(
        violations.is_empty(),
        "reverse layer dependencies: {violations:#?}"
    );
}

#[test]
fn dependency_guard_recognizes_grouped_aliases_and_qualified_calls() {
    let found = dependencies::dependencies(
        Path::new("lib.rs"),
        "use crate::{selection as ui, capture::{self, Format}}; fn f() { crate::app::run(); }",
    );
    for layer in ["selection", "capture", "app"] {
        assert!(found
            .iter()
            .any(|dependency| dependency.path.get(1).is_some_and(|name| name == layer)));
    }
    assert!(dependencies::dependencies(
        Path::new("lib.rs"),
        "// crate::app::run()\nconst DOC: &str = \"crate::selection\";"
    )
    .is_empty());
}

#[test]
fn every_production_root_has_an_explicit_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for path in rust_sources(&root) {
        let layer = source_layer(path.strip_prefix(&root).unwrap());
        assert!(
            allowed_dependencies(&layer).is_some(),
            "unclassified root: {layer}"
        );
    }
}

#[test]
fn relative_imports_cannot_bypass_layer_rules() {
    for (path, source) in [
        ("domain.rs", "use super::app as ui;"),
        (
            "domain/geometry.rs",
            "use super::super::{app as ui, capture::{self, Format}};",
        ),
        (
            "domain/geometry.rs",
            "mod nested { fn f() { super::super::super::app::run(); } }",
        ),
        (
            "window.rs",
            "use crate as root; fn f() { root::app::run(); }",
        ),
    ] {
        assert!(!violations_at(path, source).is_empty(), "missed {source}");
    }
}

#[test]
fn relative_resolution_tracks_file_and_inline_module_context() {
    for (file, source, expected) in [
        (
            "domain.rs",
            "use self::geometry::Region;",
            "crate::domain::geometry::Region",
        ),
        (
            "domain/mod.rs",
            "use self::geometry::Region;",
            "crate::domain::geometry::Region",
        ),
        (
            "domain/layout.rs",
            "use self::helper::Region;",
            "crate::domain::layout::helper::Region",
        ),
        (
            "domain/main.rs",
            "use self::helper::Region;",
            "crate::domain::main::helper::Region",
        ),
        (
            "domain/layout.rs",
            "mod nested { use super::super::geometry::Region; }",
            "crate::domain::geometry::Region",
        ),
    ] {
        let found = dependencies::dependencies(Path::new(file), source);
        assert!(
            found
                .iter()
                .any(|dependency| dependency.path.join("::") == expected),
            "{file}: expected {expected}"
        );
    }
}

#[test]
fn domain_cannot_import_external_crates_or_standard_library_effects() {
    for source in [
        "use image::RgbaImage;",
        "extern crate windows as native;",
        "fn f() { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(); }",
        "use std::{fmt, fs};",
        "fn f() { std::process::id(); }",
        "fn f() { std::thread::current(); }",
        "use std::net::TcpStream;",
        "use std as system; fn f() { system::fs::read(\"x\"); }",
        "fn f() { println!(\"side effect\"); }",
    ] {
        assert!(
            !violations_at("domain/geometry.rs", source).is_empty(),
            "missed {source}"
        );
    }
}

#[test]
fn absolute_external_paths_cannot_be_shadowed_by_local_names() {
    for source in [
        "mod windows {} fn f(_: ::windows::Win32::Foundation::POINT) {}",
        "mod windows {} use ::windows::Win32::Foundation::POINT;",
        "mod windows {} use ::windows::Win32::Foundation::POINT as NativePoint;",
        "use std::fmt as windows; fn f(_: ::windows::Win32::Foundation::POINT) {}",
    ] {
        assert!(
            !violations_at("domain.rs", source).is_empty(),
            "missed {source}"
        );
    }
    assert!(violations_at(
        "domain.rs",
        "mod windows { pub struct Local; } fn f(_: self::windows::Local) {}"
    )
    .is_empty());
}

#[test]
fn unknown_roots_and_window_reverse_dependencies_are_rejected() {
    assert!(!violations_at("new_service.rs", "fn f() {}").is_empty());
    assert!(!violations_at("window.rs", "use crate::app;").is_empty());
}

#[test]
fn executable_library_paths_obey_the_composition_boundary() {
    for source in [
        "fn main() { rustshot::capture::capture_screenshot(); }",
        "fn main() { ::rustshot::capture::capture_screenshot(); }",
        "use ::rustshot::capture;",
        "use rustshot as shot; fn main() { shot::capture::capture_screenshot(); }",
        "use rustshot::capture;",
        "extern crate rustshot as shot; fn main() { shot::capture::capture_screenshot(); }",
    ] {
        assert!(
            !violations_at("main.rs", source).is_empty(),
            "missed {source}"
        );
    }
    for source in [
        "fn main() { rustshot::app::run(); }",
        "fn main() { ::rustshot::app::run(); }",
        "use ::rustshot::app;",
        "use rustshot as shot; fn main() { shot::app::run(); }",
        "extern crate rustshot as shot; fn main() { shot::app::run(); }",
    ] {
        assert!(
            violations_at("main.rs", source).is_empty(),
            "rejected {source}"
        );
    }
}

#[test]
fn local_relative_and_pure_standard_library_paths_remain_allowed() {
    for (path, source) in [
        ("domain/layout.rs", "use super::*;"),
        ("domain.rs", "use self::geometry::*; mod geometry {}"),
        (
            "domain/geometry.rs",
            "mod nested { use super::super::PhysicalPoint; }",
        ),
        ("domain/mod.rs", "use super::domain::Format;"),
        (
            "domain.rs",
            "use std::fmt as formatting; fn f(_: formatting::Error) {}",
        ),
        ("domain.rs", "use std::cmp::{self, Ordering};"),
    ] {
        assert!(violations_at(path, source).is_empty(), "rejected {source}");
    }
}
