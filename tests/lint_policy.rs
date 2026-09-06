//! Guards the lint policy itself, and the one manifest property the rustdoc gate needs.
//!
//! `Cargo.toml`'s `[lints]` table only has teeth because `cargo lint` (defined in
//! `.cargo/config.toml`) appends `-D warnings`. Weaken either half and the whole table
//! silently becomes advisory: the build stays green while nothing is enforced. That is
//! the failure mode worth a test, because it looks exactly like success.
//!
//! Read through `include_str!` on purpose, matching the benchmark-gate guards: editing
//! `Cargo.toml`, `.cargo/config.toml` or `rust-toolchain.toml` recompiles this target,
//! so the check cannot go stale against the files it describes.

const MANIFEST: &str = include_str!("../Cargo.toml");
const CARGO_CONFIG: &str = include_str!("../.cargo/config.toml");
const CLIPPY_CONFIG: &str = include_str!("../clippy.toml");
const TOOLCHAIN: &str = include_str!("../rust-toolchain.toml");

/// Every key/value pair under a header (or the root for `""`), ignoring comments.
///
/// A hand-rolled scan rather than a `toml` dependency: only flat tables and root
/// scalar values are needed, and this crate deliberately keeps its dependency list
/// short (see the `serde_json` note in Cargo.toml).
fn table_entries(source: &str, header: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut inside = header.is_empty();

    for line in source.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Strip any trailing `# comment` so it cannot leak into the value.
        let statement = line.split('#').next().unwrap_or_default().trim();
        let Some((key, value)) = statement.split_once('=') else {
            continue;
        };
        entries.push((
            key.trim().to_owned(),
            value.trim().trim_matches('"').to_owned(),
        ));
    }
    entries
}

fn rust_lints() -> Vec<(String, String)> {
    table_entries(MANIFEST, "[lints.rust]")
}

fn clippy_lints() -> Vec<(String, String)> {
    table_entries(MANIFEST, "[lints.clippy]")
}

#[test]
fn cargo_lint_alias_still_denies_warnings() {
    let alias = CARGO_CONFIG
        .lines()
        .find(|line| line.trim_start().starts_with("lint ="))
        .expect("the `lint` alias is missing from .cargo/config.toml");

    assert!(
        alias.contains("-D warnings"),
        "the `lint` alias no longer passes `-D warnings`: {alias}\nWithout it every entry \
         in Cargo.toml's [lints] table degrades to a warning and CI goes green on \
         violations."
    );
    assert!(
        alias.contains("--lib") && alias.contains("--bins"),
        "the `lint` alias must lint library and binary targets: {alias}"
    );
    assert!(
        alias.contains("--all-features"),
        "the `lint` alias no longer passes `--all-features`: {alias}"
    );
}

#[test]
fn both_lint_tables_are_declared() {
    for header in ["[lints.rust]", "[lints.clippy]"] {
        assert!(
            MANIFEST.contains(header),
            "Cargo.toml no longer declares {header}"
        );
    }
    assert!(
        !rust_lints().is_empty(),
        "[lints.rust] is present but empty — this test would pass vacuously"
    );
    assert!(
        !clippy_lints().is_empty(),
        "[lints.clippy] is present but empty — this test would pass vacuously"
    );
}

#[test]
fn readability_checks_are_explicitly_enabled() {
    let lints = clippy_lints();
    for name in [
        "cognitive_complexity",
        "excessive_nesting",
        "too_many_arguments",
        "type_complexity",
        "allow_attributes_without_reason",
        "fn_params_excessive_bools",
        "undocumented_unsafe_blocks",
        "needless_pass_by_value",
        "doc_markdown",
    ] {
        assert!(
            lints.iter().any(|(lint, level)| lint == name
                && matches!(level.as_str(), "warn" | "deny" | "forbid")),
            "clippy::{name} must be enforced by cargo lint"
        );
    }
}

#[test]
fn readability_budgets_cannot_be_disabled_or_silently_relaxed() {
    let configuration = table_entries(CLIPPY_CONFIG, "");
    for (name, maximum) in [
        ("cognitive-complexity-threshold", 25),
        ("excessive-nesting-threshold", 4),
        ("too-many-arguments-threshold", 5),
        ("type-complexity-threshold", 250),
        ("max-fn-params-bools", 1),
    ] {
        let value = configuration
            .iter()
            .find(|(key, _)| key == name)
            .unwrap_or_else(|| panic!("clippy.toml must explicitly set {name}"))
            .1
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("{name} must be a positive integer"));
        assert!(
            value > 0 && value <= maximum,
            "{name} must be between 1 and {maximum}, got {value}; zero disables nesting"
        );
    }
}

/// The table's premise is that a lint is only added once its violations are fixed, so
/// `allow` entries never accumulate. An `allow` here is either a lint that should have
/// been left out, or a violation that should have been fixed.
#[test]
fn no_lint_is_set_to_allow() {
    for (table, entries) in [("rust", rust_lints()), ("clippy", clippy_lints())] {
        for (lint, level) in entries {
            assert_ne!(
                level, "allow",
                "[lints.{table}] sets `{lint}` to \"allow\". Remove the entry instead — an \
                 allow-list is what this table exists to avoid. If the lint fires and the \
                 violations are not worth fixing, record it in the commented \
                 \"deliberately NOT enabled\" block with its count."
            );
            assert!(
                level == "warn" || level == "deny" || level == "forbid",
                "[lints.{table}] sets `{lint}` to \"{level}\", which is not a lint level"
            );
        }
    }
}

