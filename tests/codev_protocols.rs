//! Structural validation of the codev protocol definitions under `codev/protocols/`.
//!
//! `codev/protocol-schema.json` describes the shape of a `protocol.json`, but nothing
//! in the build enforces it, so a protocol could declare a phase that transitions to
//! an id nothing defines, or a gate requiring a check it never declares. Both surface
//! only when someone actually drives the protocol, which may be weeks after the typo
//! landed.
//!
//! These tests read the files at runtime rather than through `include_str!`, so adding
//! a protocol directory is covered without editing this file.
//!
//! The cohesion tests are the ones with teeth: a protocol check that greps an artifact
//! for a literal is coupled to the template that artifact is started from. If a
//! required heading is renamed in one place and not the other, whoever runs the
//! protocol fails a gate on their first commit, having done nothing wrong.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Phase `type` values permitted by `codev/protocol-schema.json`.
const PHASE_TYPES: &[&str] = &["once", "build_verify", "per_plan_phase"];

/// `verify.type` values permitted by the schema (these map to `consult --type`).
const VERIFY_TYPES: &[&str] = &["spec", "plan", "impl", "pr", "phase", "integration"];

/// Consultation backends the schema allows.
const VERIFY_MODELS: &[&str] = &["gemini", "codex", "claude"];

/// A protocol directory paired with its parsed definition.
struct Protocol {
    /// Directory name, e.g. `spir`. The schema requires `name` to match this.
    dir: String,
    dir_path: PathBuf,
    json: Value,
}

impl Protocol {
    fn phases(&self) -> &[Value] {
        self.json["phases"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    fn phase_ids(&self) -> Vec<&str> {
        self.phases()
            .iter()
            .filter_map(|p| p["id"].as_str())
            .collect()
    }
}

fn protocols_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("codev/protocols")
}

/// Every `codev/protocols/*/protocol.json`, parsed.
fn protocols() -> Vec<Protocol> {
    let root = protocols_dir();
    let entries = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", root.display()))
        .map(|e| e.expect("cannot read protocols directory entry"));

    let mut found: Vec<Protocol> = entries
        .filter(|e| e.path().is_dir())
        .filter_map(|entry| {
            let dir_path = entry.path();
            let manifest = dir_path.join("protocol.json");
            // A directory without protocol.json is not a protocol; templates/ and
            // similar siblings must not fail the suite.
            if !manifest.is_file() {
                return None;
            }
            let text = fs::read_to_string(&manifest)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", manifest.display()));
            let json: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
                panic!("{} is not valid JSON: {e}", manifest.display());
            });
            Some(Protocol {
                dir: entry.file_name().to_string_lossy().into_owned(),
                dir_path,
                json,
            })
        })
        .collect();

    found.sort_by(|a, b| a.dir.cmp(&b.dir));
    assert!(
        !found.is_empty(),
        "no protocol.json found under {} — this test would silently pass on an empty set",
        root.display()
    );
    found
}

/// Matches the schema's `^[a-z][a-z0-9-]*$` (protocol names) or, with `underscore`
/// allowed, `^[a-z][a-z0-9_-]*$` (phase ids).
fn is_kebab_ish(s: &str, allow_underscore: bool) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || (allow_underscore && c == '_')
    })
}

/// Extracts the literal from a `grep -q '<literal>'` invocation, if the command is
/// that shape. `grep -o` and regex-bearing commands return `None` — only literal
/// searches can be checked against a template.
fn grep_literal(command: &str) -> Option<&str> {
    let rest = command.split_once("grep -q '")?.1;
    let (literal, _) = rest.split_once('\'')?;
    Some(literal)
}

