//! Guards the instruction files that agents and new contributors read first.
//!
//! A wrong command in one of these files is worse than no command. `Cargo.toml`'s
//! `[lints]` table is entirely `warn`, so it only has teeth because `cargo lint`
//! (`.cargo/config.toml`) appends `-D warnings`: invoke clippy without it and every
//! finding prints while the process still exits 0. The contributor sees green locally
//! and red in CI, then has to guess which of the two is lying. Re-typing the alias's
//! flags by hand has a slower version of the same failure: it is equivalent only until
//! the flags diverge. Naming a directory as if it were a file costs the same way from
//! the other direction — the reader follows the path and finds nothing.
//!
//! Read through `include_str!` on purpose, matching `tests/lint_policy.rs`: editing any
//! guarded file recompiles this target, so the guard cannot go stale against the text it
//! describes.

/// Repo-relative path and body of every instruction file that tells a reader how to lint.
const LINT_INSTRUCTIONS: &[(&str, &str)] = &[
    ("GEMINI.md", include_str!("../GEMINI.md")),
    ("AGENTS.md", include_str!("../AGENTS.md")),
    (
        ".codex/instructions.md",
        include_str!("../.codex/instructions.md"),
    ),
    (
        ".claude/commands/check.md",
        include_str!("../.claude/commands/check.md"),
    ),
    (
        ".codex/commands/check.md",
        include_str!("../.codex/commands/check.md"),
    ),
    (
        ".claude/agents/check.md",
        include_str!("../.claude/agents/check.md"),
    ),
    (
        ".codex/agents/check.toml",
        include_str!("../.codex/agents/check.toml"),
    ),
];

const CODEX_INSTRUCTIONS: &str = include_str!("../.codex/instructions.md");

/// Whether a line *instructs* the reader to run clippy, as opposed to mentioning it.
///
/// A plain `body.contains("cargo clippy")` cannot tell "run `cargo clippy`" from "not
/// bare `cargo clippy`", so it forbade the one thing these files most need to do:
/// explain the trap. It fired on AGENTS.md for pointing the reader AWAY from clippy.
///
/// The real defect is clippy standing where the command to run belongs — at the start
/// of a line, in a code block or a bullet, after nothing but list punctuation. A
/// mention later in a sentence is prose about the command, not an instruction to run
/// it.
fn invokes_clippy_directly(line: &str) -> bool {
    line.trim_start()
        .trim_start_matches(['-', '*', '>', '`', '#', ' '])
        .starts_with("cargo clippy")
}

#[test]
fn agent_instructions_send_the_reader_to_cargo_lint() {
    for (path, body) in LINT_INSTRUCTIONS {
        let offender = body.lines().find(|line| invokes_clippy_directly(line));
        assert!(
            offender.is_none(),
            "{path} tells the reader to invoke clippy directly ({offender:?}). Without \
             the `-D warnings` that `cargo lint` supplies, the `[lints]` table is \
             advisory and the command exits 0 on findings; hand-copied flags also drift \
             from the alias. Use `cargo lint`. Explaining why bare clippy is wrong is \
             fine — only a line that runs it fails this test."
        );
        assert!(
            body.contains("cargo lint"),
            "{path} explains how to lint but never names `cargo lint`, the only \
             invocation that reproduces CI."
        );
    }
}

#[test]
fn codex_project_structure_paths_exist_as_described() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Only the Project Structure bullets: the other sections quote commands, not paths.
    let section = CODEX_INSTRUCTIONS
        .split("## Project Structure")
        .nth(1)
        .expect(".codex/instructions.md lost its `## Project Structure` heading")
        .split("\n## ")
        .next()
        .unwrap_or_default();

    for line in section
        .lines()
        .filter(|l| l.trim_start().starts_with("- `"))
    {
        let path = line.split('`').nth(1).unwrap_or_default();
        // A brace glob such as `src/app/{cli_mode,runtime}.rs` names several files in one
        // bullet and is not an openable path, so only literal paths can be checked.
        if path.contains('{') {
            continue;
        }
        let full = root.join(path);
        // A trailing `/` promises a directory, anything else promises a file. The rot this
        // catches is the second case: `benches/benchmarks.rs` and `tests/bench_functions.rs`
        // were both split into directories and the text was not updated, so the reader was
        // sent to two paths that cannot be opened.
        if path.ends_with('/') {
            assert!(
                full.is_dir(),
                ".codex/instructions.md names `{path}` as a directory; nothing is there"
            );
        } else {
            assert!(
                full.is_file(),
                ".codex/instructions.md names `{path}` as a file; nothing is there"
            );
        }
    }
}
