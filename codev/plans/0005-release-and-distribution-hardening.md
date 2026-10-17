# Plan: RustShot Release and Distribution Hardening

## Metadata
- **ID**: plan-2026-03-13-release-and-distribution-hardening
- **Status**: draft
- **Specification**: [0005-release-and-distribution-hardening](../specs/0005-release-and-distribution-hardening.md)
- **Created**: 2026-03-13

## Executive Summary
This plan hardens the current Windows release path instead of replacing it. The
work is structured around one artifact contract: a tagged release should produce
and publish a portable executable and an installer whose names, paths, and
documentation all agree.

## Success Metrics
- [ ] All specification criteria are satisfied.
- [ ] Release workflow artifact paths are stable and correct.
- [ ] Installer generation succeeds from the same build outputs used for release assets.
- [ ] Documentation and smoke checklist match the real release flow.
- [ ] Final validation covers install, launch, capture, recording, and uninstall on Windows.

## Phases (Machine Readable)

```json
{
  "phases": [
    {"id": "phase_1_contract", "title": "Artifact Contract and Workflow Alignment"},
    {"id": "phase_2_installer", "title": "Installer Path and Packaging Hardening"},
    {"id": "phase_3_docs_checklist", "title": "Release Documentation and Smoke Checklist"},
    {"id": "phase_4_validation", "title": "Tagged Release Validation"}
  ]
}
```

## Phase Breakdown

### Phase 1: Artifact Contract and Workflow Alignment
**Dependencies**: None

#### Objectives
- Define exactly which release artifacts RustShot publishes.
- Align workflow paths and target assumptions with that contract.

#### Deliverables
- [ ] Verify the supported target triple and build output paths.
- [ ] Verify artifact names and upload paths in `.github/workflows/release.yml`.
- [ ] Remove stale assumptions about lockfile or target layout if present.

#### Implementation Details
Primary files:
- `.github/workflows/release.yml`

Technical approach:
- Keep the Windows MSVC target as the supported release target.
- Ensure all workflow steps reference the same compiled binary output path.

#### Acceptance Criteria
- [ ] Workflow uses one consistent release binary path.
- [ ] Both portable and installer artifacts are uploaded correctly.
- [ ] All tests or workflow checks associated with the release path pass.

#### Test Plan
- **Unit Tests**: None.
- **Integration Tests**: Manual workflow dispatch or test tag run.
- **Manual Testing**: Inspect workflow logs and produced artifacts.

#### Rollback Strategy
Revert workflow changes as a unit to the last known-good release configuration.

#### Risks
- **Risk**: Path changes fix one artifact but break another.
  - **Mitigation**: Treat build, copy, upload, and release steps as one artifact chain to validate together.

---

### Phase 2: Installer Path and Packaging Hardening
**Dependencies**: Phase 1

#### Objectives
- Ensure the installer consumes the right binary and emits the expected output.
- Prevent silent drift between installer script and workflow layout.

#### Deliverables
- [ ] Verify `installer.iss` input paths and output name assumptions.
- [ ] Align installer output name with workflow upload expectations.
- [ ] Confirm install directory, shortcut, and uninstall behavior assumptions.

#### Implementation Details
Primary files:
- `installer.iss`
- `.github/workflows/release.yml`

Technical approach:
- Keep Inno Setup and validate only the current path contract.
- Avoid expanding installer scope with unrelated UX changes during hardening.

#### Acceptance Criteria
- [ ] Installer builds from the workflow-prepared binary.
- [ ] Output installer name matches `RustShotSetup.exe`.
- [ ] Install/uninstall path assumptions are documented and validated.

#### Test Plan
- **Unit Tests**: None.
- **Integration Tests**: Installer generation in CI.
- **Manual Testing**: Install and uninstall the built package on Windows.

#### Rollback Strategy
Revert installer and workflow path changes together.

#### Risks
- **Risk**: Installer script lags behind workflow changes.
  - **Mitigation**: Keep both files in scope for the same phase and review them together.

---

### Phase 3: Release Documentation and Smoke Checklist
**Dependencies**: Phase 2

#### Objectives
- Make release consumption and validation explicit.
- Reduce ambiguity for both maintainers and users.

#### Deliverables
- [ ] Update installation/download docs to match real artifacts.
- [ ] Add a concise Windows release smoke checklist.
- [ ] Document what should be verified after installation.

#### Implementation Details
Primary files:
- `README.md`
- `README.ru.md`
- `GEMINI.md`

Technical approach:
- Keep the checklist short and operational.
- Document only artifacts that the workflow actually publishes.

#### Acceptance Criteria
- [ ] Docs name the correct release assets.
- [ ] The smoke checklist covers install, launch, capture, recording, and uninstall.
- [ ] Docs do not promise unsupported packaging flows.

#### Test Plan
- **Unit Tests**: None.
- **Integration Tests**: None beyond workflow and app test suite.
- **Manual Testing**: Follow docs on a clean Windows machine and verify each step.

#### Rollback Strategy
Revert doc-only changes independently if needed.

