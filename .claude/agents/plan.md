---
name: plan
description: Create a new Codev SPIR implementation plan in codev/plans/ for RustShot
---

Create a new SPIR plan document for RustShot following the template at
`codev/protocols/spir/templates/plan.md`.

Steps:

1. **Find the matching spec.**
   The plan filename must match its spec exactly (same `####-name` prefix).
   List `codev/specs/` to find the relevant spec file.
   If the user has not specified a spec, ask which spec this plan is for before continuing.

2. **Read the spec.**
   Open the matching `codev/specs/####-name.md` and extract:
    - The chosen solution approach
    - Success criteria
    - Constraints and assumptions
    - Open questions that are now resolved

3. **Determine the filename.**
   Use the exact same name as the spec: `codev/plans/####-name.md`.

4. **Fill in the template.**
   Copy the structure from `codev/protocols/spir/templates/plan.md`.
    - Set today's date in the Metadata section.
    - Set Status to `draft`.
    - Link to the spec file.
    - Break the work into logical phases. Each phase must:
        - Have a single clear objective.
        - Be independently testable.
        - Deliver observable value.
        - End with a git commit.
    - Fill in the machine-readable JSON phases block.
    - For each phase, fill in: Objectives, Deliverables, Implementation Details,
      Acceptance Criteria, Test Plan, Rollback Strategy, and Risks.

5. **Write the file** to `codev/plans/####-name.md`.

6. **Report** the filename created and summarize the phases at a glance.

Rules:

- NEVER include time estimates (hours, days, story points). Focus on deliverables only.
- Keep phases small. If a phase feels too large, split it.
- The plan defines HOW. Refer to the spec for WHY.
- Do NOT create the spec file during this step — it must already exist.
- For RustShot: keep phases Rust-idiomatic. Each phase should leave `cargo test` green.
