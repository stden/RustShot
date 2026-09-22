//! Source-level dependencies, not compiler name resolution. Explicit imports,
//! relative paths, lexical aliases and inline modules are inspected on every cfg
//! branch. Macro token expansion, method dispatch and cross-file glob expansion
//! are not reconstructed; each re-export's own declaration is still inspected.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use syn::visit::Visit;

type Scope = BTreeMap<String, Vec<String>>;

pub(super) struct Dependency {
    pub(super) path: Vec<String>,
    pub(super) imported: bool,
}

#[derive(Clone, Copy)]
enum Qualification {
    Lexical,
    Absolute,
}

pub(super) fn module_path(relative: &Path) -> Vec<String> {
    let mut module: Vec<_> = relative
        .components()
        .map(|part| {
            Path::new(part.as_os_str())
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let stem = module.last().map(String::as_str);
    if stem == Some("mod") || (module.len() == 1 && matches!(stem, Some("lib" | "main"))) {
        module.pop();
    }
    module
}

fn relative_path(module: &[String], segments: &[String]) -> Vec<String> {
    let Some(first) = segments.first().map(String::as_str) else {
        return Vec::new();
    };
    let parents = segments.iter().take_while(|part| *part == "super").count();
    if first == "crate" || (first != "self" && parents == 0) {
        return segments.to_vec();
    }
    if parents > module.len() {
        return vec!["invalid_relative_path".into()];
    }
    let mut absolute = vec!["crate".into()];
    absolute.extend_from_slice(&module[..module.len() - parents]);
    absolute.extend_from_slice(&segments[parents.max(1)..]);
    absolute
}

fn use_paths(tree: &syn::UseTree, prefix: Vec<String>, paths: &mut Vec<(Vec<String>, String)>) {
    match tree {
        syn::UseTree::Path(path) => {
            let mut next = prefix;
            next.push(path.ident.to_string());
            use_paths(&path.tree, next, paths);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                use_paths(item, prefix.clone(), paths);
            }
        }
        syn::UseTree::Name(name) => add_use_path(prefix, &name.ident.to_string(), None, paths),
        syn::UseTree::Rename(rename) => add_use_path(
            prefix,
            &rename.ident.to_string(),
            Some(rename.rename.to_string()),
            paths,
        ),
        syn::UseTree::Glob(_) => paths.push((prefix, String::new())),
    }
}

fn add_use_path(
    mut prefix: Vec<String>,
    name: &str,
    alias: Option<String>,
    paths: &mut Vec<(Vec<String>, String)>,
) {
    if name != "self" {
        prefix.push(name.into());
    }
    let binding = alias.unwrap_or_else(|| prefix.last().cloned().unwrap_or_default());
    paths.push((prefix, binding));
}

fn scope_for(items: &[&syn::Item], module: &[String]) -> Scope {
    let mut scope = Scope::new();
    for item in items {
        add_item_bindings(item, module, &mut scope);
    }
    scope
}

fn add_item_bindings(item: &syn::Item, module: &[String], scope: &mut Scope) {
    match item {
        syn::Item::Use(item) => {
            let mut paths = Vec::new();
            use_paths(&item.tree, Vec::new(), &mut paths);
            for (path, binding) in paths {
                scope.insert(binding, relative_path(module, &path));
            }
        }
        syn::Item::Mod(item) => {
            let name = item.ident.to_string();
            let mut path = vec!["crate".into()];
            path.extend_from_slice(module);
            path.push(name.clone());
            scope.insert(name, path);
        }
        syn::Item::ExternCrate(item) => {
            let name = item.ident.to_string();
            let binding = item
                .rename
                .as_ref()
                .map_or_else(|| name.clone(), |(_, alias)| alias.to_string());
            scope.insert(binding, vec![name]);
        }
        _ => {}
    }
}

struct Dependencies {
    module: Vec<String>,
    scopes: Vec<Scope>,
    found: Vec<Dependency>,
}

fn resolve_alias(mut path: Vec<String>, scopes: &[Scope]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    while let Some(first) = path.first() {
        // The executable reaches these same layers through the library facade.
        if first == "rustshot" {
            path[0] = "crate".into();
            break;
        }
        if matches!(first.as_str(), "crate" | "std" | "core" | "alloc")
            || !seen.insert(first.clone())
        {
            break;
        }
        let Some(alias) = scopes.iter().rev().find_map(|scope| scope.get(first)) else {
            break;
        };
        let mut expanded = alias.clone();
        expanded.extend_from_slice(&path[1..]);
        path = expanded;
    }
    path
}

fn record(
    visitor: &mut Dependencies,
    segments: &[String],
    imported: bool,
    qualification: Qualification,
) {
    let path = match qualification {
        Qualification::Lexical => {
            resolve_alias(relative_path(&visitor.module, segments), &visitor.scopes)
        }
        // A leading `::` selects the extern prelude, not a same-named lexical
        // module/import. Empty scopes still normalize our own library facade.
        Qualification::Absolute => resolve_alias(segments.to_vec(), &[]),
    };
    visitor.found.push(Dependency { path, imported });
}

impl<'ast> Visit<'ast> for Dependencies {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let qualification = if path.leading_colon.is_some() {
            Qualification::Absolute
        } else {
            Qualification::Lexical
        };
        let segments: Vec<_> = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        if segments.len() > 1 {
            record(self, &segments, false, qualification);
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let qualification = if item.leading_colon.is_some() {
            Qualification::Absolute
        } else {
            Qualification::Lexical
        };
        let mut paths = Vec::new();
        use_paths(&item.tree, Vec::new(), &mut paths);
        for (path, _) in paths {
            record(self, &path, true, qualification);
        }
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        self.found.push(Dependency {
            path: vec![item.ident.to_string()],
            imported: true,
        });
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let Some((_, items)) = &item.content else {
            return;
        };
        self.module.push(item.ident.to_string());
        self.scopes
            .push(scope_for(&items.iter().collect::<Vec<_>>(), &self.module));
        syn::visit::visit_item_mod(self, item);
        self.scopes.pop();
        self.module.pop();
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let items: Vec<_> = block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                syn::Stmt::Item(item) => Some(item),
                _ => None,
            })
            .collect();
        self.scopes.push(scope_for(&items, &self.module));
        syn::visit::visit_block(self, block);
        self.scopes.pop();
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.is_ident("println")
            || mac.path.is_ident("print")
            || mac.path.is_ident("eprintln")
            || mac.path.is_ident("eprint")
            || mac.path.is_ident("dbg")
        {
            self.found.push(Dependency {
                path: vec!["std".into(), "io".into()],
                imported: false,
            });
        }
        syn::visit::visit_macro(self, mac);
    }
}

pub(super) fn dependencies(relative: &Path, source: &str) -> Vec<Dependency> {
    let syntax = syn::parse_file(source).expect("production source is valid Rust");
    let module = module_path(relative);
    let scopes = vec![scope_for(&syntax.items.iter().collect::<Vec<_>>(), &module)];
    let mut visitor = Dependencies {
        module,
        scopes,
        found: Vec::new(),
    };
    visitor.visit_file(&syntax);
    visitor.found
}
