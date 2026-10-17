Add tests to improve coverage for the specified module (or all modules if none specified).

Argument: $ARGUMENTS (optional module name, e.g. "capture", "record", "selection", "settings", "tray", "main")

Steps:
1. Read the target module source file(s).
2. Identify functions that lack test coverage, focusing on:
   - Private helper functions
   - Edge cases for existing tested functions
   - Error paths
   - Recent bug-prone behavior that should get regression coverage
3. Add new `#[test]` functions in the existing `#[cfg(test)]` blocks or under `tests/`
   when integration coverage is the better fit.
4. Ensure no duplicate test names.
5. Prefer not to change production code. Only make minimal testability refactors if they
   are required and do not change behavior.
6. Run `cargo fmt --all`.
7. Run `cargo clippy --all-targets -- -D warnings`.
8. Run `cargo test` to verify all tests pass.
9. Report how many tests were added and their names.

Keep the implementation changes as small as possible.
