---
name: check
description: Run cargo check, clippy, and fmt --check to verify code health
---

Run the three standard code-quality checks for RustShot in order. Stop and report if any step fails.

Steps:
1. `cargo check` — verify the code compiles without building a binary.
2. `cargo clippy --all-targets -- -D warnings` — lint all targets; treat all warnings as errors.
3. `cargo fmt --all --check` — verify formatting without modifying files.

For each step:
- If it passes, say so briefly.
- If it fails, show the relevant error or warning output and stop. Do NOT proceed to the next step.

After all three pass, give a one-line summary: "check, clippy, fmt: all clean."

Do NOT auto-fix issues. Only report them unless the user asks for a fix.