/// Whether a template actually satisfies a protocol check's literal.
///
/// Plain `contains` is too weak to be worth asserting. The templates explain their own
/// required headings in an HTML comment at the top (``## Units of Work` and ... are
/// enforced by protocol checks``), so a substring search is satisfied by that mention
/// even if the real heading has been deleted — the test passes while anyone starting
/// from the template fails the gate.
///
/// So a literal that names a heading must match a heading *line*. The aidlc checks
/// anchor with `^` for the same reason; SPIR's do not, and are held to the stricter
/// standard here anyway, since a mention in prose satisfying a gate is a bug in either
/// protocol.
fn template_satisfies(template: &str, literal: &str) -> bool {
    let anchored = literal.strip_prefix('^');
    let needle = anchored.unwrap_or(literal);

    if anchored.is_some() || needle.starts_with("## ") {
        return template.lines().any(|line| line.starts_with(needle));
    }
    // Not a heading (e.g. the `"phases":` key, which is indented inside a fenced JSON
    // block) — position carries no meaning, so a substring is the honest check.
    template.contains(needle)
}

/// Maps the artifact path mentioned in a check command to the template a user starts
/// that artifact from.
fn template_for_command(command: &str) -> Option<&'static str> {
    if command.contains("codev/specs/") {
        Some("spec.md")
    } else if command.contains("codev/plans/") {
        Some("plan.md")
    } else if command.contains("codev/reviews/") {
        Some("review.md")
    } else {
        None
    }
}

/// Every `(check_name, command)` pair declared by any phase of a protocol.
fn checks(protocol: &Protocol) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for phase in protocol.phases() {
        let Some(map) = phase["checks"].as_object() else {
            continue;
        };
        for (name, check) in map {
            // The schema allows a bare command string or an object with `command`.
            let command = check
                .as_str()
                .or_else(|| check["command"].as_str())
                .unwrap_or_else(|| {
                    panic!(
                        "{}: check `{name}` has neither a string command nor a `command` field",
                        protocol.dir
                    )
                });
            out.push((name.clone(), command.to_string()));
        }
    }
    out
}

#[test]
fn both_shipped_protocols_are_present() {
    let names: HashSet<String> = protocols().into_iter().map(|p| p.dir).collect();
    for expected in ["spir", "aidlc"] {
        assert!(
            names.contains(expected),
            "protocol `{expected}` is missing from codev/protocols/ (found: {names:?})"
        );
    }
}

#[test]
fn protocol_name_matches_its_directory() {
    for p in protocols() {
        let name = p.json["name"]
            .as_str()
            .unwrap_or_else(|| panic!("{}: `name` is required and must be a string", p.dir));
        assert_eq!(
            name, p.dir,
            "{}: `name` is \"{name}\" but the directory is \"{}\" — porch resolves \
             protocols by directory, so these must agree",
            p.dir, p.dir
        );
        assert!(
            is_kebab_ish(name, false),
            "{}: `name` \"{name}\" violates the schema pattern ^[a-z][a-z0-9-]*$",
            p.dir
        );
    }
}

#[test]
fn version_is_semver_shaped() {
    for p in protocols() {
        let Some(version) = p.json["version"].as_str() else {
            continue; // `version` is optional in the schema.
        };
        let core = version.split('-').next().unwrap_or_default();
        let parts: Vec<&str> = core.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "{}: `version` \"{version}\" is not MAJOR.MINOR.PATCH",
            p.dir
        );
        for part in parts {
            assert!(
                !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()),
                "{}: `version` \"{version}\" has a non-numeric component",
                p.dir
            );
        }
    }
}

#[test]
fn phase_ids_are_unique_and_well_formed() {
    for p in protocols() {
        assert!(
            !p.phases().is_empty(),
            "{}: `phases` is required and must hold at least one phase",
            p.dir
        );
        let mut seen = HashSet::new();
        for phase in p.phases() {
            let id = phase["id"]
                .as_str()
                .unwrap_or_else(|| panic!("{}: every phase needs a string `id`", p.dir));
            assert!(
                is_kebab_ish(id, true),
                "{}: phase id \"{id}\" violates the schema pattern ^[a-z][a-z0-9_-]*$",
                p.dir
            );
            assert!(
                phase["name"].is_string(),
                "{}: phase \"{id}\" needs a human-readable `name`",
                p.dir
            );
            assert!(
                seen.insert(id),
                "{}: phase id \"{id}\" is declared twice — transitions would be ambiguous",
                p.dir
            );
        }
    }
}

