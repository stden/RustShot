# Plan: RustShot Screenshot and Recording Utility

## Metadata
- **ID**: plan-2026-03-13-rustshot-screenshot-utility
- **Status**: draft
- **Specification**: [0001-rustshot-screenshot-utility](../specs/0001-rustshot-screenshot-utility.md)
- **Created**: 2026-03-13

## Executive Summary
This plan implements the approved core RustShot product scope: tray-first
screenshots, CLI capture, interactive region selection, Windows recording, and
multi-monitor correctness. The chosen approach is to preserve the current
single-binary architecture and tighten behavior with small helper extraction,
focused tests, and minimal user-visible change.

The plan is intentionally centered on shipping a stable baseline before adding
new product surfaces. Screenshot and recording correctness come first, then
runtime polish, then final validation and documentation alignment.

## Success Metrics
- [ ] All specification criteria from `0001-rustshot-screenshot-utility.md` remain satisfied.
- [ ] Core screenshot and recording behavior is covered by unit tests for helper logic.
- [ ] `cargo test` passes on Windows and Linux CI jobs.
- [ ] `cargo clippy -D warnings` passes on the Windows CI job.
- [ ] Windows tray mode, PrintScreen, and Ctrl+Shift+R smoke tests pass manually.
- [ ] Documentation reflects the actual supported modes and limitations.

## Phases (Machine Readable)

```json
{
  "phases": [
    {"id": "phase_1_cli_capture", "title": "CLI and Single-Screen Capture Hardening"},
    {"id": "phase_2_selection_clipboard", "title": "Selection Overlay and Clipboard Hardening"},
    {"id": "phase_3_recording", "title": "Recording Robustness and Stop Flow"},
    {"id": "phase_4_validation_docs", "title": "Validation and Documentation Closure"}
  ]
}
```

## Phase Breakdown

### Phase 1: CLI and Single-Screen Capture Hardening
**Dependencies**: None

#### Objectives
- Make CLI mode routing deterministic and easy to test.
- Ensure capture defaults are correct on single-monitor and multi-monitor systems.

#### Deliverables
- [ ] Finalize `should_run_cli_mode` behavior for `--cli`, `--list`, explicit output, screen, and non-default format cases.
- [ ] Finalize default screen resolution logic when `--screen` is omitted.
- [ ] Keep output path, format mapping, and directory creation behavior stable.
- [ ] Add or adjust helper-level tests for routing and screen selection.

#### Implementation Details
Primary files:
- `src/main.rs`
- `src/capture.rs`

Technical approach:
- Keep `clap`-driven argument parsing and existing `run_cli_with_capture` test seams.
- Resolve default monitor selection through a pure helper that prefers the primary screen and falls back to index 0.
- Preserve current `Result<_, String>` behavior and human-readable error messages.

#### Acceptance Criteria
- [ ] CLI mode is entered for all documented CLI triggers and not entered for tray-only launches.
- [ ] Default screen capture selects the primary monitor when available.
- [ ] Out-of-range screen indexes still produce clear human-readable errors.
- [ ] Output paths and format handling remain backward-compatible.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: CLI mode detection, screen index resolution, format conversion, output path resolution, parent directory creation.
- **Integration Tests**: Existing `capture_screenshot_with` and `run_cli_with_capture` tests.
- **Manual Testing**: `rustshot --list`, `rustshot --cli`, explicit `--screen`, and explicit `--output` on Windows.

#### Rollback Strategy
Revert changes in `src/main.rs` and `src/capture.rs` to the previous mode-routing
and screen-resolution helpers without touching unrelated modules.

#### Risks
- **Risk**: New CLI routing rules accidentally bypass tray mode.
  - **Mitigation**: Keep a single helper for mode decisions and cover every trigger combination with tests.

---

### Phase 2: Selection Overlay and Clipboard Hardening
**Dependencies**: Phase 1

#### Objectives
- Keep the interactive selection flow stable across cancel, retry, and multi-monitor scenarios.
- Ensure clipboard behavior stays reliable and isolated from GUI-only code paths.

#### Deliverables
- [ ] Finalize selection-region normalization and minimum-size rejection behavior.
- [ ] Finalize screen lookup and screen-local coordinate conversion for selected regions.
- [ ] Keep clipboard writes behind testable helper boundaries.
- [ ] Add targeted tests for crosshair, drag state, cancel flow, and screen lookup helpers.

