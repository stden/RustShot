---
name: review
description: Write a Codev SPIR review document into codev/reviews/ for a completed project. Produces a file. For a read-only code review that only reports findings, use the /review command instead.
---

Create a SPIR review document for RustShot following the template at
`codev/protocols/spir/templates/review.md`.

Steps:
1. **Find the matching spec and plan.**
   The review filename must match the spec and plan exactly (same `####-name` prefix).
   List `codev/specs/` and `codev/plans/` to locate the files.
   If the user has not specified which feature, ask before continuing.

2. **Read the spec and plan.**
   Open both files and extract:
   - All success/acceptance criteria from the spec.
   - All phases and their acceptance criteria from the plan.
   - Any deviations or notes recorded during implementation.

3. **Gather implementation facts.**
   Run these commands to collect objective data:
   - `git log --oneline` — count commits on the branch.
   - `cargo test` — confirm all tests pass; note count.
   - `git status --short` — capture any uncommitted changes that may affect the review.
   - List files created or deleted for this feature.

4. **Determine the filename.**
   Use the exact same name as the spec and plan: `codev/reviews/####-name.md`.

5. **Fill in the template.**
   Copy the structure from `codev/protocols/spir/templates/review.md`.
   - Summary: what was built, how many phases, outcome.
   - Spec Compliance: check each acceptance criterion against what was implemented.
   - Deviations from Plan: note any phases that changed and why.
   - Current Worktree State: note any relevant uncommitted changes rather than ignoring them.
   - Key Metrics: commits, tests passing, files created/deleted, net LOC.
   - Lessons Learned: what went well, what was hard, what to do differently.
   - Architecture Updates: note if the module rustdoc (start at `src/lib.rs`) needs updating.
   - Technical Debt: any shortcuts or rough edges left behind.
   - Follow-up Items: future work identified but out of scope.

6. **Write the file** to `codev/reviews/####-name.md`.

7. **Report** the filename and a one-paragraph summary of the feature outcome.

Rules:
- Be honest. If criteria were not met, mark them as not met and explain why.
- Keep the review factual. Avoid vague praise.
- Do NOT modify the spec or plan during this step.
- Architecture and design rationale live in the code as rustdoc (see `src/lib.rs`),
  not in a separate arch document. If new modules or design decisions need
  documenting, note it clearly in the review and offer to update the rustdoc separately.
