# RustShot Documentation Map

This index describes where each kind of documentation lives after the repository
was consolidated. Start here to find the right place to read or add docs.

## Onboarding & agent instructions (repo root)

| File | Purpose |
|------|---------|
| [`README.md`](../README.md) / [`README.ru.md`](../README.ru.md) | Project overview and usage. |
| [`CLAUDE.md`](../CLAUDE.md) / [`CLAUDE.ru.md`](../CLAUDE.ru.md) | Instructions for Claude, incl. the active Codev/SPIR protocol. |
| [`AGENTS.md`](../AGENTS.md) | Generic agent guidance (tool-agnostic). |
| [`GEMINI.md`](../GEMINI.md) | Gemini-specific guidance and the LLM wiki workflow. |
| [`TESTING_TDD_GUIDE.md`](../TESTING_TDD_GUIDE.md), [`TESTING_CHECKLIST_RUST.md`](../TESTING_CHECKLIST_RUST.md) | TDD guide and manual test checklist. |
| [`IDEAS.md`](../IDEAS.md), [`HEARTBEAT.md`](../HEARTBEAT.md), [`MEMORY.md`](../MEMORY.md) | Backlog, status log, and agent-memory index. |

## Specifications, plans & methodology — `codev/` (single source of truth)

RustShot uses the **Codev / SPIR** protocol. All specs and plans live here.

| Path | Contents |
|------|----------|
| [`codev/specs/`](../codev/specs/) | Numbered specifications (the canonical spec home). |
| [`codev/plans/`](../codev/plans/) | Implementation plans (incl. the former `docs/plans/`). |
| [`codev/reviews/`](../codev/reviews/) | Post-implementation reviews. |
| [`codev/protocols/spir/`](../codev/protocols/spir/) | The active protocol (Spec → Plan → Implement → Review). |
| [`codev/roles/`](../codev/roles/), [`codev/resources/`](../codev/resources/), [`codev/templates/`](../codev/templates/) | Role definitions, reference material, and templates. |
| [`codev/legacy/kiro/`](../codev/legacy/kiro/) | Archived Kiro specs/steering, migrated from the retired `.kiro/`. |

> Unused Codev protocol families (air, aspir, bugfix, experiment, maintain,
> spike, tick) were removed; only `spir` (the active protocol) is kept.

## Technical documentation — `docs/`

| File | Topic |
|------|-------|
| [`INSTALLERS.md`](INSTALLERS.md) / [`INSTALLERS.ru.md`](INSTALLERS.ru.md) | Installer overview. |
| [`INNO_SETUP_PIPELINE.md`](INNO_SETUP_PIPELINE.md) / [`.ru.md`](INNO_SETUP_PIPELINE.ru.md) | Inno Setup packaging pipeline. |
| [`github-pipeline.md`](github-pipeline.md) | GitHub/CI pipeline notes. |
| [`rust-best-practices.md`](rust-best-practices.md), [`rust-installers.md`](rust-installers.md) | Rust practices and installer research. |

## Long-form essays — `articles/`

RU essays on the engineering themes behind RustShot, indexed by
[`ARTICLES.ru.md`](../ARTICLES.ru.md). These are the canonical source texts the
wiki summarizes.

## LLM knowledge base — `wiki/`

[`wiki/`](../wiki/) is the retrieval layer over [`articles/`](../articles/), project docs,
and code. [`wiki/pages/`](../wiki/pages/) holds concept, entity, and source pages with
stable metadata IDs, tags, canonical `source_paths`, and `related` page IDs;
[`wiki/index.md`](../wiki/index.md) catalogs them. `python wiki/lint.py` rejects
missing sources, broken local links, duplicate IDs, and pages absent from the
retrieval index; CI runs the same validation.

## Tooling

| Path | Purpose |
|------|---------|
| [`.claude/`](../.claude/), [`.codex/`](../.codex/) | Per-tool agent and slash-command definitions (kept separate; different tools read different directories). |
| [`mcp-server/`](../mcp-server/) | MCP server integration. |
| [`memory/`](../memory/) | Agent memory store. |
