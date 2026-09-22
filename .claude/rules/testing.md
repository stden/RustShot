---
paths:
  - "tests/**"
  - "src/**/tests.rs"
  - "src/**/tests/**"
---

# Testing RustShot

<!-- Path-scoped: loads when a test file is opened, not in every session. -->

## Live capture needs a *changing* desktop

Windows Graphics Capture delivers a frame only when the desktop changes, so on an
idle screen the capture tests time out for purely environmental reasons — not
because anything regressed. This has produced repeated false failures, each passing
on a plain re-run and reproducible on older commits.

`RUSTSHOT_LIVE_CAPTURE_STRICT` waives **exactly** the idle-desktop timeout, matched
by its exact message, and nothing else: wrong dimensions, any other error, or a
panic still fail. Do not widen that waiver — and do not remove the `served > 0`
floor in the concurrency stress test, which is the only assertion left that catches
a total WGC frame-delivery break on Windows.

## Honesty about red-first

Write the test first, watch it fail, make the minimal fix, watch it pass, run the
whole suite. Some changes genuinely cannot have a failing test: a comment, a doc, a
platform path this machine cannot execute, a benchmark budget. Say so plainly. A
test that passes before and after is a *pin*, which is worth having — but calling it
TDD is a lie that hides the absence of evidence.

If a proposed test cannot be made to fail against current code, try mutating the
code to prove the test would catch the regression it claims to guard, then revert
the mutation. That is what turned an AVX2 tail sweep from decoration into a test
that kills a mutant surviving the whole suite.

## Shape

Prefer small pure-logic units to GUI tests; drive selection, window and overlay
logic that way. Where a real window or display is unavoidable, guard on the
environment and skip loudly rather than failing on it — but make sure the guard
cannot silently become an everywhere-green no-op.

Behaviour is specified executably in `tests/features/` (Gherkin, run by
`tests/bdd.rs`). That *is* the behavioural spec, so a behaviour change belongs there
as well as in a unit test.
