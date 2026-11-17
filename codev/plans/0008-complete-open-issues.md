# Issues 6–10 Implementation Plan

> For agentic workers: use subagent-driven development, test-driven development,
> and independent specification and code-quality review.

**Goal:** implement the remaining code, tests, measurement tools and documentation
for issues 6–10 without changing established defaults.

**Architecture:** extend existing Rust modules with small pure helpers, keep
platform calls at their current boundaries, and use separate executables for live
measurement and bounded recording verification.

**Tech stack:** Rust, Criterion, Cucumber, native Win32 controls, global-hotkey,
arboard, screenshots, D-Bus desktop portals, GitHub Actions.

## Task 1 — Runtime hotkeys (#9)

Files: `src/hotkeys.rs`, `src/settings.rs`, `src/settings_dialog.rs`, `src/main.rs`,
`src/lib.rs`, relevant settings struct literals, and BDD settings scenarios.

- [ ] Test defaults, parsing, canonical duplicate keys, successful updates, reused
  bindings, registration rollback, and unknown nested JSON round trips.
- [ ] Observe failures with `cargo test --lib hotkeys` and focused settings tests.
- [ ] Implement typed bindings and a transaction that registers new keys before
  unregistering obsolete keys; keep original IDs on any registration failure.
- [ ] Add native shortcut fields and reset controls, and a tray refresh path that
  applies saved settings without restarting or repeatedly retrying a failed file.
- [ ] Run hotkey/settings unit and BDD tests; document runtime error behavior.

## Task 2 — Capture latency (#6)

Files: `src/latency.rs`, `examples/capture_latency.rs`, module exports, production
capture/clipboard boundary integration, and dedicated measurement documentation.

- [ ] Test nearest-rank percentiles, empty input, sample order, phase totals,
  disabled instrumentation and error propagation with deterministic durations.
- [ ] Observe focused failures with `cargo test --lib latency`.
- [ ] Implement opt-in phase timing and a real capture/clipboard request harness,
  using fixed 1080p/4K regions, warmup and a documented sample count.
- [ ] Report phase and total p50/p95/worst and explain human-selection exclusion.
- [ ] Compile the harness and run pure timing tests; record platform prerequisites.

## Task 3 — 4K overlay budgets (#7)

Files: `benches/benchmarks.rs`, `src/selection.rs`, a pure overlay-budget example,
and dedicated measurement documentation.

- [ ] Test memory arithmetic, overflow and cache invalidation across dimensions.
- [ ] Observe focused test failures before adding new helpers.
- [ ] Add 4K crosshair, drag and cache-rebuild Criterion cases and dimension-change
  coverage, without changing the pixel representation.
- [ ] Run the pure measurement executable and relevant Criterion cases; report
  hardware, sample counts, expected/actual allocated cache bytes and <16.7 ms target.
- [ ] Record measurements without claiming Windows or compositor performance.

## Task 4 — Windows lifecycle integration (#8)

Files: a dedicated `tests/windows_recording_lifecycle.rs`, Windows CI steps and
recording-test documentation; change recording code only if regression tests expose
a necessary cleanup fix.

- [ ] Test metadata validation with valid/invalid ffprobe JSON or structured output
  and test timeout handling using a harmless process.
- [ ] Run the tests to observe failure before implementing helpers.
- [ ] Implement a parent watchdog with child-process isolation, deterministic
  temporary output directories and cleanup guards; child calls the real recording
  API and stops through the application lifecycle.
- [ ] Validate MP4 container, H.264 codec, encoded dimensions and nonzero duration;
  fail on missing desktop or missing validation tools.
- [ ] Run the Windows test explicitly in CI with a workflow timeout and documented
  ffprobe installation; compile platform code where possible locally.

## Task 5 — Wayland completion (#10)

Files: `src/portal.rs`, `src/capture.rs`, `src/selection.rs`, CLI dispatch where
needed, `README.md`, `README.ru.md`, and Wayland test documentation.

- [ ] Add regression tests for unsupported region/list operations and preserved
  X11 dispatch, alongside existing cancellation/no-output tests.
- [ ] Observe failures and implement capability guards before X11 enumeration.
- [ ] Document GNOME and KDE prerequisites, permission approval, cancellation,
  denial, absent portal, and unsupported-operation commands; mark actual runtime
  results as unverified until tested on those desktops.
- [ ] Correct both capability tables and identify MCP as a separate dependency.

## Integration and review

- [ ] Review each task against its issue criteria, then review code quality.
- [ ] Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
  the full suite under Xvfb, and `cargo bench --no-run`.
- [ ] Run repository duplication checks and knowledge validation if indexed
  documentation changes require it.
- [ ] Record actual results and remaining platform checks in the matching review.
- [ ] Integrate the verified change into the user checkout and push the completed
  implementation; report remaining external validation explicitly.