/// clippy.toml's `msrv` is documented as tracking rust-toolchain.toml's `channel`, and
/// nothing else keeps them in step. Drift is silent: clippy simply starts suggesting
/// APIs newer than the toolchain provides, or stops suggesting ones it does.
#[test]
fn clippy_msrv_matches_the_pinned_toolchain() {
    let msrv = CLIPPY_CONFIG
        .lines()
        .find_map(|line| line.trim().strip_prefix("msrv ="))
        .map(|value| value.trim().trim_matches('"').to_owned())
        .expect("clippy.toml no longer sets `msrv`");

    let channel = TOOLCHAIN
        .lines()
        .find_map(|line| line.trim().strip_prefix("channel ="))
        .map(|value| {
            value
                .split('#')
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches('"')
                .to_owned()
        })
        .expect("rust-toolchain.toml no longer sets `channel`");

    assert_eq!(
        msrv, channel,
        "clippy.toml msrv (\"{msrv}\") and rust-toolchain.toml channel (\"{channel}\") have \
         drifted. Update both, or drop the msrv if the pin is gone."
    );
}

/// Keystone lints, each of which had real violations fixed to enable it. Deleting one
/// would quietly re-open the bug class the fixes closed.
#[test]
fn lints_fixed_to_be_enabled_are_still_enabled() {
    let clippy: Vec<String> = clippy_lints().into_iter().map(|(lint, _)| lint).collect();
    for lint in ["or_fun_call", "redundant_clone", "todo", "dbg_macro"] {
        assert!(
            clippy.contains(&lint.to_owned()),
            "clippy::{lint} was removed from [lints.clippy]"
        );
    }

    let rust: Vec<String> = rust_lints().into_iter().map(|(lint, _)| lint).collect();
    for lint in ["trivial_casts", "unreachable_pub"] {
        assert!(
            rust.contains(&lint.to_owned()),
            "{lint} was removed from [lints.rust]"
        );
    }
}

/// Native functions must retain explicit unsafe-operation boundaries even when
/// they are themselves unsafe to call. Unlike the Clippy checks, this compiler
/// lint protects ordinary builds too.
#[test]
fn the_unsafe_fn_lint_stays_enabled() {
    // Flipped from "stays recorded" now that the 101 violations are fixed and the lint
    // is on, which is what the previous version of this test asked for. It has to be
    // asserted rather than trusted: in edition 2021 an `unsafe fn` body is implicitly
    // an unsafe block, so removing this line would let the next unsafe operation be
    // added inside any `unsafe fn` with no inner block and no SAFETY comment — and
    // nothing would fail. The ~60 SAFETY comments across the Win32 layer are only
    // load-bearing while this holds.
    assert_eq!(
        rust_lints()
            .into_iter()
            .find(|(lint, _)| lint == "unsafe_op_in_unsafe_fn")
            .map(|(_, level)| level),
        Some("deny".to_string()),
        "unsafe_op_in_unsafe_fn must stay `deny` in Cargo.toml's [lints.rust]. `warn` is \
         not enough: every other lint here relies on `cargo lint` adding -D warnings, but \
         this one must also bite a bare `cargo build`, because the mistake it catches \
         leaves no other trace."
    );
}
/// `cargo doc` must keep producing the library HTML on a case-insensitive filesystem.
///
/// rustdoc writes each target to `target/doc/<target name>` and clears that directory
/// before writing. The bin target is `RustShot` and the lib target `rustshot`, which are
/// the same directory on Windows and macOS, and the bin is documented second — so it used
/// to delete the entire library tree and leave four files: no `record` module page, no
/// `fn.round_to_even.html`. `doc = false` on `[[bin]]` is the fix.
///
/// This is the failure mode worth a test because it is silent in both directions. Nothing
/// warns, and CI's `Rustdoc warnings` job goes green either way: intra-doc links are
/// resolved while each target compiles, independently of which target's output survives.
/// So the gate that exists cannot notice, and a future rename of either target would
/// re-break the docs with no signal at all.
#[test]
fn bin_target_cannot_clobber_the_library_doc_output() {
    fn value(entries: &[(String, String)], key: &str) -> Option<String> {
        entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    }
    // rustdoc replaces `-` with `_` in the output directory name, so compare the
    // directories rather than the target names: `rustshot-gui` would not in fact collide.
    fn doc_dir(target_name: &str) -> String {
        target_name.replace('-', "_")
    }

    let package = table_entries(MANIFEST, "[package]");
    let bin = table_entries(MANIFEST, "[[bin]]");
    let lib_dir = doc_dir(&value(&package, "name").expect("[package] has no name"));
    let bin_dir = doc_dir(&value(&bin, "name").expect("[[bin]] has no name"));

    let collides = bin_dir.eq_ignore_ascii_case(&lib_dir);
    let bin_docs_disabled = value(&bin, "doc").as_deref() == Some("false");

    assert!(
        !collides || bin_docs_disabled,
        "the bin target writes its rustdoc output to target/doc/{bin_dir}, which is the \
         library's target/doc/{lib_dir} on a case-insensitive filesystem. rustdoc clears \
         that directory before writing and the bin is documented second, so `cargo doc` \
         yields no library pages at all — while still passing, because intra-doc link \
         checking runs either way. Keep `doc = false` on [[bin]], or give the bin a name \
         that does not collide case-insensitively with the lib."
    );
}
