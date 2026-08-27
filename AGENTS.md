# RustShot — AI agent instructions

> For OpenAI Codex, Cursor, GitHub Copilot and any other coding agent.

**Read [`CLAUDE.md`](CLAUDE.md). It is the whole of the project's agent instructions and
it applies to you, not just to Claude Code.**

This file is deliberately a pointer rather than a copy. It used to restate CLAUDE.md's
rules, layout and commands inline, and the two drifted: this file said "do not change
behavior" where CLAUDE.md said "do not break behaviour", and it never grew CLAUDE.md's
`## Traps` section — facts that have each already cost a wasted edit or measurement
cycle, starting with the files being CRLF. An agent following the copy did the wrong
thing while believing it was following the rules. One file cannot go stale against
itself.

What lives where, once you are in CLAUDE.md:

- `## Traps` — read first. Non-obvious failure modes, each with the reason.
- `## Commands` — in particular `cargo lint`, the only lint invocation that reproduces
  CI. Invoking clippy directly exits 0 on findings, so it reads green locally and fails
  in CI.
- `## Rules`, `## Non-obvious facts about the layout`, `## Documentation`, `## Codev`.

Two further trap sets live in `.claude/rules/`: `benchmarks.md` and `testing.md`. Claude
Code loads those on demand when a matching file is opened, which is why they are not in
CLAUDE.md. If your tool does not support path-scoped rules, read them directly before
benchmarking or writing tests.

Two things CLAUDE.md does not spell out, because they are environment setup rather than
working rules:

- **Linux build dependencies:** `libxcb1-dev libxcb-render0-dev libxcb-shape0-dev
  libxcb-xfixes0-dev`, plus GTK and `libayatana-appindicator3` at runtime for the tray.
- **Dependency roles** are documented where they are declared — `Cargo.toml` carries a
  comment on every non-obvious choice, including why settings parsing is deliberately
  serde-free while `src/mcp.rs` uses `serde_json`. Read the manifest rather than a list
  here that would drift from it.
