# Plan: RustShot Platform Hardening and Integration Roadmap

## Metadata
- **ID**: plan-2026-03-13-rustshot-platform-roadmap
- **Status**: draft
- **Specification**: [0001-rustshot-screenshot-utility](../specs/0001-rustshot-screenshot-utility.md)
- **Supporting Validation**: [0002-test-plan-screenshots-recording](../specs/0002-test-plan-screenshots-recording.md)
- **Created**: 2026-03-13

## Executive Summary
This plan covers the full current RustShot repository scope: screenshot capture,
interactive selection, Windows-only recording, tray UX, persisted settings,
MCP integration, release packaging, and documentation sync.

The implementation strategy stays deliberately conservative. RustShot remains a
single-binary application with small helper extraction for testability instead
of large architectural refactors. The work is ordered to finish and harden
features already visible in the tree first, then wire new integrations on top
of that stable base.

The sequence is:
1. Stabilize core screenshot and recording behavior already in flight.
2. Turn the new settings module into real application state.
3. Add MCP mode as a thin wrapper around existing capture and recording logic.
4. Harden release packaging and Windows distribution.
5. Close documentation and validation gaps so the branch is ready for the
   Implement gate and later Review work.

## Artifact Map
This roadmap is the umbrella planning document for the repository. Detailed
feature-level SPIR artifacts live in matching spec/plan pairs:

- Core product: `codev/specs/0001-rustshot-screenshot-utility.md` and
  `codev/plans/0001-rustshot-screenshot-utility.md`
- Settings: `codev/specs/0003-settings-and-runtime-configuration.md` and
  `codev/plans/0003-settings-and-runtime-configuration.md`
- MCP: `codev/specs/0004-mcp-server-integration.md` and
  `codev/plans/0004-mcp-server-integration.md`
- Release hardening: `codev/specs/0005-release-and-distribution-hardening.md`
  and `codev/plans/0005-release-and-distribution-hardening.md`

## Success Metrics
- [ ] All approved specification criteria remain satisfied.
- [ ] Core behavior changes are covered by focused unit tests in the affected modules.
- [ ] `cargo test` passes on Windows and Linux CI jobs.
- [ ] `cargo clippy -D warnings` passes on the Windows CI job.
- [ ] Settings defaults, round-trip persistence, and CLI override behavior are verified.
- [ ] MCP mode supports initialization, tool listing, tool execution, and structured errors.
- [ ] Tagged releases upload both `rustshot.exe` and `RustShotSetup.exe`.
- [ ] User-facing docs match shipped behavior, including Windows-only recording limits.

## Phases (Machine Readable)

<!-- REQUIRED: porch uses this JSON to track phase progress. -->

```json
{
  "phases": [
    {"id": "phase_1_core_hardening", "title": "Core Capture and Recording Hardening"},
    {"id": "phase_2_settings", "title": "Settings Persistence and Application"},
    {"id": "phase_3_mcp", "title": "MCP Server Delivery"},
    {"id": "phase_4_release", "title": "Release and Packaging Hardening"},
    {"id": "phase_5_docs_validation", "title": "Documentation and Validation Closure"}
  ]
}
```

## Phase Breakdown

### Phase 1: Core Capture and Recording Hardening
**Dependencies**: None

#### Objectives
- Finish correctness fixes in the current screenshot, selection, tray, and recording paths.
- Keep behavior changes minimal and always backed by small, explicit tests.

#### Deliverables
- [ ] Normalize CLI and tray routing around the existing application modes.
- [ ] Finalize default monitor selection and multi-monitor edge-case handling.
- [ ] Finalize recording guard rails such as FPS validation, even-dimension handling, and stop-flow behavior.
- [ ] Remove misleading tray actions or labels that do not reflect real behavior.
- [ ] Sync minimal docs for any user-visible behavior changes.

#### Implementation Details
Primary files:
- `src/main.rs`
- `src/capture.rs`
- `src/selection.rs`
- `src/record.rs`
- `src/tray.rs`
- `GEMINI.md`

