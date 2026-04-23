# Specification: RustShot Release and Distribution Hardening

## Metadata
- **ID**: spec-2026-03-13-release-and-distribution-hardening
- **Status**: draft
- **Created**: 2026-03-13

## Clarifying Questions Asked
1. **Q**: What should a successful RustShot release produce?
   **A**: At minimum a portable `rustshot.exe` and an installer `RustShotSetup.exe`.
2. **Q**: Which platform is the release target?
   **A**: Windows is the primary release target.
3. **Q**: Should release work include documentation and installer validation?
   **A**: Yes. Artifact generation without matching docs and install validation is incomplete.
4. **Q**: Is a new packaging system required?
   **A**: No. The current GitHub Actions plus Inno Setup path should be hardened, not replaced.

## Problem Statement
RustShot already has a Windows release workflow and an Inno Setup installer, but
release quality depends on more than "cargo build succeeded." Artifact paths,
installer assumptions, workflow uploads, release notes, and documentation can
drift independently. That drift creates a high risk of tagged releases that look
successful in CI but fail in actual user installation or distribution flows.

The repository also already contains release-related edits in flight, which
increases the need for a written contract rather than ad hoc fixes.

## Current State
Today the repository has:
- A GitHub Actions workflow for tagged Windows releases.
- `installer.iss` for building `RustShotSetup.exe`.
- Upload steps for both the portable executable and installer.
- Top-level docs that describe installation and usage.

Current limitations:
- Artifact paths and installer expectations are easy to get out of sync.
- There is no explicit smoke checklist for install, launch, capture, and uninstall.
- Documentation may lag behind workflow or packaging changes.
- There is no clear contract for what "release-ready" means beyond workflow success.

## Desired State
RustShot should have a repeatable Windows release path where:
- A `v*` tag produces the expected artifacts.
- Installer and portable outputs match the workflow contract exactly.
- Release notes and uploaded assets reflect the real shipped deliverables.
- Documentation tells users what to download and how to verify installation.
- A lightweight manual smoke checklist exists for final confidence.

## Stakeholders
- **Primary Users**: Windows users downloading RustShot binaries and installer packages.
- **Secondary Users**: Maintainers preparing releases and troubleshooting packaging failures.
- **Technical Team**: Contributors maintaining `.github/workflows/release.yml`, `installer.iss`, and release docs.
- **Business Owners**: Project maintainers responsible for public distribution quality.

## Success Criteria
- [ ] Tagged Windows releases produce both `rustshot.exe` and `RustShotSetup.exe`.
- [ ] Workflow artifact paths, GitHub Release assets, and installer output names are aligned.
- [ ] The installer launches and installs a working RustShot build on Windows.
- [ ] Release docs match actual artifact names and installation flow.
- [ ] A release smoke checklist exists and covers install, launch, capture, recording, and uninstall.
- [ ] Release workflow changes are validated without introducing unrelated packaging complexity.

## Constraints
### Technical Constraints
- Windows remains the primary distribution target.
- The release workflow currently uses GitHub Actions and Inno Setup.
- RustShot is still a single-binary desktop app; packaging should respect that.
- CI cannot fully replace manual desktop validation.

### Business Constraints
- The project should keep its current release tooling unless there is a strong reason to replace it.
- Packaging hardening should not block normal development indefinitely.
- Documentation must remain simple enough for end users.

## Assumptions
- GitHub Actions is the canonical release automation path.
- Inno Setup remains acceptable for Windows installer generation.
- Tagged release flow is the supported publish path.
- Manual smoke testing will remain part of the final release process.

## Solution Approaches

### Approach 1: Harden the Existing GitHub Actions and Inno Setup Flow
**Description**: Keep the current release pipeline, artifact names, and installer
technology, but explicitly align paths, validation, and docs.

**Pros**:
- Smallest operational change.
- Builds on repository code already present.
- Easy to reason about and document.

**Cons**:
- Still depends on Windows-specific tooling.
- Manual smoke testing remains necessary.

