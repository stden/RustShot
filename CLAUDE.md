# CLAUDE.md

RustShot project rules. Loaded every session, so: facts that change behaviour, no
prose. `AGENTS.md` points other agents here.

## Traps

Each has already cost a wasted edit or measurement cycle.

- **Files are CRLF.** A multi-line search/replace or `git apply` patch quoted with
  LF matches **zero** times, silently — normalise first. `src/tray/tests/menu_and_tray.rs`
  is *mixed*, where no single convention matches and the edit must be done by hand.
- **Bin target is `RustShot`, lib crate is `rustshot`.** So `target/release/rustshot`
  does not exist and fails outright off Windows. The bin's `doc = false` is
  load-bearing: without it both write to `target/doc/rustshot` on a case-insensitive
  filesystem and `cargo doc` silently emits no library docs. Renaming the target
  breaks `build.rs` manifest embedding, `installer.iss`, `release.yml`,
  `RustShot.cmd` and the auto-start registry value.
- **GUI-subsystem binary, no console.** In PowerShell a CLI run prints nothing and
  leaves `$LASTEXITCODE` empty. Pipe to `Out-String`, or use `cmd.exe`.
- **Never hardcode a test count in docs.** One was wrong by 3x; its replacement went
  stale within the hour. Describe coverage; let `cargo test` count.

Two more trap sets are path-scoped in `.claude/rules/`, so they load only when
relevant rather than every session: `benchmarks.md` (±25% measurement noise, and the
benchmark/budget parity the suite enforces) and `testing.md` (live capture needs a
*changing* desktop; being honest about what was red first).

## Commands

<!-- The standard cargo verbs are omitted on purpose: check/test/fmt/build are
     derivable and cost context to restate. Only the non-obvious one is listed. -->

**`cargo lint`** is how CI lints — alias for
`clippy --all-targets --all-features -- -D warnings`. Bare `cargo clippy` prints the
same findings and **exits 0**, because `[lints]` in `Cargo.toml` is `warn`: green
locally, red in CI. `cargo test` includes the Gherkin features via `tests/bdd.rs`;
`cargo bench --bench performance_gate` is the CI-gated target.

## Rules

- English in comments, commits, reviews, discussion.
- Small changes; no large refactor for a small task; don't break behaviour without a
  reason.
- `Result<_, String>`, human-readable, never silently swallowed unless documented.
- Comments explain *why* — the failure mode avoided, the measurement behind it.
  Match surrounding density.
- Commits: `[+]` add · `[-]` remove · `[!]` fix/refactor.
- Test-first discipline and what counts as honest red-first evidence: see
  `.claude/rules/testing.md`, which loads when you open a test.

## Non-obvious facts about the layout

<!-- The module map itself is deliberately absent: Claude derives directory layout
     from the codebase faster than it can read a table, and a stale table is worse
     than none. Only what reading src/ does not reveal belongs here. -->

- `src/main.rs` is a 33-line launcher. Routing lives in `src/app.rs` and
  `src/app/{cli_mode,tray_app,runtime,autorun}.rs`.
- `src/settings_dialog.rs` is the **only** writer of `settings.json`, and the file
  does not exist until the dialog saves — until then `Settings::default()` supplies
  everything, including `auto_start: true`.
- Recording and active-window capture are Windows-only. Linux additionally needs GTK
  and `libayatana-appindicator3` **at runtime**, which the tray `dlopen`s: without it
  the process panics rather than returning an error.
- Any capture-related flag (`--cli`, `--list`, `--window`, `--output`, `--screen`,
  explicit `--format`, `--list-windows`, `--window-id`) puts the binary in one-shot
  CLI mode; `--mcp` beats all of them; only a bare command line reaches tray mode.

## Documentation

- **Rationale is rustdoc beside the code** — start at `src/lib.rs`. Never add a
  separate architecture document.
- **Behaviour is executable**: Gherkin in `tests/features/` *is* the spec.
- Users: `README.md` (tutorial part 1) → `docs/TUTORIAL.md`. Map: `docs/README.md`.
  Generic Rust/testing guidance lives in the `iopm` repo, not here.

## Codev

SPIR active, driven by `/spec`, `/plan`, `/review`. AIDLC is defined but **has no
driver commands, so it cannot be run**. Treat `codev/` as scaffolding, not a record —
don't assume a task needs an artifact there. Why, and what would have to change:
`codev/protocols/aidlc/protocol.md`.
