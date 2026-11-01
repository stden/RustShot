//! Detect exact clones of complete Rust statements, with a five-code-line minimum.
//!
//! Comments, punctuation-only lines, item signatures and partial argument lists
//! are not executable blocks. Statement sequences never cross block/item
//! boundaries. Test code and every cfg branch are parsed without exclusions.
//!
//! Usage: cargo run --manifest-path tools/check_duplicates/Cargo.toml -- src
//! Exit codes: 0 = clean, 1 = duplicates, 2 = invalid input or scan failure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::{env, fs, io, process};

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// Keep the original five-line threshold, counting lines containing code tokens.
const MIN_LINES: usize = 5;
const SKIP_DIRS: &[&str] = &["target", ".git"];

struct Statement {
    key: String,
    lines: BTreeSet<usize>,
    start: usize,
    end: usize,
}

struct Block {
    path: PathBuf,
    statements: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Location {
    path: PathBuf,
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct Seed {
    block: usize,
    start: usize,
    end: usize,
}

struct Duplicate {
    locations: Vec<Location>,
    code: Vec<String>,
}

struct Collector<'a> {
    path: &'a Path,
    blocks: &'a mut Vec<Block>,
}

impl Collector<'_> {
    fn push(&mut self, statements: &mut Vec<Statement>) {
        if !statements.is_empty() {
            self.blocks.push(Block {
                path: self.path.to_owned(),
                statements: std::mem::take(statements),
            });
        }
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let mut statements = Vec::new();
        for statement in &block.stmts {
            if matches!(statement, syn::Stmt::Item(_)) {
                // A nested item is a boundary, not part of its parent's logic.
                self.push(&mut statements);
                continue;
            }
            let tokens = statement.to_token_stream();
            let mut lines = BTreeSet::new();
            code_lines(tokens.clone(), &mut lines);
            statements.push(Statement {
                key: tokens.to_string(),
                lines,
                start: statement.span().start().line,
                end: statement.span().end().line,
            });
        }
        self.push(&mut statements);
        visit::visit_block(self, block);
    }
}

/// Count identifier/literal-bearing source lines. Preserve string literals as
/// tokens: comment markers or newlines inside a string are data, not comments.
fn code_lines(tokens: TokenStream, lines: &mut BTreeSet<usize>) {
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Punct(ref punct) if punct.as_char() == '#' => {
                if matches!(tokens.peek(), Some(TokenTree::Punct(p)) if p.as_char() == '!') {
                    tokens.next();
                }
                if matches!(tokens.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket)
                {
                    tokens.next();
                }
            }
            TokenTree::Ident(_) | TokenTree::Literal(_) => {
                lines.insert(token.span().start().line);
            }
            TokenTree::Group(group) => code_lines(group.stream(), lines),
            TokenTree::Punct(_) => {}
        }
    }
}

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_unstable_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if !SKIP_DIRS.iter().any(|name| entry.file_name() == *name) {
                collect_rs_files(&path, files)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn load_blocks(dir: &Path) -> Result<Vec<Block>, String> {
    let mut files = Vec::new();
    collect_rs_files(dir, &mut files).map_err(|error| format!("{}: {error}", dir.display()))?;
    if files.is_empty() {
        return Err(format!("{}: no Rust source files found", dir.display()));
    }
    let mut blocks = Vec::new();
    for path in files {
        let source =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let syntax =
            syn::parse_file(&source).map_err(|error| format!("{}: {error}", path.display()))?;
        Collector {
            path: &path,
            blocks: &mut blocks,
        }
        .visit_file(&syntax);
    }
    Ok(blocks)
}

fn location(blocks: &[Block], seed: Seed) -> Location {
    let block = &blocks[seed.block];
    Location {
        path: block.path.clone(),
        start: block.statements[seed.start].start,
        end: block.statements[seed.end - 1].end,
    }
}

fn find_duplicates(blocks: &[Block]) -> Vec<Duplicate> {
    let mut seen: BTreeMap<Vec<String>, Vec<Seed>> = BTreeMap::new();
    for (block_index, block) in blocks.iter().enumerate() {
        for start in 0..block.statements.len() {
            let mut lines = BTreeSet::new();
            let mut key = Vec::new();
            for (offset, statement) in block.statements[start..].iter().enumerate() {
                lines.extend(statement.lines.iter().copied());
                key.push(statement.key.clone());
                if lines.len() >= MIN_LINES {
                    seen.entry(key).or_default().push(Seed {
                        block: block_index,
                        start,
                        end: start + offset + 1,
                    });
                    break;
                }
            }
        }
    }

    let mut duplicates = Vec::new();
    for mut seeds in seen.into_values().filter(|seeds| seeds.len() > 1) {
        // Expand equal windows to maximal statement sequences before reporting.
        loop {
            let first = seeds[0];
            let Some(next) = blocks[first.block].statements.get(first.end) else {
                break;
            };
            if !seeds.iter().all(|seed| {
                blocks[seed.block]
                    .statements
                    .get(seed.end)
                    .is_some_and(|statement| statement.key == next.key)
            }) {
                break;
            }
            for seed in &mut seeds {
                seed.end += 1;
            }
        }
        let mut locations: Vec<_> = seeds.iter().map(|seed| location(blocks, *seed)).collect();
        locations.sort_unstable();
        locations.dedup();
        // A nested block can be visited through a larger statement too. One
        // source occurrence must never be reported as a clone of itself.
        if locations.len() < 2 {
            continue;
        }
        let first = seeds[0];
        duplicates.push(Duplicate {
            locations,
            code: blocks[first.block].statements[first.start..first.end]
                .iter()
                .map(|statement| statement.key.clone())
                .collect(),
        });
    }

    // Remove a report only if every occurrence is contained in a larger report.
    // Subsets with additional occurrences are retained.
    duplicates.sort_by_key(|duplicate| {
        std::cmp::Reverse(
            duplicate
                .locations
                .iter()
                .map(|location| location.end - location.start + 1)
                .sum::<usize>(),
        )
    });
    let mut maximal: Vec<Duplicate> = Vec::new();
    for duplicate in duplicates {
        let contained = maximal.iter().any(|other| {
            duplicate.locations.iter().all(|location| {
                other.locations.iter().any(|outer| {
                    outer.path == location.path
                        && outer.start <= location.start
                        && outer.end >= location.end
                })
            })
        });
        if !contained {
            maximal.push(duplicate);
        }
    }
    maximal.sort_by(|a, b| a.locations.cmp(&b.locations));
    maximal
}

fn run() -> Result<bool, String> {
    let mut args = env::args_os().skip(1);
    let src_dir = PathBuf::from(args.next().unwrap_or_else(|| "src".into()));
    if args.next().is_some() {
        return Err("usage: check_duplicates [directory]".into());
    }
    let blocks = load_blocks(&src_dir)?;
    let duplicates = find_duplicates(&blocks);
    if duplicates.is_empty() {
        println!("Duplication check passed.");
        return Ok(false);
    }
    for duplicate in duplicates {
        println!(
            "Duplicated block ({MIN_LINES}+ code lines) in {} locations:",
            duplicate.locations.len()
        );
        for location in duplicate.locations {
            println!(
                "  {}:{}-{}",
                location.path.display(),
                location.start,
                location.end
            );
        }
        for statement in duplicate.code {
            println!("  {statement}");
        }
        println!();
    }
    Ok(true)
}

fn main() {
    match run() {
        Ok(false) => {}
        Ok(true) => {
            eprintln!("Duplication check FAILED.");
            process::exit(1);
        }
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(2);
        }
    }
}
