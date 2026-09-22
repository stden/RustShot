Create a new Codev SPIR implementation plan in codev/plans/ for RustShot.

Argument: $ARGUMENTS (spec name or number)

1. Find the matching spec in `codev/specs/`. If no spec name or number is given, ask which
   spec the plan should target before continuing.
2. Read the spec and extract requirements.
3. Read the template at `codev/protocols/spir/templates/plan.md`.
4. Break work into small, independently testable phases. Each phase ends with a green `cargo test`.
5. Write to `codev/plans/####-name.md` (matching the spec filename).
6. Report the phases at a glance.

NEVER include time estimates. The plan defines HOW; refer to the spec for WHY.