#[test]
fn phase_types_and_iteration_limits_are_valid() {
    for p in protocols() {
        for phase in p.phases() {
            let id = phase["id"].as_str().unwrap_or("<unnamed>");
            if let Some(ty) = phase["type"].as_str() {
                assert!(
                    PHASE_TYPES.contains(&ty),
                    "{}: phase \"{id}\" has type \"{ty}\", not one of {PHASE_TYPES:?}",
                    p.dir
                );
            }
            if let Some(max) = phase["max_iterations"].as_u64() {
                assert!(
                    (1..=20).contains(&max),
                    "{}: phase \"{id}\" has max_iterations {max}, outside the schema's 1..=20",
                    p.dir
                );
            }
        }
    }
}

#[test]
fn verify_blocks_use_known_types_and_models() {
    for p in protocols() {
        for phase in p.phases() {
            let id = phase["id"].as_str().unwrap_or("<unnamed>");
            let verify = &phase["verify"];
            if verify.is_null() {
                continue;
            }
            let ty = verify["type"].as_str().unwrap_or_else(|| {
                panic!(
                    "{}: phase \"{id}\" has a `verify` block without `type`",
                    p.dir
                )
            });
            assert!(
                VERIFY_TYPES.contains(&ty),
                "{}: phase \"{id}\" verifies with type \"{ty}\", not one of {VERIFY_TYPES:?}",
                p.dir
            );
            let models = verify["models"].as_array().unwrap_or_else(|| {
                panic!(
                    "{}: phase \"{id}\" has a `verify` block without `models`",
                    p.dir
                )
            });
            assert!(
                !models.is_empty(),
                "{}: phase \"{id}\" verifies with an empty model list",
                p.dir
            );
            for model in models {
                let model = model.as_str().unwrap_or_default();
                assert!(
                    VERIFY_MODELS.contains(&model),
                    "{}: phase \"{id}\" consults \"{model}\", not one of {VERIFY_MODELS:?}",
                    p.dir
                );
            }
        }
    }
}

#[test]
fn phase_next_pointers_resolve() {
    for p in protocols() {
        let ids = p.phase_ids();
        for phase in p.phases() {
            let id = phase["id"].as_str().unwrap_or("<unnamed>");
            // `next` is either a phase id or explicit null for a terminal phase.
            // A *missing* next is not the same as null, and per_plan_phase uses
            // `transition` instead, so only validate when a string is present.
            if let Some(next) = phase["next"].as_str() {
                assert!(
                    ids.contains(&next),
                    "{}: phase \"{id}\" transitions to \"{next}\", which no phase declares \
                     (declared: {ids:?})",
                    p.dir
                );
            }
        }
    }
}

#[test]
fn exactly_one_phase_is_terminal() {
    for p in protocols() {
        let terminal: Vec<&str> = p
            .phases()
            .iter()
            .filter(|phase| phase.get("next").is_some_and(Value::is_null))
            .filter_map(|phase| phase["id"].as_str())
            .collect();
        assert_eq!(
            terminal.len(),
            1,
            "{}: expected exactly one terminal phase (\"next\": null), found {terminal:?} — \
             zero means the protocol never finishes, more than one is ambiguous",
            p.dir
        );
    }
}

#[test]
fn per_plan_phase_transitions_resolve() {
    for p in protocols() {
        let ids = p.phase_ids();
        for phase in p.phases() {
            assert_per_plan_phase_transitions(&p, phase, &ids);
        }
    }
}