#### Implementation Details
Primary files:
- `src/selection.rs`
- `src/main.rs`

Technical approach:
- Preserve the child-process overlay model.
- Test through extracted pure functions rather than event-loop integration.
- Keep cancel and retry behavior user-driven instead of adding new automatic retries.

#### Acceptance Criteria
- [ ] Valid selections return a cropped image in the expected region.
- [ ] Tiny selections are rejected consistently.
- [ ] Cancel flows do not mutate clipboard state unexpectedly.
- [ ] Multi-monitor selected regions map to the correct underlying screen.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: Region creation, drag-to-rectangle logic, minimum size checks, screen lookup, screen-local coordinate conversion.
- **Integration Tests**: Existing selection helpers with mock clipboard paths.
- **Manual Testing**: Select, cancel with Escape, cancel with right click, and select across virtual desktop boundaries.

#### Rollback Strategy
Revert selection helper changes while preserving the existing window/event-loop
construction and clipboard wiring.

#### Risks
- **Risk**: Pure helper changes drift from `winit` event behavior.
  - **Mitigation**: Keep helper APIs close to the data already used by the event loop and run manual smoke tests after changes.

---

### Phase 3: Recording Robustness and Stop Flow
**Dependencies**: Phase 2

#### Objectives
- Harden Windows recording behavior without introducing a second recording architecture.
- Make recording start/stop behavior predictable from both tray and internal child-process paths.

#### Deliverables
- [ ] Finalize recording parameter validation including FPS checks and Windows-only guard rails.
- [ ] Finalize even-dimension rounding for H.264 frame sizes.
- [ ] Preserve the stop-flag and background-thread model while improving error handling and tests.
- [ ] Remove misleading tray affordances that do not reflect the true toggle behavior.

#### Implementation Details
Primary files:
- `src/record.rs`
- `src/tray.rs`
- `src/main.rs`

Technical approach:
- Keep Media Foundation as the encoder and avoid re-architecting to FFmpeg or another backend.
- Keep the existing background-thread model and `stop_tx` join semantics.
- Continue using a file-based stop signal between the tray process and the recording child process.

#### Acceptance Criteria
- [ ] `RecordingSession::start` rejects invalid FPS values.
- [ ] Non-Windows recording attempts fail with a clear message.
- [ ] Odd dimensions are rounded to valid H.264 dimensions before encoding.
- [ ] The tray recording action clearly represents toggle semantics.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: FPS validation, frame duration helper, even-dimension helper, RGBA-to-BGRA conversion, stop-path helpers, tray label/menu logic.
- **Integration Tests**: Existing `record.rs` tests and any non-GUI process-level tests for stop signaling.
- **Manual Testing**: Windows region recording start/stop, saved MP4 playback, repeated recording toggles.

#### Rollback Strategy
Revert recording and tray UX changes as a unit so menu semantics and background
recording behavior stay aligned.

#### Risks
- **Risk**: Recording fixes mask platform-specific Media Foundation failures.
  - **Mitigation**: Preserve explicit error propagation and keep Windows smoke tests in the checkpoint list.

---

### Phase 4: Validation and Documentation Closure
**Dependencies**: Phase 3

#### Objectives
- Finish the core product work with evidence, not just code changes.
- Ensure docs and tests describe what actually ships.

#### Deliverables
- [ ] Reconcile the implementation against the functional and edge-case scenarios in the supporting validation doc.
- [ ] Update assistant-facing and user-facing docs for any net behavior changes.
- [ ] Produce a manual smoke checklist for tray, CLI, selection, and recording.
- [ ] Confirm the repo is ready for dependent work such as settings and MCP.

#### Implementation Details
Primary files:
- `README.md`
- `README.ru.md`
- `GEMINI.md`
- `CLAUDE.md`
- `CLAUDE.ru.md`

Technical approach:
- Treat docs as behavior contracts.
- Keep the supporting validation document in `codev/specs/0002-test-plan-screenshots-recording.md` as the source of test scenarios.
- Avoid starting feature expansion during closure work.

#### Acceptance Criteria
- [ ] Docs do not promise unsupported behavior.
- [ ] Windows-only recording limitations are explicit.
- [ ] Manual validation steps exist for tray, hotkeys, selection, and recording.
- [ ] Core product scope is ready for review or dependent feature specs.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: None expected beyond any doc-driven helper corrections.
- **Integration Tests**: Full `cargo test` run.
- **Manual Testing**: End-to-end run through tray capture, CLI capture, selection cancel, and recording start/stop.

