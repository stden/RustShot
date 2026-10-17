# Plan: RustShot MCP Server Integration

## Metadata
- **ID**: plan-2026-03-13-mcp-server-integration
- **Status**: draft
- **Specification**: [0004-mcp-server-integration](../specs/0004-mcp-server-integration.md)
- **Created**: 2026-03-13

## Executive Summary
This plan adds a real MCP server mode to the existing `rustshot` binary. The
chosen implementation is a focused `--mcp` mode that reuses existing capture,
selection, and recording logic while staying isolated from tray and hotkey
startup.

## Success Metrics
- [ ] All specification criteria are satisfied.
- [ ] `--mcp` mode initializes without tray or hotkey side effects.
- [ ] Read-only and mutating tools both return structured responses.
- [ ] Recording lifecycle is consistent within one server process.
- [ ] Documentation and example config files match the implemented contract.

## Phases (Machine Readable)

```json
{
  "phases": [
    {"id": "phase_1_mode_protocol", "title": "MCP Mode Routing and Protocol Skeleton"},
    {"id": "phase_2_read_tools", "title": "Read-Only MCP Tool Delivery"},
    {"id": "phase_3_interactive_recording", "title": "Interactive and Recording MCP Tools"},
    {"id": "phase_4_docs_validation", "title": "Documentation and MCP Validation"}
  ]
}
```

## Phase Breakdown

### Phase 1: MCP Mode Routing and Protocol Skeleton
**Dependencies**: None

#### Objectives
- Add a dedicated server mode and a small JSON-RPC skeleton.
- Ensure server mode is isolated from tray mode from the first commit.

#### Deliverables
- [ ] Add `--mcp` CLI flag and top-level mode routing.
- [ ] Create `src/mcp.rs` with request/response data structures and the server loop.
- [ ] Add parsing and structured error helpers.

#### Implementation Details
Primary files:
- `src/main.rs`
- `src/mcp.rs`

Technical approach:
- Parse args normally, then branch into MCP mode before tray startup.
- Keep protocol handling synchronous for v1.
- Model errors explicitly so malformed requests never crash the process.

#### Acceptance Criteria
- [ ] `rustshot --mcp` starts server mode only.
- [ ] Invalid requests produce structured protocol errors.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: Mode routing, request parsing, error serialization.
- **Integration Tests**: Spawn `--mcp` and send minimal initialize/list requests where practical.
- **Manual Testing**: Start the server from a shell and inspect response shapes.

#### Rollback Strategy
Remove `--mcp` routing and `src/mcp.rs` together, leaving other modes untouched.

#### Risks
- **Risk**: MCP mode leaks tray initialization through shared startup code.
  - **Mitigation**: Route into server mode before any tray or hotkey construction.

---

### Phase 2: Read-Only MCP Tool Delivery
**Dependencies**: Phase 1

#### Objectives
- Ship the safe and straightforward tool surface first.
- Reuse existing helper logic for listing screens, capturing screenshots, and opening folders.

#### Deliverables
- [ ] Implement `list_screens`.
- [ ] Implement `screenshot`.
- [ ] Implement `open_screenshots_folder`.
- [ ] Add argument validation and response schemas for these tools.

#### Implementation Details
Primary files:
- `src/mcp.rs`
- `src/capture.rs`
- `src/main.rs`

Technical approach:
- Use existing capture helpers where possible.
- Return file-path-based payloads rather than binary blobs.
- Keep open-folder behavior restricted to known RustShot locations.

#### Acceptance Criteria
- [ ] Read-only tools return valid success payloads.
- [ ] Invalid arguments return structured errors.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: Tool dispatch, argument validation, response shapes.
- **Integration Tests**: End-to-end MCP calls for `list_screens` and `screenshot`.
- **Manual Testing**: Run the three tools from a local MCP client.

#### Rollback Strategy
Remove tool registration and dispatch entries while keeping the protocol skeleton.

#### Risks
- **Risk**: Existing helpers are too coupled to CLI output formatting.
  - **Mitigation**: Add small adapter helpers rather than duplicating logic.

---

### Phase 3: Interactive and Recording MCP Tools
**Dependencies**: Phase 2

#### Objectives
- Add the tools that depend on user interaction or mutable process state.
- Keep the state model intentionally simple.

#### Deliverables
- [ ] Implement `screenshot_region`.
- [ ] Implement `start_recording`.
- [ ] Implement `stop_recording`.
- [ ] Maintain single-session recording state within the server process.

#### Implementation Details
Primary files:
- `src/mcp.rs`
- `src/selection.rs`
- `src/record.rs`
- `src/settings.rs`

Technical approach:
- Reuse the existing interactive selection and recording logic.
- Return explicit errors for duplicate `start_recording` and `stop_recording` calls.
- Support optional parameter overrides only if they do not complicate the first release.

#### Acceptance Criteria
- [ ] Interactive region capture completes with a clear result or cancellation error.
- [ ] Recording start and stop work in one process lifecycle.
- [ ] Unsupported-platform behavior is explicit.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: Recording session state transitions, parameter validation, cancellation/error mapping.
- **Integration Tests**: Process-level recording lifecycle on Windows where feasible.
- **Manual Testing**: Region capture and recording start/stop via a real MCP client.