fn assert_per_plan_phase_transitions(p: &Protocol, phase: &Value, ids: &[&str]) {
    let id = phase["id"].as_str().unwrap_or("<unnamed>");
    if phase["type"].as_str() != Some("per_plan_phase") {
        return;
    }
    let transition = &phase["transition"];
    assert!(
        transition.is_object(),
        "{}: phase \"{id}\" is per_plan_phase but declares no `transition`, so the \
         engine cannot tell where to go after each plan phase",
        p.dir
    );
    for key in ["on_complete", "on_all_phases_complete"] {
        let target = transition[key]
            .as_str()
            .unwrap_or_else(|| panic!("{}: phase \"{id}\" is missing transition.{key}", p.dir));
        assert!(
            ids.contains(&target),
            "{}: phase \"{id}\" transition.{key} points at \"{target}\", which no \
             phase declares (declared: {ids:?})",
            p.dir
        );
    }
}

#[test]
fn gate_requires_reference_checks_declared_on_the_same_phase() {
    for p in protocols() {
        for phase in p.phases() {
            let id = phase["id"].as_str().unwrap_or("<unnamed>");
            let gate = &phase["gate"];
            // The simple gate form is a bare string with no `requires`.
            let Some(requires) = gate["requires"].as_array() else {
                continue;
            };
            let declared: HashSet<&str> = phase["checks"]
                .as_object()
                .map(|m| m.keys().map(String::as_str).collect())
                .unwrap_or_default();
            for required in requires {
                let required = required.as_str().unwrap_or_default();
                assert!(
                    declared.contains(required),
                    "{}: gate on phase \"{id}\" requires check \"{required}\", which the phase \
                     does not declare (declared: {declared:?}) — the gate could never be \
                     satisfied",
                    p.dir
                );
            }
        }
    }
}

#[test]
fn gate_next_pointers_resolve() {
    for p in protocols() {
        let ids = p.phase_ids();
        for phase in p.phases() {
            let id = phase["id"].as_str().unwrap_or("<unnamed>");
            let gate = &phase["gate"];
            if let Some(next) = gate["next"].as_str() {
                assert!(
                    ids.contains(&next),
                    "{}: the gate on phase \"{id}\" continues to \"{next}\", which no phase \
                     declares (declared: {ids:?})",
                    p.dir
                );
            }
        }
    }
}