Technical approach:
- Preserve the current child-process overlay flow for `--select` and `--select-record`.
- Preserve the file-based stop flag for recording unless a concrete defect requires a different IPC path.
- Extract only small pure helper functions when a behavior change needs direct unit coverage.
- Keep `Result<_, String>` and current module boundaries unless a change becomes untestable without a tiny helper split.

#### Acceptance Criteria
- [ ] Default capture without `--screen` resolves to the primary monitor or the first available monitor as fallback.
- [ ] CLI/tray mode routing remains deterministic and covered by tests.
- [ ] Recording start validation rejects invalid FPS and non-Windows recording attempts cleanly.
- [ ] Tray recording UX reflects the actual start/stop toggle behavior.
- [ ] `cargo test` passes after the phase changes.

#### Test Plan
- **Unit Tests**: Screen index resolution, CLI mode detection, recording validation, even-dimension helpers, tray label/menu logic.
- **Integration Tests**: Existing capture and record module tests plus any lightweight process-spawn tests that do not require GUI assertions.
- **Manual Testing**: Windows smoke test for PrintScreen capture, region selection cancel path, Ctrl+Shift+R start/stop, tray actions.

#### Rollback Strategy
Revert helper extractions and user-visible behavior changes in `main`, `capture`,
`record`, `selection`, and `tray` while keeping unrelated documentation edits out
of scope.

#### Risks
- **Risk**: Hotkey or tray changes regress Windows-only behavior.
  - **Mitigation**: Keep menu/hotkey logic changes local, add pure tests, and do a short Windows smoke test after changes.
- **Risk**: Multi-monitor fixes break single-monitor assumptions.
  - **Mitigation**: Prefer helper-level tests over GUI-only validation and preserve first-screen fallback behavior.

---

### Phase 2: Settings Persistence and Application
**Dependencies**: Phase 1

#### Objectives
- Turn the existing `settings.rs` module from passive file storage into active app configuration.
- Keep v1 settings simple and file-based instead of introducing a heavy GUI settings window.

#### Deliverables
- [ ] Finalize the supported v1 settings set: screenshot directory, recording directory, default image format, recording FPS, and Windows auto-start.
- [ ] Load settings on startup and apply them where no explicit CLI override exists.
- [ ] Wire the tray "Settings" action to a stable edit/view flow for the config file.
- [ ] Make auto-start honor the persisted setting on Windows, including disable behavior.
- [ ] Add tests for settings serialization, partial JSON input, invalid JSON fallback, and override precedence.

#### Implementation Details
Primary files:
- `src/settings.rs`
- `src/main.rs`
- `src/capture.rs`
- `src/record.rs`
- `src/tray.rs`

Technical approach:
- Keep the current JSON file at the config-dir path unless validation complexity forces a dependency like `serde`.
- Add helper functions for "CLI args + persisted settings -> effective runtime settings".
- Avoid a native settings UI in this phase; the tray action can continue to open the settings file if the behavior is clear and documented.
- Split Windows registry behavior into helpers so enabling and disabling auto-start can be tested without invasive refactoring.

#### Acceptance Criteria
- [ ] Missing settings file falls back to defaults.
- [ ] Saved settings round-trip without losing supported fields.
- [ ] Explicit CLI options override persisted defaults.
- [ ] Recording defaults can be sourced from settings when CLI values are absent.
- [ ] Windows auto-start behavior follows the persisted toggle.

#### Test Plan
- **Unit Tests**: JSON parse/format helpers, default fallback, effective-settings resolution, Windows autorun command formatting helpers.
- **Integration Tests**: Read/write settings file in temp directories where feasible.
- **Manual Testing**: Open settings from tray, edit values, restart app, verify new defaults apply.

#### Rollback Strategy
Keep `settings.rs` as an isolated persistence module and revert the call sites in
`main`, `capture`, and `record` if settings application causes regressions.

#### Risks
- **Risk**: The manual JSON parser accepts malformed input unexpectedly.
  - **Mitigation**: Add narrow parser tests and fail back to defaults on ambiguous input.
- **Risk**: Auto-start disable flow becomes Windows-specific and brittle.
  - **Mitigation**: Keep registry operations in small helper functions and validate only the supported HKCU path.

---

### Phase 3: MCP Server Delivery
**Dependencies**: Phase 2

