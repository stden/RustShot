//! Dependency rules are executable so shared code cannot grow UI dependencies again.

use std::path::{Path, PathBuf};
use syn::visit::Visit;

#[derive(Default)]
struct Dependencies(Vec<String>);

impl Dependencies {
    fn record(&mut self, segments: &[String]) {
        if segments.len() > 1 && segments[0] == "crate" {
            self.0.push(segments[1].clone());
        }
    }

    fn use_tree(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.use_tree(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.use_tree(item, prefix);
                }
            }
            syn::UseTree::Name(name) => {
                prefix.push(name.ident.to_string());
                self.record(prefix);
                prefix.pop();
            }
            syn::UseTree::Rename(rename) => {
                prefix.push(rename.ident.to_string());
                self.record(prefix);
                prefix.pop();
            }
            syn::UseTree::Glob(_) => self.record(prefix),
        }
    }
}

impl<'ast> Visit<'ast> for Dependencies {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.record(
            &path
                .segments
                .iter()
                .map(|part| part.ident.to_string())
                .collect::<Vec<_>>(),
        );
        syn::visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.use_tree(&item.tree, &mut Vec::new());
    }
}

fn dependencies(source: &str) -> Vec<String> {
    let syntax = syn::parse_file(source).expect("production source is valid Rust");
    let mut dependencies = Dependencies::default();
    dependencies.visit_file(&syntax);
    dependencies.0
}

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
    // Same-layer modules are allowed; arrows point toward shared policy and
    // adapters. Test-only compatibility aliases are checked too, so they must
    // not quietly recreate a production dependency cycle.
    let rules: &[(&str, &[&str])] = &[
        ("domain", &["domain"]),
        ("pixels", &["pixels", "domain"]),
        ("platform", &["platform", "pixels", "domain", "timing"]),
        (
            "capture",
            &[
                "capture", "domain", "pixels", "platform", "settings", "timing",
            ],
        ),
        (
            "record",
            &[
                "record", "domain", "pixels", "platform", "settings", "timing",
            ],
        ),
        ("settings", &["settings", "domain", "platform"]),
        (
            "selection",
            &["selection", "domain", "pixels", "platform", "timing"],
        ),
        ("cli", &["cli", "domain"]),
        (
            "application",
            &[
                "application",
                "domain",
                "platform",
                "capture",
                "record",
                "settings",
                "window",
            ],
        ),
    ];
    let mut violations = Vec::new();
    for path in rust_sources(&root.join("src")) {
        let relative = path.strip_prefix(root.join("src")).unwrap();
        let first = relative.components().next().unwrap().as_os_str();
        let layer = Path::new(first).file_stem().unwrap().to_str().unwrap();
        let Some((_, allowed)) = rules.iter().find(|(name, _)| *name == layer) else {
            continue;
        };
        let source = std::fs::read_to_string(&path).expect("source is readable");
        for dependency in dependencies(&source) {
            if !allowed.contains(&dependency.as_str()) {
                violations.push(format!("{} -> {dependency}", relative.display()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "reverse layer dependencies: {violations:#?}"
    );
}

#[test]
fn dependency_guard_recognizes_grouped_aliases_and_qualified_calls() {
    let found = dependencies(
        "use crate::{selection as ui, capture::{self, Format}}; fn f() { crate::app::run(); }",
    );
    assert!(found.iter().any(|name| name == "selection"));
    assert!(found.iter().any(|name| name == "capture"));
    assert!(found.iter().any(|name| name == "app"));
    assert!(
        dependencies("// crate::app::run()\nconst DOC: &str = \"crate::selection\";").is_empty()
    );
}