#[test]
fn checks_that_grep_an_artifact_match_its_template() {
    let mut asserted = 0usize;
    for p in protocols() {
        for (name, command) in checks(&p) {
            let Some(literal) = grep_literal(&command) else {
                continue;
            };
            let Some(template_name) = template_for_command(&command) else {
                continue;
            };
            let template_path = p.dir_path.join("templates").join(template_name);
            assert!(
                template_path.is_file(),
                "{}: check `{name}` greps a {} artifact, but {} does not exist",
                p.dir,
                template_name.trim_end_matches(".md"),
                template_path.display()
            );
            let template = fs::read_to_string(&template_path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", template_path.display()));
            assert!(
                template_satisfies(&template, literal),
                "{}: check `{name}` requires \"{literal}\" in the artifact, but {} has no line \
                 matching it. Anyone starting from the template fails this gate immediately; \
                 rename it in both places or not at all. (A mention inside a comment does not \
                 count — see template_satisfies.)",
                p.dir,
                template_path.display()
            );
            asserted += 1;
        }
    }
    assert!(
        asserted > 0,
        "no literal grep checks were cross-referenced against templates — the parser in \
         grep_literal() has probably stopped matching the commands in protocol.json"
    );
}

#[test]
fn every_artifact_producing_phase_has_a_template() {
    for p in protocols() {
        for phase in p.phases() {
            let id = phase["id"].as_str().unwrap_or("<unnamed>");
            let Some(artifact) = phase["build"]["artifact"].as_str() else {
                continue;
            };
            // Source-code artifacts (src/**/*.rs) have no template; only the three
            // codev document kinds do.
            let Some(template_name) = template_for_command(artifact) else {
                continue;
            };
            let template_path = p.dir_path.join("templates").join(template_name);
            assert!(
                template_path.is_file(),
                "{}: phase \"{id}\" produces {artifact} but {} is missing",
                p.dir,
                template_path.display()
            );
        }
    }
}

#[test]
fn build_blocks_declare_both_prompt_and_artifact() {
    for p in protocols() {
        for phase in p.phases() {
            let id = phase["id"].as_str().unwrap_or("<unnamed>");
            let build = &phase["build"];
            if build.is_null() {
                continue;
            }
            for key in ["prompt", "artifact"] {
                assert!(
                    build[key].is_string(),
                    "{}: phase \"{id}\" has a `build` block without `{key}` (both are required \
                     by the schema)",
                    p.dir
                );
            }
        }
    }
}

/// SPIR's `protocol.json` carries `npm run build` and `npm test`, inherited from the
/// JavaScript project it came from. Those commands do not run here, so a protocol
/// authored for `RustShot` must not copy them.
#[test]
fn aidlc_checks_use_cargo_not_npm() {
    let protocols = protocols();
    let aidlc = protocols
        .iter()
        .find(|p| p.dir == "aidlc")
        .expect("the aidlc protocol is missing");

    let mut cargo_checks = 0usize;
    for (name, command) in checks(aidlc) {
        assert!(
            !command.contains("npm"),
            "aidlc: check `{name}` runs \"{command}\" — npm was copied from SPIR and does not \
             run in this Rust project"
        );
        if command.contains("cargo ") {
            cargo_checks += 1;
        }
    }
    assert!(
        cargo_checks >= 4,
        "aidlc: expected the Construction and Operations phases to gate on cargo (compile, \
         test, fmt, clippy, release build), found only {cargo_checks} cargo checks"
    );

    for (name, command) in aidlc.json["phase_completion"]
        .as_object()
        .expect("aidlc: `phase_completion` is missing")
    {
        assert!(
            !command.as_str().unwrap_or_default().contains("npm"),
            "aidlc: phase_completion.{name} runs npm, which does not run in this Rust project"
        );
    }
}

/// The `commit_has_code` completion check exists to stop a phase being marked done by
/// a docs-only commit. Keyed on the wrong extension it silently never fires.
#[test]
fn aidlc_commit_check_looks_for_rust_files() {
    let protocols = protocols();
    let aidlc = protocols
        .iter()
        .find(|p| p.dir == "aidlc")
        .expect("the aidlc protocol is missing");
    let check = aidlc.json["phase_completion"]["commit_has_code"]
        .as_str()
        .expect("aidlc: phase_completion.commit_has_code is missing");
    assert!(
        check.contains(".rs"),
        "aidlc: commit_has_code is \"{check}\", which does not look for .rs files — it would \
         never detect a code-free phase in this project"
    );
}

/// AIDLC splits the methodology's Construction phase into `design` and `construction`
/// because the engine needs a plan artifact before it can iterate units of work. The
/// three published phase names must all still be reachable.
#[test]
fn aidlc_covers_the_three_published_phases() {
    let protocols = protocols();
    let aidlc = protocols
        .iter()
        .find(|p| p.dir == "aidlc")
        .expect("the aidlc protocol is missing");
    let ids = aidlc.phase_ids();
    for expected in ["inception", "design", "construction", "operations"] {
        assert!(
            ids.contains(&expected),
            "aidlc: phase \"{expected}\" is missing (declared: {ids:?})"
        );
    }

    // Inception must reach Operations by following transitions, or a bolt can never
    // ship. `construction` is per_plan_phase, so its exit is transition-based.
    let phase = |id: &str| {
        aidlc
            .phases()
            .iter()
            .find(|p| p["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("aidlc: phase \"{id}\" is missing"))
            .clone()
    };
    assert_eq!(phase("inception")["next"].as_str(), Some("design"));
    assert_eq!(phase("design")["next"].as_str(), Some("construction"));
    assert_eq!(
        phase("construction")["transition"]["on_all_phases_complete"].as_str(),
        Some("operations"),
        "aidlc: construction must hand off to operations once every unit of work is built"
    );
    assert!(
        phase("operations")["next"].is_null(),
        "aidlc: operations must be the terminal phase"
    );
}

/// The documentation and the definition drift apart easily, and `protocol.md` is what
/// a human reads before driving the protocol.
#[test]
fn aidlc_documentation_covers_every_phase_and_signal() {
    let doc_path = protocols_dir().join("aidlc/protocol.md");
    let doc = fs::read_to_string(&doc_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", doc_path.display()));

    let protocols = protocols();
    let aidlc = protocols
        .iter()
        .find(|p| p.dir == "aidlc")
        .expect("the aidlc protocol is missing");

    for id in aidlc.phase_ids() {
        assert!(
            doc.contains(id),
            "aidlc/protocol.md does not mention phase \"{id}\""
        );
    }
    for signal in aidlc.json["signals"]
        .as_object()
        .expect("aidlc: `signals` is missing")
        .keys()
    {
        assert!(
            doc.contains(signal),
            "aidlc/protocol.md does not document signal \"{signal}\""
        );
    }
}

// ---------------------------------------------------------------------------
// Verification of the documents as documents: does starting from a template
// actually satisfy the gates, and are the two protocols really incompatible?
// ---------------------------------------------------------------------------

/// Every heading literal a protocol's checks require, paired with the template the
/// artifact is started from.
fn required_headings(protocol: &Protocol) -> Vec<(&'static str, String)> {
    checks(protocol)
        .into_iter()
        .filter_map(|(_, command)| {
            let literal = grep_literal(&command)?;
            let template = template_for_command(&command)?;
            // Only headings; `"phases":` is a JSON key both protocols share.
            let needle = literal.strip_prefix('^').unwrap_or(literal);
            needle
                .starts_with("## ")
                .then(|| (template, literal.to_owned()))
        })
        .collect()
}

/// `template_satisfies` exists because a plain substring search was satisfied by the
/// templates' own explanatory comments, which name their required headings in
/// backticks. That hole let a template with the real heading deleted pass the test
/// while failing the gate. Unit-tested directly so the fix cannot silently regress.
#[test]
fn a_heading_named_only_in_a_comment_does_not_satisfy_a_gate() {
    let comment_only = "# Inception: Test\n\n<!-- `## Units of Work` is enforced -->\n";
    assert!(
        !template_satisfies(comment_only, "^## Units of Work"),
        "an anchored heading requirement was satisfied by a mention inside a comment"
    );
    assert!(
        !template_satisfies(comment_only, "## Units of Work"),
        "an unanchored heading requirement was satisfied by a mention inside a comment"
    );
    // The real heading, on its own line, must still count.
    let real = "# Inception: Test\n\n## Units of Work\n\n### U1: Thing\n";
    assert!(
        template_satisfies(real, "^## Units of Work"),
        "a genuine heading line failed to satisfy its own requirement"
    );
    // Non-heading literals stay substring matches: the plan template's `"phases":`
    // key is indented inside a fenced JSON block, so position carries no meaning.
    assert!(
        template_satisfies("  \"phases\": [\n", "\"phases\":"),
        "an indented JSON key failed a substring requirement"
    );
}

/// AIDLC's heading checks must stay anchored with `^`. Unanchored, `grep -q` matches
/// the artifact's own instructional comments, so an artifact that never grew the
/// section still passes the gate — demonstrated by SPIR's unanchored equivalents.
#[test]
fn aidlc_heading_checks_are_anchored_to_line_start() {
    let protocols = protocols();
    let aidlc = protocols
        .iter()
        .find(|p| p.dir == "aidlc")
        .expect("the aidlc protocol is missing");

    let mut checked = 0usize;
    for (name, command) in checks(aidlc) {
        let Some(literal) = grep_literal(&command) else {
            continue;
        };
        if !literal.starts_with("^## ") && !literal.starts_with("## ") {
            continue;
        }
        assert!(
            literal.starts_with('^'),
            "aidlc: check `{name}` greps \"{literal}\" without a `^` anchor. Unanchored, a \
             heading named in the template's own HTML comment satisfies the gate, so an \
             artifact missing the section passes."
        );
        checked += 1;
    }
    assert!(
        checked >= 5,
        "expected at least 5 anchored heading checks in aidlc, found {checked} — the parser \
         has probably stopped matching protocol.json"
    );
}

/// Starting an artifact from a protocol's own template must satisfy every heading gate
/// that protocol declares for it. This is the claim a user relies on; the per-check
/// test above verifies it one at a time, this asserts the set is complete per template.
#[test]
fn each_template_satisfies_every_heading_gate_for_its_artifact() {
    for p in protocols() {
        let required = required_headings(&p);
        assert!(
            !required.is_empty(),
            "{}: no heading gates found, so this test proves nothing for it",
            p.dir
        );
        for (template_name, literal) in required {
            let path = p.dir_path.join("templates").join(template_name);
            let template = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            assert!(
                template_satisfies(&template, &literal),
                "{}: {} does not satisfy its own gate \"{literal}\"",
                p.dir,
                path.display()
            );
        }
    }
}

/// `aidlc/protocol.md` warns that the protocols must not be mixed inside one project:
/// they share the `codev/specs`, `codev/plans` and `codev/reviews` paths but enforce
/// different sections. That warning is only worth writing if switching is actually
/// detected, so assert the templates are mutually non-conforming.
///
/// Concretely: this is what happens today if someone "uses AIDLC" but runs the
/// `/spec` command, which hardcodes SPIR's template.
#[test]
fn protocols_do_not_silently_accept_each_others_artifacts() {
    let protocols = protocols();
    assert!(
        protocols.len() >= 2,
        "fewer than two protocols; nothing to cross-check"
    );

    for a in &protocols {
        for b in &protocols {
            if a.dir == b.dir {
                continue;
            }
            assert_foreign_protocol_artifacts_rejected(a, b);
        }
    }
}

fn assert_foreign_protocol_artifacts_rejected(a: &Protocol, b: &Protocol) {
    let required = required_headings(a);
    if required.is_empty() {
        return;
    }
    // Grouped PER TEMPLATE, not across the whole protocol. Phases gate their
    // own artifact independently, so "some gate somewhere rejects" is not
    // enough: if A's review gates all pass against B's review template, a
    // project that switched protocols sails through the Operations gate even
    // though its spec would have been caught earlier.
    let mut templates: Vec<&'static str> = required.iter().map(|(template, _)| *template).collect();
    templates.sort_unstable();
    templates.dedup();

    for template_name in templates {
        let path = b.dir_path.join("templates").join(template_name);
        // A missing template is itself a rejection.
        let Ok(template) = fs::read_to_string(&path) else {
            continue;
        };
        let gates: Vec<&String> = required
            .iter()
            .filter(|(name, _)| *name == template_name)
            .map(|(_, literal)| literal)
            .collect();
        let rejected = gates
            .iter()
            .filter(|literal| !template_satisfies(&template, literal))
            .count();
        assert!(
            rejected > 0,
            "every `{}` gate on the {} artifact ({gates:?}) is satisfied by `{}`'s \
             {template_name}, so a project that switched from `{}` to `{}` would clear \
             that phase's gate unnoticed. Either the protocols have converged and the \
             \"do not mix\" warning in aidlc/protocol.md is stale, or a required \
             section was added to the wrong template.",
            a.dir,
            template_name.trim_end_matches(".md"),
            b.dir,
            b.dir,
            a.dir
        );
    }
}