#### Objectives
- Add a real `--mcp` mode that exposes RustShot features to MCP clients without booting tray mode.
- Reuse existing capture and recording logic rather than creating a second feature stack.

#### Deliverables
- [ ] Add `--mcp` CLI flag and top-level mode routing.
- [ ] Create `src/mcp.rs` with stdin/stdout JSON-RPC handling.
- [ ] Implement MCP tools for `list_screens`, `screenshot`, `screenshot_region`, `start_recording`, `stop_recording`, and `open_screenshots_folder`.
- [ ] Maintain in-process recording session state for MCP start/stop flows.
- [ ] Add tests for request parsing, tool dispatch, error mapping, and mode separation.
- [ ] Update MCP docs and setup examples.

#### Implementation Details
Primary files:
- `src/mcp.rs`
- `src/main.rs`
- `src/capture.rs`
- `src/record.rs`
- `src/selection.rs`
- `mcp-server/README.md`
- `.mcp.json`
- `README.md`
- `README.ru.md`

Technical approach:
- Implement a small protocol layer for JSON-RPC request decoding and response encoding.
- Define a dispatcher that maps tool names to pure RustShot operations and returns serializable results.
- Keep `screenshot_region` explicitly interactive so clients know it requires user selection.
- Use structured JSON-RPC and MCP error responses instead of stringly stdout logging in server mode.
- Ensure `--mcp` never initializes the tray or global hotkeys.

#### Acceptance Criteria
- [ ] MCP clients can initialize the server and list available tools.
- [ ] Each shipped tool returns success data or a structured error response.
- [ ] Recording sessions can be started and stopped within the same MCP process.
- [ ] `--mcp` mode is isolated from tray-mode startup.
- [ ] README and `.mcp.json` examples match the implemented CLI contract.

#### Test Plan
- **Unit Tests**: JSON-RPC parsing, tool name dispatch, argument validation, error object generation.
- **Integration Tests**: Spawn the binary in `--mcp` mode and exercise a minimal initialize/list/call sequence where practical.
- **Manual Testing**: Connect from a local MCP client and verify screenshot and monitor-list tools end-to-end.

#### Rollback Strategy
Remove `--mcp` routing and the `src/mcp.rs` module while preserving the core
capture and record APIs unchanged.

#### Risks
- **Risk**: Interactive tools do not fit well into a request/response protocol.
  - **Mitigation**: Mark interactive tools clearly in documentation and return explicit status/result payloads.
- **Risk**: Recording state becomes inconsistent across MCP calls.
  - **Mitigation**: Keep a single session owner in process and return deterministic errors for duplicate start/stop calls.

---

### Phase 4: Release and Packaging Hardening
**Dependencies**: Phase 3

#### Objectives
- Make tagged Windows releases reproducible and aligned with the actual binary layout and installer outputs.
- Reduce release drift between workflow, installer, and README instructions.

#### Deliverables
- [ ] Verify the GitHub Actions release workflow for build, artifact upload, and GitHub Release publishing.
- [ ] Align `installer.iss` inputs and output names with the workflow assumptions.
- [ ] Confirm the portable executable and installer artifact paths are stable.
- [ ] Document Windows build and packaging prerequisites clearly.
- [ ] Add or update a release smoke checklist for install, launch, tray, and uninstall behavior.

#### Implementation Details
Primary files:
- `.github/workflows/release.yml`
- `installer.iss`
- `README.md`
- `README.ru.md`

Technical approach:
- Keep the release flow Windows-first and avoid premature multi-platform packaging work.
- Treat the Windows MSVC target as the supported release target unless requirements change explicitly.
- Keep workflow fixes small and traceable so CI failures map cleanly to either cargo, installer, or upload steps.

#### Acceptance Criteria
- [ ] A tag matching `v*` produces `rustshot.exe` and `RustShotSetup.exe`.
- [ ] Release assets are uploaded both as workflow artifacts and GitHub Release files.
- [ ] The workflow does not depend on a missing `Cargo.lock` contract or mismatched target path.
- [ ] Installer instructions in docs match the actual published artifacts.

