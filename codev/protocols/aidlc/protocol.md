# AIDLC Protocol

> **AIDLC** = **AI D**evelopment **L**ife **C**ycle
>
> **Inception** → **Construction** → **Operations**, worked in *bolts* rather than sprints.

## ⚠️ This protocol cannot currently be run

The definition below is complete and `tests/codev_protocols.rs` validates it, but
**nothing can drive it**. Verified, not assumed:

- There is no `/inception`, `/design`, `/construction` or `/operations` entry point.
  Grepping `.claude/` and `.codex/` for those names, or for `aidlc`, returns nothing.
- The four `build.prompt` values below (`inception.md` and friends) name files that
  exist nowhere in the repo. SPIR has the same property — its prompts ship with the
  external `codev` tooling — but AIDLC exists only here, so no package can supply them.
- The commands that *do* exist hardcode SPIR's templates:
  `.claude/commands/spec.md` and `.claude/agents/spec.md` both read
  `codev/protocols/spir/templates/spec.md`.

So "picking AIDLC" and then running `/spec` produces a SPIR-shaped document that fails
**every** AIDLC document gate. The mismatch is in heading *names*, not content — SPIR's
spec template carries `## Clarifying Questions Asked`, which holds exactly what
`## Clarifications` wants, and `## Solution Approaches` where `## Units of Work` is
required. The information is right there and the gate still rejects it.

**To make this runnable:** add `/inception`, `/design`, `/construction`, `/operations`
commands and agents mirroring the SPIR four but pointing at
`codev/protocols/aidlc/templates/`, and a test asserting every protocol has a driver —
the test that would have caught this.

Also note `codev/specs/` and `codev/reviews/` contain only `.gitkeep`, and the three
files in `codev/plans/` predate SPIR's `"phases":` check, so they would fail the
plan-approval gate meant to admit them. The directory is scaffolding, not a record of
how this project actually works; that is rustdoc, `tests/features/`, and the commit log.

> **Quick Reference**: See `CLAUDE.md` for the commands and where documentation lives.

## Overview

AIDLC inverts the default authorship of SPIR. In SPIR a human drives and consults
models at checkpoints. In AIDLC **AI proposes and the team validates** — AI writes the
requirements, the logical design, the code and the tests, and stops to ask when a
decision is genuinely the team's to make. The slogan is *AI-powered execution with
human oversight*.

The unit of scheduling is a **bolt**: a work cycle measured in hours or days, not
weeks. A bolt is scoped from exactly one *unit of work* identified during Inception.

### The three phases

| Phase | Practice | AI does | The team does | Artifact |
|---|---|---|---|---|
| **Inception** | Mob Elaboration | Asks clarifying questions, turns intent into requirements and units of work | Validates that the units match the business intent | `codev/specs/<project>.md` |
| **Construction** | Mob Construction | Proposes logical architecture and domain model, then writes code and tests per unit | Clarifies technical and architectural choices in real time | `codev/plans/<project>.md`, `src/**/*.rs` |
| **Operations** | — | Proposes the build/release path, carries context forward | Oversees and approves the release | `codev/reviews/<project>.md` |

## Why four engine phases, not three

`protocol.json` declares **four** phases: `inception`, `design`, `construction`,
`operations`. AIDLC itself has three. The split is a constraint of the codev engine,
not a change to the methodology:

- A `per_plan_phase` phase iterates over the phases declared in a plan artifact, so
  the plan has to exist before iteration can start.
- AIDLC's Construction covers *both* logical design and code.

So Construction is split into `design` (produces the plan, one plan phase per unit of
work) and `construction` (one bolt per unit). Both belong to AIDLC's Construction
phase; `design` is its front half. Read the two together when comparing this protocol
against the published methodology.

## Phase 1 — Inception (Mob Elaboration)

**Goal**: business intent → validated requirements and units of work.

AI drafts `codev/specs/<project>.md` from the template. Three sections are enforced by
checks:

- `## Units of Work` — the decomposition. Each entry becomes one bolt, and later one
  phase in the design plan. If a unit cannot be built and verified inside a single
  bolt, split it here rather than during Construction.
