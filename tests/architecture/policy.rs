use super::dependencies::{dependencies, Dependency};
use std::path::Path;

// Every source root is classified, including composition and presentation. A new
// root must choose a policy instead of silently falling out of the layer check.
pub(super) fn allowed_dependencies(layer: &str) -> Option<&'static [&'static str]> {
    match layer {
        "lib" | "app" => Some(&[
            "app",
            "application",
            "capture",
            "cli",
            "domain",
            "mcp",
            "pixels",
            "platform",
            "record",
            "selection",
            "settings",
            "settings_dialog",
            "timing",
            "tray",
            "window",
        ]),
        "main" => Some(&["app"]),
        "domain" => Some(&["domain"]),
        "pixels" => Some(&["pixels", "domain"]),
        "platform" => Some(&["platform", "pixels", "domain", "timing"]),
        "capture" => Some(&[
            "capture", "domain", "pixels", "platform", "settings", "timing",
        ]),
        "record" => Some(&[
            "record", "domain", "pixels", "platform", "settings", "timing",
        ]),
        "settings" => Some(&["settings", "domain", "platform"]),
        "selection" => Some(&["selection", "domain", "pixels", "platform", "timing"]),
        "cli" => Some(&["cli", "domain"]),
        "application" => Some(&[
            "application",
            "domain",
            "platform",
            "capture",
            "record",
            "settings",
            "window",
        ]),
        "window" => Some(&[
            "window", "capture", "domain", "platform", "settings", "timing",
        ]),
        "mcp" => Some(&[
            "mcp",
            "application",
            "capture",
            "cli",
            "domain",
            "settings",
            "window",
        ]),
        "tray" => Some(&["tray", "domain"]),
        "settings_dialog" => Some(&[
            "settings_dialog",
            "application",
            "settings",
            "domain",
            "platform",
            "timing",
        ]),
        "timing" => Some(&["timing"]),
        _ => None,
    }
}

fn pure_standard_path(path: &[String]) -> bool {
    // These modules provide values, formatting and in-memory algorithms. Missing
    // entries fail closed: fs/process/thread/net/io/env/time are deliberately absent.
    let allowed = [
        "array",
        "borrow",
        "boxed",
        "char",
        "clone",
        "cmp",
        "collections",
        "convert",
        "default",
        "error",
        "f32",
        "f64",
        "fmt",
        "hash",
        "i8",
        "i16",
        "i32",
        "i64",
        "i128",
        "isize",
        "iter",
        "marker",
        "mem",
        "num",
        "ops",
        "option",
        "result",
        "slice",
        "str",
        "string",
        "u8",
        "u16",
        "u32",
        "u64",
        "u128",
        "usize",
        "vec",
    ];
    path.get(1)
        .is_some_and(|module| allowed.contains(&module.as_str()))
}

fn pure_dependency(dependency: &Dependency) -> bool {
    let path = &dependency.path;
    let Some(root) = path.first() else {
        return true;
    };
    if matches!(root.as_str(), "std" | "core" | "alloc") {
        return pure_standard_path(path);
    }
    if root == "crate" {
        return true; // The ordinary layer policy checks its destination.
    }
    // Associated paths on local/prelude types and generic parameters are not
    // external crate imports. Rust's type resolution is outside this source guard;
    // explicit imports remain checked even when their alias is capitalized.
    if !dependency.imported && root.starts_with(char::is_uppercase) {
        return true;
    }
    !dependency.imported
        && [
            "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "str", "u8",
            "u16", "u32", "u64", "u128", "usize",
        ]
        .contains(&root.as_str())
}

pub(super) fn source_layer(relative: &Path) -> String {
    let first = relative
        .components()
        .next()
        .expect("source path is not empty");
    Path::new(first.as_os_str())
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

pub(super) fn violations_at(relative: &str, source: &str) -> Vec<String> {
    let path = Path::new(relative);
    let layer = source_layer(path);
    let Some(allowed) = allowed_dependencies(&layer) else {
        return vec![format!("unclassified production root: {layer}")];
    };
    dependencies(path, source)
        .into_iter()
        .filter_map(|dependency| {
            let parts = &dependency.path;
            let reverse = parts.first().is_some_and(|root| root == "crate")
                && parts
                    .get(1)
                    .is_some_and(|target| !allowed.contains(&target.as_str()));
            let impure = layer == "domain" && !pure_dependency(&dependency);
            (reverse || impure).then(|| format!("{relative} -> {}", parts.join("::")))
        })
        .collect()
}