#### Risks
- **Risk**: Release docs over-document untested paths.
  - **Mitigation**: Limit docs to the paths explicitly validated in Phase 4.

---

### Phase 4: Tagged Release Validation
**Dependencies**: Phase 3

#### Objectives
- Prove the hardened release path works end to end.
- Capture release-ready evidence before review.

#### Deliverables
- [ ] Run a tagged or equivalent release workflow validation.
- [ ] Validate the portable executable on Windows.
- [ ] Validate the installer flow on Windows.
- [ ] Record any remaining manual-only caveats.

#### Implementation Details
Primary files:
- `.github/workflows/release.yml`
- `installer.iss`
- top-level docs as evidence references

Technical approach:
- Use a controlled tag or workflow dispatch as the proof point.
- Treat workflow success as necessary but not sufficient; manual smoke completion is required.

#### Acceptance Criteria
- [ ] Tagged validation produces both release artifacts.
- [ ] Installed RustShot launches and performs core capture behavior.
- [ ] Portable RustShot launches and performs core capture behavior.
- [ ] Final release checklist is complete.

#### Test Plan
- **Unit Tests**: None.
- **Integration Tests**: Full release workflow run.
- **Manual Testing**: Install, launch, capture, record, exit, and uninstall.

#### Rollback Strategy
Revert release hardening changes to the previous known-good tag behavior.

#### Risks
- **Risk**: Final validation reveals packaging issues late.
  - **Mitigation**: Keep checklist-driven validation and validate workflow and installer assumptions earlier in the plan.

## Dependency Map
```text
Phase 1 -> Phase 2 -> Phase 3 -> Phase 4
```

## Resource Requirements
### Development Resources
- **Engineers**: Contributor familiar with GitHub Actions, Windows packaging, and RustShot runtime behavior.
- **Environment**: Windows machine for install, launch, recording, and uninstall validation.

### Infrastructure
- GitHub Actions
- Inno Setup
- GitHub Releases

## Integration Points
### External Systems
- **GitHub Actions**
  - **Integration Type**: CI/CD workflow
  - **Phase**: All phases
  - **Fallback**: Manual validation can diagnose failures but does not replace CI
- **Inno Setup**
  - **Integration Type**: Windows installer tooling
  - **Phase**: Phase 2 and Phase 4
  - **Fallback**: Portable executable remains available, but installer quality is not considered complete

### Internal Systems
- **Release workflow and installer script**
  - **Integration Type**: Packaging contract
  - **Phase**: Phase 1 and Phase 2
  - **Fallback**: Revert to last known-good release contract

## Risk Analysis
### Technical Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Artifact paths drift between workflow and installer | M | H | Validate one artifact contract across both files | Implementer |
| CI success hides installer or runtime breakage | M | H | Require manual smoke validation in the final phase | Implementer |
| Docs and release assets drift after workflow changes | M | M | Update docs in the same plan, not as follow-up work | Implementer |

### Schedule Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Tagged release validation takes longer than expected | M | M | Validate workflow and installer assumptions in earlier phases | Implementer |

## Validation Checkpoints
1. **After Phase 1**: Confirm workflow path and artifact contract are coherent.
2. **After Phase 2**: Confirm installer script and workflow output paths align.
3. **After Phase 3**: Follow the documented checklist on Windows.
4. **Before Production**: Run a tagged validation and archive the release evidence.

## Monitoring and Observability
### Metrics to Track
- Release workflow pass/fail.
- Installer generation pass/fail.
- Manual checklist completion for each release candidate.

### Logging Requirements
- Workflow logs must clearly identify build, copy, installer, upload, and release stages.
- Packaging docs should capture any manual-only caveats discovered in validation.

### Alerting
- GitHub Actions failures are the primary release alerting path.

## Documentation Updates Required
- [x] User guides
- [x] Configuration guides
- [x] Release instructions
- [ ] Review document during SPIR Review

## Post-Implementation Tasks
- [ ] Evaluate whether zipped portable bundles are worth adding later
- [ ] Evaluate whether release notes should summarize platform limits automatically

## Expert Review
**Date**: 2026-03-13
**Model**: GPT-5 Codex
**Key Feedback**:
- Keep the current release stack and harden it rather than replacing it.
- Treat workflow, installer, and docs as one artifact contract.
- Require manual desktop smoke testing even after successful CI.

**Plan Adjustments**:
- Added a final tagged validation phase instead of ending at workflow updates.

## Approval
- [ ] Technical Lead Review
- [ ] Engineering Manager Approval
- [ ] Resource Allocation Confirmed
- [ ] Expert AI Consultation Complete

## Change Log
| Date | Change | Reason | Author |
|------|--------|--------|--------|
| 2026-03-13 | Created detailed release hardening implementation plan | Needed a matching execution artifact for release quality work | Codex |

## Notes
- This plan is release-quality work, not a packaging-stack migration plan.

---

## Amendment History

This section tracks all TICK amendments to this plan. TICKs modify both the
spec and plan together as an atomic unit.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