- `## Clarifications` — every question AI asked the team, **with the answer**. This is
  the phase's real output: it is the context the later phases inherit, and it is what
  makes the next bolt cheaper than the last.
- Requirements themselves.

**Do not guess.** When intent is ambiguous, emit `CLARIFICATION_NEEDED` with the
question and pause. A guessed requirement that survives to Operations costs far more
than the interruption.

**Gate**: `intent-approval`. The team confirms the units match the intent.

## Phase 2 — Construction, logical design

**Goal**: a design the team has validated, before any code exists.

AI drafts `codev/plans/<project>.md`: the logical architecture, plus a `## Domain
Model` section naming the types and invariants introduced. For this project that
means concrete Rust — which newtypes, which enums replace stringly-typed state, which
invariant each type makes unrepresentable.

The plan carries a `"phases":` block, one phase per unit of work from Inception.

**Gate**: `design-approval`. Code is not written before this gate passes.

## Phase 3 — Construction, Mob Construction

**Goal**: one bolt per unit of work.

For each plan phase, AI writes the code and the tests together, then runs the checks:

| Check | Command | On failure |
|---|---|---|
| `build` | `cargo check --all-targets` | retry ×2 |
| `tests` | `cargo test` | retry ×2 |
| `fmt` | `cargo fmt --check` | retry ×1 |
| `clippy` | `cargo clippy --all-targets -- -D warnings` | fail |

These are cargo commands, not the npm commands SPIR's `protocol.json` carries — SPIR's
were inherited from a JavaScript project and do not run here.

Follow the project workflow inside the bolt: write the test, watch it fail, make the
minimal fix, watch it pass. A bolt that ends with passing tests that were never
observed failing has not demonstrated anything.

`clippy` does not retry on purpose. A lint failure after `build`, `tests` and `fmt`
have passed usually means the design is fighting the borrow checker or the API, and
that is a question for the team rather than something to iterate at.

**Transition**: back to `construction` for the next unit; to `operations` when all
units are done.

## Phase 4 — Operations

**Goal**: release readiness and a retrospective that improves the next bolt.

AI drafts `codev/reviews/<project>.md` with two enforced sections:

- `## Context Updates` — what the next bolt inherits. Since this project keeps
  architecture and design rationale in rustdoc starting at `src/lib.rs`, that means
  naming the rustdoc, feature flags and settings that changed, or stating that none
  did. Do not create a separate architecture document.
- `## Bolt Retrospective` — where AI proposals needed human correction. This is the
  feedback channel that makes Mob Elaboration cheaper next time.

`cargo build --release` must succeed: the release binary is what users run, and it can
fail where `cargo check` passes.

**Gate**: `operations-approval`.

## Signals

| Signal | Meaning |
|---|---|
| `PHASE_COMPLETE` | The current bolt's build step is done; move to verification. |
| `CLARIFICATION_NEEDED` | AI has a question only the team can answer. Requires `question`. The bolt pauses. |
| `BLOCKED` | The bolt cannot proceed. Requires `reason`. |

`CLARIFICATION_NEEDED` is what distinguishes AIDLC from a protocol that simply runs
unattended. Using it is a success, not a stall.

## Multi-agent consultation

Every phase verifies with `gemini`, `codex` and `claude` in parallel, as SPIR does,
and requires the `consult` CLI. Check with `codev doctor`. To run without
consultation, say "without consultation" when starting work.

Consultation is not a substitute for the gates. Models verify the artifact; the team
approves the intent. An artifact all three models like can still be the wrong thing to
build, and only the gate catches that.

## Choosing between AIDLC and SPIR

Both protocols live in this repo. SPIR remains the active default.

- **AIDLC** suits work where the intent is known but the decomposition is not, and
  where the team is available to answer questions inside the bolt.
- **SPIR** suits work where a human already holds the design and wants models as
  reviewers at checkpoints.

Do not mix them inside one project: the artifacts share paths under `codev/specs/`,
`codev/plans/` and `codev/reviews/`, but the required sections differ, and each
protocol's checks enforce its own.
