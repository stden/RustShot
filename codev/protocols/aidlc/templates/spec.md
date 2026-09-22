# Inception: [Title]

<!--
AIDLC Inception artifact (Mob Elaboration).

This defines WHAT and WHY, and decomposes it into units of work. It does NOT
define HOW — no file paths, no type signatures, no code. Those belong in the
design artifact, codev/plans/[title].md.

`## Units of Work` and `## Clarifications` are enforced by protocol checks.
Renaming either heading fails the intent-approval gate.
-->

## Metadata
- **ID**: inception-[YYYY-MM-DD]-[short-name]
- **Status**: draft
- **Created**: [YYYY-MM-DD]
- **Protocol**: aidlc

## Business Intent

[The outcome someone wants, in their words, not in implementation terms. One paragraph.]

## Clarifications

<!--
Every question AI asked the team, WITH the answer. This is the phase's real
output: later bolts inherit this context, and it is what makes the next bolt
cheaper than this one.

An unanswered question is not a clarification — either emit
CLARIFICATION_NEEDED and wait, or record it under Assumptions below and say
what happens if it is wrong.
-->

| Question | Answer | Asked of |
|---|---|---|
| [Question] | [Answer] | [Who] |

### Assumptions

[Anything proceeding on an unconfirmed answer. State the assumption and what breaks if it is wrong. Empty is the goal.]

## Requirements

- **R1**: [Requirement, observable from outside the program]
- **R2**: [...]

### Out of scope

[What this explicitly does not cover. Prevents scope drift during Construction.]

## Units of Work

<!--
The decomposition. Each unit becomes one bolt, and one phase in the design plan.

Sizing rule: a unit must be buildable AND verifiable within a single bolt
(hours, not weeks). If it cannot be, split it HERE — splitting during
Construction means the design was validated against the wrong shape.
-->

### U1: [Name]
- **Delivers**: [Observable change when this unit is done]
- **Requirements covered**: R1, R2
- **Verified by**: [What test proves it — the kind of test, not its name]
- **Depends on**: [none, or U-n]

### U2: [Name]
- **Delivers**: [...]
- **Requirements covered**: [...]
- **Verified by**: [...]
- **Depends on**: [...]

## Acceptance Criteria

- [ ] **AC1**: [Testable condition, traceable to a requirement]
- [ ] **AC2**: [...]

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| [Risk] | [What it costs] | [What we do about it] |