#### Rollback Strategy
Revert documentation-only changes independently if wording outruns the actual
implementation state.

#### Risks
- **Risk**: Documentation drifts again during follow-up feature work.
  - **Mitigation**: Use this phase as the required closure checkpoint before moving to dependent specs.

## Dependency Map
```text
Phase 1 -> Phase 2 -> Phase 3 -> Phase 4
```

## Resource Requirements
### Development Resources
- **Engineers**: Rust contributor comfortable with `clap`, `winit`, `screenshots`, and Windows-only recording APIs.
- **Environment**: Windows machine for tray/hotkey/recording smoke tests and Linux CI for non-Windows validation.

### Infrastructure
- GitHub Actions for format, clippy, and test jobs.
- No additional services required.

## Integration Points
### External Systems
- **Windows Media Foundation**
  - **Integration Type**: Native multimedia API
  - **Phase**: Phase 3
  - **Fallback**: Clear Windows-only error when unavailable
- **Clipboard**
  - **Integration Type**: Desktop OS service
  - **Phase**: Phase 2
  - **Fallback**: Return a human-readable failure instead of silently dropping the image

### Internal Systems
- **Capture pipeline**
  - **Integration Type**: Reused module API
  - **Phase**: Phase 1, Phase 2, Phase 3
  - **Fallback**: Keep helper seams stable for tests

## Risk Analysis
### Technical Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Multi-monitor edge cases are under-tested | M | H | Add helper-level tests and manual virtual desktop smoke tests | Implementer |
| Tray/hotkey behavior regresses on Windows only | M | H | Keep changes local and validate on Windows after each phase | Implementer |
| Media Foundation failures are hard to reproduce in CI | H | M | Preserve unit-testable helpers and keep a manual recording smoke checklist | Implementer |

### Schedule Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Windows-only validation takes longer than code changes | M | M | Front-load smoke checks after the recording phase instead of at the end only | Implementer |

## Validation Checkpoints
1. **After Phase 1**: Run CLI capture flows and `cargo test`.
2. **After Phase 2**: Verify selection and clipboard behavior manually on Windows.
3. **After Phase 3**: Verify region recording start/stop and MP4 playback on Windows.
4. **Before Production**: Re-run docs sanity pass, full tests, and the manual smoke checklist.

## Monitoring and Observability
### Metrics to Track
- CI pass/fail for test and clippy jobs.
- Manual smoke-test completion for tray capture and recording flows.

### Logging Requirements
- Keep runtime errors human-readable through `Result<_, String>`.
- Print warnings for recoverable recording frame-write failures only where they help debugging.

### Alerting
- GitHub Actions failures are the primary alerting path.

## Documentation Updates Required
- [x] User guides
- [x] Configuration notes
- [x] Assistant-facing architecture notes
- [ ] Final review document in the later SPIR Review phase

## Post-Implementation Tasks
- [ ] Manual performance sanity check for capture latency and recording responsiveness
- [ ] Reconcile open items in the supporting validation doc
- [ ] Prepare review-ready evidence for core product behavior

## Expert Review
**Date**: 2026-03-13
**Model**: GPT-5 Codex
**Key Feedback**:
- Preserve the single-binary architecture and make behavior fixes through testable helpers.
- Finish correctness and validation work before expanding the product surface.
- Keep Windows-only behavior explicit in both code and docs.

**Plan Adjustments**:
- Broke the work into CLI/capture, selection, recording, and validation phases.
- Kept settings, MCP, and release work out of the core product plan so follow-up specs remain independent.

## Approval
- [ ] Technical Lead Review
- [ ] Engineering Manager Approval
- [ ] Resource Allocation Confirmed
- [ ] Expert AI Consultation Complete

## Change Log
| Date | Change | Reason | Author |
|------|--------|--------|--------|
| 2026-03-13 | Created detailed implementation plan for the approved core RustShot spec | Needed a matching SPIR plan artifact for spec 0001 | Codex |

## Notes
- The supporting validation scenarios currently live in `codev/specs/0002-test-plan-screenshots-recording.md`.
- This plan is the execution document for the existing shipped and near-shipped core behavior only.

---

## Amendment History

This section tracks all TICK amendments to this plan. TICKs modify both the
spec and plan together as an atomic unit.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
