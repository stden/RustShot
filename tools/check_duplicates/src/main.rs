//! Code duplication detector for Rust source files.
//!
//! Scans all `*.rs` files under a given directory (default: `src/`) and reports
//! any block of [`MIN_LINES`] or more consecutive non-blank lines that appears
//! verbatim in more than one location.
//!
//! # Usage
//!
//! ```text
//! cargo run --manifest-path tools/check_duplicates/Cargo.toml [-- <dir>]
//! ```
//!
//! Exit codes: `0` = clean, `1` = duplicates found, `2` = argument error.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs, process};

/// Minimum block length (in meaningful lines) to flag as duplication.
const MIN_LINES: usize = 5;

/// Directories to skip during recursive file discovery.
const SKIP_DIRS: &[&str] = &["target", ".git"];

/// Return `(1-based line number, stripped content)` for every non-blank line.
fn meaningful_lines(path: &Path) -> std::io::Result<Vec<(usize, String)>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .enumerate()
        .filter_map(|(i, raw)| {
            let s = raw.trim().to_owned();
            (!s.is_empty()).then_some((i + 1, s))
        })
        .collect())
}

/// Recursively collect `*.rs` files under `dir` in sorted order, skipping
/// directories listed in [`SKIP_DIRS`].
fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    children.sort_unstable();

    let mut files = Vec::new();
    for child in children {
        if child.is_dir() {
            let skip = child
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIP_DIRS.contains(&n));
            if !skip {
                files.extend(collect_rs_files(&child));
            }
        } else if child.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(child);
        }
    }
    files
}

fn main() {
    let src_dir = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "src".into()));
    if !src_dir.is_dir() {
        eprintln!("error: {} is not a directory", src_dir.display());
        process::exit(2);
    }

    // Map block content → list of (path, first_line) occurrences.
    let mut seen: HashMap<String, Vec<(PathBuf, usize)>> = HashMap::new();

    for path in collect_rs_files(&src_dir) {
        let Ok(lines) = meaningful_lines(&path) else {
            continue;
        };
        let n = lines.len();
        for i in 0..n.saturating_sub(MIN_LINES - 1) {
            let block = lines[i..i + MIN_LINES]
                .iter()
                .map(|(_, l)| l.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            seen.entry(block)
                .or_default()
                .push((path.clone(), lines[i].0));
        }
    }

    // Sort for deterministic output.
    let mut duplicates: Vec<(String, Vec<(PathBuf, usize)>)> = seen
        .into_iter()
        .filter(|(_, locs)| locs.len() > 1)
        .collect();
    duplicates.sort_unstable_by(|a, b| {
        a.1[0]
            .0
            .cmp(&b.1[0].0)
            .then_with(|| a.1[0].1.cmp(&b.1[0].1))
    });

    if duplicates.is_empty() {
        println!("Duplication check passed.");
        return;
    }

    for (block, mut locs) in duplicates {
        locs.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        println!(
            "Duplicated block ({MIN_LINES}+ lines) in {} locations:",
            locs.len()
        );
        for (path, line) in &locs {
            println!("  {}:{line}", path.display());
        }
        println!("  ---");
        for line in block.lines() {
            println!("  {line}");
        }
        println!();
    }

    eprintln!("Duplication check FAILED.");
    process::exit(1);
}
