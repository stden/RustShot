---
name: test
description: Run the full RustShot test suite and report failures
---

Run all tests for RustShot with `cargo test` and report the results clearly.

Steps:
1. Run `cargo test` in the project root.
2. If all tests pass, confirm with the count of tests that passed.
3. If any tests fail, show:
   - The name of each failing test
   - The failure output / assertion message
   - A brief diagnosis of what likely caused the failure
4. Do NOT auto-fix failures. Report them and stop. Only fix if the user explicitly asks.

Keep the output concise. Lead with pass/fail status, then details.