#### Test Plan
- **Unit Tests**: None expected; validation is workflow and packaging oriented.
- **Integration Tests**: CI release workflow run on a test tag or controlled manual dispatch.
- **Manual Testing**: Install, launch, capture, record, exit, and uninstall on a Windows machine.

#### Rollback Strategy
Revert workflow and installer changes together so release behavior returns to the
last known-good tagged configuration.

#### Risks
- **Risk**: CI passes cargo build but fails installer generation.
  - **Mitigation**: Keep installer path assumptions explicit and test them with the built artifact layout.
- **Risk**: Release artifacts differ from README guidance.
  - **Mitigation**: Update docs in the same phase as workflow changes, not afterward.

---

### Phase 5: Documentation and Validation Closure
**Dependencies**: Phase 4

#### Objectives
- Bring all project documentation back into sync with implemented behavior.
- Produce a clean validation checklist that maps the spec to evidence before moving to Review.

#### Deliverables
- [ ] Update English and Russian user docs for actual CLI, tray, recording, settings, and MCP behavior.
- [ ] Update assistant-facing docs where behavior or file ownership changed.
- [ ] Document explicit Windows-only limits and non-goals.
- [ ] Reconcile remaining manual-only test areas and known gaps.
- [ ] Prepare a review-ready checklist that maps spec success criteria to tests or manual validation.

#### Implementation Details
Primary files:
- `README.md`
- `README.ru.md`
- `GEMINI.md`
- `CLAUDE.md`
- `CLAUDE.ru.md`

Technical approach:
- Treat documentation as executable interface surface: if a feature is not implemented, docs must say "planned" or omit it.
- Keep internal agent docs aligned with the actual important modules and current workflow.
- Do not create the final review document in this phase; only prepare the repository for the Review stage.

#### Acceptance Criteria
- [ ] Docs do not claim unimplemented behavior.
- [ ] Windows-only recording limitations are explicit everywhere they matter.
- [ ] MCP setup instructions can be copied directly from the docs.
- [ ] Manual smoke-test checklist exists for tray, hotkeys, recording, and release install paths.
- [ ] The repository is ready to enter SPIR Implement or Review work without plan ambiguity.

#### Test Plan
- **Unit Tests**: None.
- **Integration Tests**: None beyond build/test/lint verification.
- **Manual Testing**: Copy/paste docs on a clean Windows setup and validate each documented path once.

#### Rollback Strategy
Revert documentation-only commits independently if wording changes outpace implementation.

#### Risks
- **Risk**: Docs drift again while implementation continues.
  - **Mitigation**: Require doc updates in the same phase as feature work wherever possible.
- **Risk**: Manual validation gaps remain hidden.
  - **Mitigation**: Make the manual-only checklist explicit and keep it short enough to run before release.

## Dependency Map
```text
Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5
```

## Resource Requirements
### Development Resources
- **Engineers**: Rust contributor familiar with `clap`, `winit`, Windows-only APIs, and lightweight test extraction.
- **Environment**: Windows machine for tray, hotkey, recording, and installer validation; Linux CI coverage for non-Windows build/test sanity.

### Infrastructure
- GitHub Actions for build, test, clippy, and release.
- Inno Setup available in the Windows release workflow.
- No database, network service, or external runtime required.

## Integration Points
### External Systems
- **Windows Media Foundation**
  - **Integration Type**: Native multimedia API
  - **Phase**: Phase 1, Phase 4
  - **Fallback**: Recording remains Windows-only and should fail clearly when unavailable
- **Windows Registry (HKCU Run)**
  - **Integration Type**: Native OS configuration
  - **Phase**: Phase 2
  - **Fallback**: Leave auto-start disabled and surface a clear error if registry writes fail
- **MCP Clients (Claude Code, Kiro, others)**
  - **Integration Type**: JSON-RPC over stdin/stdout
  - **Phase**: Phase 3
  - **Fallback**: Standard CLI and tray modes remain fully usable

### Internal Systems
- **Capture pipeline**
  - **Integration Type**: Module API reuse
  - **Phase**: Phase 1, Phase 2, Phase 3
  - **Fallback**: Keep module boundaries stable and avoid duplicate implementations
