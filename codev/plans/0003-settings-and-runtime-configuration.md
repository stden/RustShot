# Plan: RustShot Settings and Runtime Configuration

## Metadata
- **ID**: plan-2026-03-13-settings-and-runtime-configuration
- **Status**: draft
- **Specification**: [0003-settings-and-runtime-configuration](../specs/0003-settings-and-runtime-configuration.md)
- **Created**: 2026-03-13

## Executive Summary
This plan implements a lightweight but complete settings contract for RustShot.
The selected approach is to keep file-based configuration, apply it as the
canonical source of runtime defaults, and defer any native settings UI until the
underlying semantics are stable.

## Success Metrics
- [ ] All specification criteria are satisfied.
- [ ] Settings round-trip tests cover full, partial, and malformed input.
- [ ] Runtime defaults are applied from settings where explicit CLI input is absent.
- [ ] Windows auto-start behavior follows the saved setting.
- [ ] Documentation reflects file location, precedence rules, and platform-specific behavior.

## Phases (Machine Readable)

```json
{
  "phases": [
    {"id": "phase_1_contract", "title": "Settings Contract and Validation"},
    {"id": "phase_2_runtime_defaults", "title": "Runtime Default Resolution"},
    {"id": "phase_3_autostart", "title": "Windows Auto-Start Application"},
    {"id": "phase_4_docs_validation", "title": "Documentation and Manual Validation"}
  ]
}
```

## Phase Breakdown

### Phase 1: Settings Contract and Validation
**Dependencies**: None

#### Objectives
- Finalize the shape and semantics of the persisted settings file.
- Make malformed or partial input safe by design.

#### Deliverables
- [ ] Confirm the supported v1 fields and defaults.
- [ ] Add helper coverage for parsing, formatting, and fallback behavior.
- [ ] Decide normalization behavior for invalid string-based values such as `default_format`.

#### Implementation Details
Primary files:
- `src/settings.rs`

Technical approach:
- Keep the current JSON-file location and manual parser unless tests prove it insufficient.
- Add helper functions for field normalization instead of scattering validation logic.

#### Acceptance Criteria
- [ ] Full valid settings round-trip cleanly.
- [ ] Partial settings input fills missing fields with defaults.
- [ ] Malformed settings input falls back safely.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: Parse/format helpers, malformed JSON fallback, invalid field normalization, path escaping.
- **Integration Tests**: Temp-file read/write where useful.
- **Manual Testing**: Open the settings file, edit it, and verify no crash on next startup.

#### Rollback Strategy
Revert `src/settings.rs` helper changes without touching runtime call sites yet.

#### Risks
- **Risk**: Parser complexity grows unexpectedly.
  - **Mitigation**: Switch to `serde` only if helper-level validation becomes unmaintainable.

---

### Phase 2: Runtime Default Resolution
**Dependencies**: Phase 1

#### Objectives
- Make settings the default runtime source while preserving CLI precedence.
- Avoid duplicating default logic across modules.

#### Deliverables
- [ ] Introduce an effective-settings resolution path for startup and CLI modes.
- [ ] Apply screenshot and recording defaults from settings.
- [ ] Preserve explicit CLI override behavior.

#### Implementation Details
Primary files:
- `src/main.rs`
- `src/capture.rs`
- `src/record.rs`
- `src/settings.rs`

Technical approach:
- Add a small resolver that combines CLI args and loaded settings into effective runtime values.
- Keep CLI parsing unchanged where possible; adapt only post-parse behavior.

#### Acceptance Criteria
- [ ] Screenshot defaults come from settings when CLI values are absent.
- [ ] Recording FPS defaults come from settings when CLI values are absent.
- [ ] Explicit CLI values always win.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: Effective-settings resolution, override precedence, default-format mapping.
- **Integration Tests**: CLI-mode path with injected capture/record helpers.
- **Manual Testing**: Change defaults in settings, relaunch, and verify behavior changes without CLI flags.

#### Rollback Strategy
Revert runtime call-site integration but keep the validated settings file contract.

#### Risks
- **Risk**: Settings integration changes behavior in silent ways.
  - **Mitigation**: Add explicit precedence tests and document the rules.

---

### Phase 3: Windows Auto-Start Application
**Dependencies**: Phase 2

#### Objectives
- Make auto-start a real persisted preference on Windows.
- Ensure enable and disable paths are both supported.

#### Deliverables
- [ ] Refactor startup registration into helpers that can enable and disable the Run entry.
- [ ] Apply the saved auto-start setting at startup on Windows.
- [ ] Preserve non-Windows no-op behavior.

#### Implementation Details
Primary files:
- `src/main.rs`
- `src/settings.rs`