**Estimated Complexity**: Low
**Risk Level**: Low

### Approach 2: Replace Packaging with a New Windows Distribution Stack
**Description**: Move to a different installer or release system entirely.

**Pros**:
- Could improve future flexibility.

**Cons**:
- Larger migration cost.
- Not justified by the current repository state.
- Creates extra release risk during transition.

**Estimated Complexity**: High
**Risk Level**: High

### Approach 3: Keep Release Quality Informal
**Description**: Rely on the workflow and ad hoc maintainer checks without a
formal artifact contract.

**Pros**:
- No additional planning work.

**Cons**:
- High risk of broken public releases.
- Harder to debug failures and drift.

**Estimated Complexity**: Low
**Risk Level**: High

## Open Questions

### Critical (Blocks Progress)
- [ ] None.

### Important (Affects Design)
- [ ] Should the release workflow also preserve zipped portable bundles, or is the raw executable sufficient for v1?
- [ ] Should release smoke validation require recording on every release, or only on milestone releases?

### Nice-to-Know (Optimization)
- [ ] Should release notes eventually include a generated summary of supported platforms and known limitations?

## Performance Requirements
- **Build Reliability**: Release builds must complete successfully under the supported Windows target configuration.
- **Artifact Stability**: Output file names and paths should remain stable across releases.
- **Installer Behavior**: Installation should complete without requiring elevated complexity beyond current assumptions.
- **Recovery**: Packaging failures should be attributable to a clear stage in the workflow.

## Security Considerations
- Installer contents should include only the intended executable and packaged assets.
- Release workflow should not depend on unnecessary write scopes or ad hoc secret usage beyond the normal GitHub release path.
- Auto-start or registry behavior in the installed app should remain user-level and documented.
- Public artifacts must correspond exactly to what the workflow produced.

## Test Scenarios
### Functional Tests
1. Trigger a release build and verify both portable and installer artifacts are produced.
2. Install RustShot from the generated installer and verify the app launches.
3. Verify tray capture, CLI capture, and recording in the installed build on Windows.
4. Uninstall RustShot and verify the expected cleanup path.

### Non-Functional Tests
1. Verify workflow logs clearly identify the failing stage when packaging breaks.
2. Verify docs point to the right file names and install flow.
3. Verify release assets attached to GitHub match workflow outputs exactly.

## Dependencies
- **External Services**: GitHub Actions, GitHub Releases.
- **Internal Systems**: `installer.iss`, release workflow, top-level docs, Windows build target.
- **Libraries/Frameworks**: Rust toolchain, Inno Setup.

## References
- [001-feature-roadmap umbrella plan](../plans/001-feature-roadmap.md)
- `.github/workflows/release.yml`
- `installer.iss`
- `README.md`

## Risks and Mitigation
| Risk | Probability | Impact | Mitigation Strategy |
|------|-------------|--------|---------------------|
| Workflow and installer paths drift apart | Medium | High | Align and validate the artifact contract in one phase |
| CI succeeds but installed app is broken | Medium | High | Require a manual install-and-run smoke checklist |
| Docs reference obsolete artifact names | Medium | Medium | Update docs in the same phase as workflow changes |

## Expert Consultation
**Date**: 2026-03-13
**Models Consulted**: GPT-5 Codex
**Sections Updated**:
- **Desired State**: Added an explicit artifact contract and manual smoke requirement.
- **Solution Approaches**: Selected hardening of the existing release path.
- **Security Considerations**: Added public artifact correspondence requirements.

Note: All consultation feedback has been incorporated directly into the relevant sections above.

## Approval
- [ ] Technical Lead Review
- [ ] Product Owner Review
- [ ] Stakeholder Sign-off
- [ ] Expert AI Consultation Complete

## Notes
- This spec is about repeatability and confidence, not a packaging technology migration.

---

## Amendments

This section tracks all TICK amendments to this specification. TICKs are
lightweight changes that refine an existing spec rather than creating a new one.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