- **Selection overlay**
  - **Integration Type**: Child-process interactive flow
  - **Phase**: Phase 1, Phase 3
  - **Fallback**: Interactive capture remains tray/CLI-only if MCP interactive flow proves too fragile

## Risk Analysis
### Technical Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Windows-only tray/hotkey behavior regresses during cleanup | M | H | Keep changes local and run Windows smoke tests | Implementer |
| Manual JSON settings parser is too brittle for future fields | M | M | Add narrow tests and switch to `serde` only if current parser becomes unsafe to extend | Implementer |
| MCP recording lifecycle becomes hard to reason about | M | H | Keep one process-owned session and deterministic duplicate-call errors | Implementer |
| Release workflow diverges from installer layout | M | H | Update workflow and installer together and verify artifact paths | Implementer |

### Schedule Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Windows manual validation is delayed | M | M | Front-load core smoke tests after each Windows-heavy phase | Implementer |
| MCP scope expands beyond the current repo needs | M | M | Ship a minimal server over existing capabilities before adding extras | Implementer |

## Validation Checkpoints
1. **After Phase 1**: Run `cargo test`, confirm primary-monitor capture default, and verify tray capture/record toggle manually on Windows.
2. **After Phase 2**: Verify settings round-trip, restart behavior, and CLI-overrides-settings precedence.
3. **After Phase 3**: Run an MCP initialize/list/call smoke test from a local client.
4. **After Phase 4**: Validate release workflow artifacts and installer flow on Windows.
5. **Before Production**: Re-run build, test, clippy, release smoke checklist, and doc sanity pass.

## Monitoring and Observability
### Metrics to Track
- CI pass/fail for `fmt`, `test`, `clippy`, and release workflow jobs.
- Manual smoke-test completion for tray, hotkey, selection, recording, and installer paths.

### Logging Requirements
- Keep user-facing runtime errors human-readable through `Result<_, String>`.
- MCP mode must emit structured protocol errors instead of ad hoc stdout text.
- Release workflow failures must identify whether cargo build, installer generation, or upload failed.

### Alerting
- GitHub Actions failures are the primary alerting mechanism for build, lint, test, and release regressions.

## Documentation Updates Required
- [x] User guides
- [x] Configuration guides
- [x] MCP setup documentation
- [x] Assistant-facing architecture notes
- [ ] Review document during the later SPIR Review phase

## Post-Implementation Tasks
- [ ] Windows manual performance sanity check for capture and selection responsiveness
- [ ] Final release smoke test using the installer artifact
- [ ] Review whether the manual settings parser should remain or be replaced
- [ ] Map spec success criteria to evidence before opening review work

## Expert Review
**Date**: 2026-03-13
**Model**: GPT-5 Codex
**Key Feedback**:
- Finish the already-visible core fixes before adding new surfaces like MCP.
- Treat `settings.rs` as a lightweight configuration layer, not a GUI project.
- Keep MCP as a thin adapter over existing RustShot behaviors.
- Tie release workflow changes to artifact and documentation validation in the same plan.

**Plan Adjustments**:
- Added a dedicated core-hardening phase ahead of settings and MCP work.
- Split settings from MCP so runtime defaults are settled before remote control is added.
- Added a final documentation/validation closure phase instead of leaving doc sync implicit.

## Approval
- [ ] Technical Lead Review
- [ ] Engineering Manager Approval
- [ ] Resource Allocation Confirmed
- [ ] Expert AI Consultation Complete

## Change Log
| Date | Change | Reason | Author |
|------|--------|--------|--------|
| 2026-03-13 | Replaced draft roadmap with full SPIR/CODEV plan structure | Needed a complete plan tied to current repo scope and the approved spec | Codex |

## Notes
- This plan intentionally references the existing spec and test-plan artifacts instead of creating new specification scope.
- The filename does not yet mirror the spec filename exactly; this document is acting as the active repository roadmap draft until the project decides whether to normalize artifact naming.
- The plan assumes Windows remains the primary supported platform for tray mode, hotkeys, and recording.

---

## Amendment History

This section tracks all TICK amendments to this plan. TICKs modify both the
spec and plan together as an atomic unit.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