Technical approach:
- Extract registry access helpers for set/remove operations.
- Keep the target registry location at `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

#### Acceptance Criteria
- [ ] Auto-start enabled writes the expected quoted executable path.
- [ ] Auto-start disabled removes or avoids creating the entry.
- [ ] Non-Windows builds still compile cleanly.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: Quoted autorun command, helper selection logic, Windows-specific path formatting.
- **Integration Tests**: None required for registry mutation in CI.
- **Manual Testing**: Toggle auto-start, relaunch, and inspect Run entry on Windows.

#### Rollback Strategy
Revert registry helper changes as a unit if enable/disable behavior proves unstable.

#### Risks
- **Risk**: Registry mutation code becomes hard to test.
  - **Mitigation**: Keep pure formatting helpers separate from side-effecting calls.

---

### Phase 4: Documentation and Manual Validation
**Dependencies**: Phase 3

#### Objectives
- Make the settings behavior discoverable and supportable.
- Close the gap between code and user expectations.

#### Deliverables
- [ ] Document settings file location and supported keys.
- [ ] Document CLI precedence over persisted defaults.
- [ ] Document Windows-only auto-start behavior and limitations.

#### Implementation Details
Primary files:
- `README.md`
- `README.ru.md`
- `GEMINI.md`

#### Acceptance Criteria
- [ ] Docs match the implemented settings contract.
- [ ] Manual validation steps are recorded.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: None beyond code-driven corrections.
- **Integration Tests**: Full `cargo test`.
- **Manual Testing**: Fresh settings file creation, manual edit flow, restart verification.

#### Rollback Strategy
Revert doc-only changes independently if needed.

#### Risks
- **Risk**: Docs over-promise live settings reload or native UI behavior.
  - **Mitigation**: State explicitly that the v1 tray action opens the config file.

## Dependency Map
```text
Phase 1 -> Phase 2 -> Phase 3 -> Phase 4
```

## Resource Requirements
### Development Resources
- **Engineers**: Rust contributor familiar with startup flow, Windows registry helpers, and small pure-function test extraction.
- **Environment**: Windows machine for auto-start validation and any desktop integration checks.

### Infrastructure
- No new services or build systems.
- Existing test and CI setup only.

## Integration Points
### External Systems
- **Windows Registry**
  - **Integration Type**: Native OS configuration
  - **Phase**: Phase 3
  - **Fallback**: Fail gracefully and preserve app startup without elevation

### Internal Systems
- **Settings module**
  - **Integration Type**: Runtime dependency
  - **Phase**: Phase 1, Phase 2, Phase 3
  - **Fallback**: Default settings if load fails

## Risk Analysis
### Technical Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Manual parser remains brittle | M | M | Tight tests and normalization helpers | Implementer |
| Precedence rules become inconsistent | M | H | One resolver helper and explicit tests | Implementer |
| Auto-start toggling is difficult to validate in CI | H | M | Keep manual Windows validation in checkpoints | Implementer |

### Schedule Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Windows validation is delayed | M | M | Run auto-start smoke tests immediately after Phase 3 | Implementer |

## Validation Checkpoints
1. **After Phase 1**: Validate parsing and fallback behavior with tests.
2. **After Phase 2**: Validate settings-backed defaults and CLI precedence.
3. **After Phase 3**: Validate Windows auto-start enable/disable manually.
4. **Before Production**: Re-run docs sanity pass and full test suite.

## Monitoring and Observability
### Metrics to Track
- Test pass/fail for settings parsing and runtime-resolution tests.
- Manual completion of the Windows auto-start checklist.

### Logging Requirements
- Settings load/save failures must remain human-readable.
- Invalid settings should fall back safely without noisy crash output.

### Alerting
- GitHub Actions failures remain the main alert path.

## Documentation Updates Required
- [x] User guides
- [x] Configuration guides
- [x] Assistant-facing notes
- [ ] Review document during SPIR Review

## Post-Implementation Tasks
- [ ] Reassess whether a native settings UI is still needed
- [ ] Reassess whether `serde` should replace the manual parser

## Expert Review
**Date**: 2026-03-13
**Model**: GPT-5 Codex
**Key Feedback**:
- Stabilize settings semantics before building a richer UI.
- Keep CLI precedence explicit and testable.
- Treat auto-start as a reversible user preference.

**Plan Adjustments**:
- Separated parser validation, runtime integration, and Windows startup behavior into distinct phases.

## Approval
- [ ] Technical Lead Review
- [ ] Engineering Manager Approval
- [ ] Resource Allocation Confirmed
- [ ] Expert AI Consultation Complete

## Change Log
| Date | Change | Reason | Author |
|------|--------|--------|--------|
| 2026-03-13 | Created detailed settings implementation plan | Needed a matching execution artifact for the settings spec | Codex |

## Notes
- This plan assumes the v1 settings UX remains file-backed.

---

## Amendment History

This section tracks all TICK amendments to this plan. TICKs modify both the
spec and plan together as an atomic unit.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
