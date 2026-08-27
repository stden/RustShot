# Operations: [Title]

<!--
AIDLC Operations artifact: release readiness plus the retrospective that makes
the next bolt cheaper.

`## Context Updates` and `## Bolt Retrospective` are enforced by protocol
checks. Renaming either heading fails the operations-approval gate.
-->

## Metadata
- **ID**: operations-[YYYY-MM-DD]-[short-name]
- **Status**: draft
- **Inception**: [Link to codev/specs/[title].md]
- **Design**: [Link to codev/plans/[title].md]
- **Created**: [YYYY-MM-DD]
- **Protocol**: aidlc

## Summary

[1-3 sentences: what shipped, how many bolts, net outcome.]

## Bolts Completed

| Bolt | Unit | Outcome | Commits |
|---|---|---|---|
| u1 | U1: [name] | [shipped / descoped — reason] | [short shas] |

## Acceptance Criteria

- [x] **AC1**: [Description] (bolt u1)
- [ ] **ACn**: [Not met — reason, and whether it blocks release]

## Release Readiness

| Check | Command | Result |
|---|---|---|
| Compile | `cargo check --all-targets` | [pass/fail] |
| Tests | `cargo test` | [N passing] |
| Format | `cargo fmt --check` | [pass/fail] |
| Lint | `cargo clippy --all-targets -- -D warnings` | [pass/fail] |
| Release binary | `cargo build --release` | [pass/fail] |
| Benchmarks | `cargo bench` | [within budget / drift — see below] |

### Platform verification

| Platform | Verified how | Result |
|---|---|---|
| Windows | [manual / CI / not verified] | [result] |
| Linux | [manual / CI / not verified] | [result] |

## Context Updates

<!--
What the next bolt inherits. This project keeps architecture and design
rationale in rustdoc starting at src/lib.rs — there is no separate
architecture document, and one must not be created here.

Name what changed, or state explicitly that nothing did and why that is right.
-->

- **Rustdoc**: [Which module/function docs changed, and what they now explain. Or "none — no design rationale changed".]
- **Gherkin features**: [Features added or changed in tests/features/. Or "none".]
- **Settings / CLI surface**: [New flags or persisted settings, and their defaults. Or "none".]
- **Benchmark budgets**: [Budgets added or moved in benches/performance_gate.rs, with the measured basis. Or "none".]

## Bolt Retrospective

<!--
Where AI proposals needed human correction. This is the feedback channel that
makes Mob Elaboration cheaper next time — it is the point of the phase, not
paperwork.

Be specific about the cause. "AI misunderstood the requirement" is not
actionable; "Inception recorded no answer for how the overlay behaves on
multi-monitor, so the design assumed one surface" is.
-->

| What AI proposed | What the team corrected | Root cause | What to do differently |
|---|---|---|---|
| [Proposal] | [Correction] | [Missing clarification / wrong assumption / stale context] | [Concrete change to the next Inception] |

### Clarifications that paid off

[Which `CLARIFICATION_NEEDED` pauses prevented rework. Justifies the interruption cost.]

### Clarifications that were missed

[Questions that should have been asked during Inception and were not. These become required questions next time.]

## Deviations from Design

- **Bolt u-n**: [What changed from the validated design, and why. A deviation is not a failure; an unrecorded one is.]

## Follow-up Work

| Item | Why deferred | Where tracked |
|---|---|---|
| [Item] | [Reason] | [Issue / next Inception] |
