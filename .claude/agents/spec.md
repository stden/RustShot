---
name: spec
description: Create a new Codev SPIR specification in codev/specs/ for RustShot
---

Create a new SPIR specification document for RustShot following the template at
`codev/protocols/spir/templates/spec.md`.

Steps:

1. **Determine the next ID.**
   List files in `codev/specs/`. The filename format is `####-descriptive-name.md`
   (zero-padded to 4 digits). Find the highest existing number and increment by 1.
   If the directory is empty, start at `0001`.

2. **Choose the filename.**
   Use the feature name or short description provided by the user.
   If the feature description is missing, ask for it before continuing.
   Format: `codev/specs/####-short-name.md`
   Example: `codev/specs/0002-hotkey-configuration.md`

3. **Fill in the template.**
   Copy the structure from `codev/protocols/spir/templates/spec.md`.
   - Set today's date in the Metadata section.
   - Set Status to `draft`.
   - Fill in all sections based on what the user has described.
   - Leave placeholder text for sections you do not have enough information to fill.
   - Mark open questions that need user input.

4. **Write the file** to `codev/specs/####-short-name.md`.

5. **Report** the filename created and list any sections that still need the user's input.

Rules:
- The spec defines WHAT and WHY. Do NOT include implementation steps, file paths to modify,
  or code examples — those belong in the plan.
- Do NOT create the plan file during this step.
- Do NOT include time estimates anywhere.
- Keep the document factual and concise.