#### Rollback Strategy
Remove only the interactive and recording tools while preserving the protocol and read-only tools.

#### Risks
- **Risk**: Recording state and interactive selection become hard to reason about together.
  - **Mitigation**: Keep one active recording session and simple blocking behavior for interactive tools.

---

### Phase 4: Documentation and MCP Validation
**Dependencies**: Phase 3

#### Objectives
- Make MCP setup copy-pasteable and trustworthy.
- Close the loop between code, sample config, and docs.

#### Deliverables
- [ ] Update `mcp-server/README.md`.
- [ ] Update `.mcp.json` example if needed.
- [ ] Update top-level docs to mention MCP support and its limits.

#### Implementation Details
Primary files:
- `mcp-server/README.md`
- `.mcp.json`
- `README.md`
- `README.ru.md`

#### Acceptance Criteria
- [ ] Docs match the actual `--mcp` interface.
- [ ] Tool list and parameter expectations are documented.
- [ ] Manual MCP validation steps are recorded.
- [ ] All tests pass.

#### Test Plan
- **Unit Tests**: None beyond code-driven corrections.
- **Integration Tests**: Full test suite plus a minimal MCP subprocess smoke test.
- **Manual Testing**: Connect from a real MCP client using the documented config.

#### Rollback Strategy
Revert documentation-only changes independently if needed.

#### Risks
- **Risk**: Docs promise tool behavior that the protocol does not implement.
  - **Mitigation**: Generate docs directly from the delivered tool set and keep the v1 scope small.

## Dependency Map
```text
Phase 1 -> Phase 2 -> Phase 3 -> Phase 4
```

## Resource Requirements
### Development Resources
- **Engineers**: Rust contributor comfortable with command routing, JSON protocol handling, and existing RustShot capture/record helpers.
- **Environment**: Windows machine for interactive and recording validation plus any local MCP client.

### Infrastructure
- No new services required.
- Existing binary packaging path remains intact.

## Integration Points
### External Systems
- **MCP Client**
  - **Integration Type**: JSON-RPC over stdin/stdout
  - **Phase**: All phases
  - **Fallback**: Standard RustShot CLI and tray usage remains available

### Internal Systems
- **Capture, selection, and record modules**
  - **Integration Type**: Shared module API reuse
  - **Phase**: Phase 2 and Phase 3
  - **Fallback**: Restrict the tool set rather than duplicating behavior

## Risk Analysis
### Technical Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Server mode and tray mode are not cleanly separated | M | H | Route into `--mcp` before any tray startup | Implementer |
| Interactive tool semantics confuse client behavior | M | M | Document blocking behavior and explicit cancellations | Implementer |
| Recording session state causes edge-case errors | M | H | Limit v1 to one session per process and test state transitions | Implementer |

### Schedule Risks
| Risk | Probability | Impact | Mitigation | Owner |
|------|-------------|--------|------------|-------|
| Real-client MCP validation takes extra time | M | M | Keep a minimal local validation checklist and tool set | Implementer |

## Validation Checkpoints
1. **After Phase 1**: Confirm `--mcp` isolates startup and returns protocol errors correctly.
2. **After Phase 2**: Confirm read-only tools work from a subprocess test.
3. **After Phase 3**: Confirm interactive and recording tools work from a real client on Windows.
4. **Before Production**: Re-run doc-config examples and full tests.

## Monitoring and Observability
### Metrics to Track
- MCP subprocess smoke-test pass/fail.
- Recording lifecycle test pass/fail where supported.

### Logging Requirements
- Keep protocol errors structured.
- Avoid leaking ad hoc debug text into stdout in server mode.

### Alerting
- GitHub Actions and manual MCP validation remain the alerting path.

## Documentation Updates Required
- [x] User guides
- [x] Configuration guides
- [x] MCP setup docs
- [ ] Review document during SPIR Review

## Post-Implementation Tasks
- [ ] Evaluate whether MCP tool metadata should become code-generated
- [ ] Evaluate whether output payloads should later support richer metadata

## Expert Review
**Date**: 2026-03-13
**Model**: GPT-5 Codex
**Key Feedback**:
- Keep MCP as a thin adapter over existing RustShot functionality.
- Ship read-only tools before stateful tools.
- Separate server startup from tray startup at the first routing decision.

**Plan Adjustments**:
- Split protocol skeleton, read-only tools, and stateful tools into separate phases.

## Approval
- [ ] Technical Lead Review
- [ ] Engineering Manager Approval
- [ ] Resource Allocation Confirmed
- [ ] Expert AI Consultation Complete

## Change Log
| Date | Change | Reason | Author |
|------|--------|--------|--------|
| 2026-03-13 | Created detailed MCP implementation plan | Needed a matching execution artifact for MCP support | Codex |

## Notes
- This plan assumes v1 responses stay file-path based for screenshots and recordings.

---

## Amendment History

This section tracks all TICK amendments to this plan. TICKs modify both the
spec and plan together as an atomic unit.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
