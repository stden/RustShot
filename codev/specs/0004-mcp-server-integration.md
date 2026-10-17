# Specification: RustShot MCP Server Integration

## Metadata
- **ID**: spec-2026-03-13-mcp-server-integration
- **Status**: draft
- **Created**: 2026-03-13

## Clarifying Questions Asked
1. **Q**: Should MCP support be a separate binary or part of `rustshot`?
   **A**: It should be available through the existing binary using `rustshot --mcp`.
2. **Q**: Which operations should be exposed first?
   **A**: Screen listing, screenshot capture, interactive region capture, recording control, and opening the screenshots folder.
3. **Q**: Should MCP mode initialize the tray and hotkeys?
   **A**: No. Server mode must be isolated from tray mode.
4. **Q**: Can interactive tools remain interactive?
   **A**: Yes, if the behavior is clearly documented and the protocol response is explicit.

## Problem Statement
RustShot already contains the beginnings of MCP-facing documentation:
`.mcp.json` exists, `mcp-server/README.md` exists, and the intended
`rustshot --mcp` entry point is described. However, the implementation is still
missing. This creates a mismatch between repository shape and actual product
capability.

Without a real MCP server:
- LLM clients cannot control RustShot programmatically.
- Existing docs and sample config overstate what the binary can do.
- Future automation flows must keep shelling out through ad hoc CLI paths.
- Recording and interactive capture cannot participate in a structured tool protocol.

## Current State
Today RustShot has:
- Tray mode for desktop usage.
- CLI mode for single-capture flows.
- Internal child-process modes for selection and recording setup.
- Preliminary MCP documentation that describes a planned server.

Missing pieces:
- No `--mcp` flag in the active binary contract.
- No JSON-RPC or MCP request loop.
- No tool registry or structured tool error handling.
- No in-process recording session lifecycle for server requests.

## Desired State
RustShot should expose a real MCP server over stdin/stdout through
`rustshot --mcp`. That server should:
- Initialize cleanly without tray startup or global hotkey registration.
- List and execute a small, well-defined set of tools.
- Reuse existing capture, selection, recording, and open-folder behavior.
- Return structured success and error responses.
- Clearly distinguish between non-interactive and interactive tools.

The first MCP release does not need streaming progress, subscriptions, or
advanced session multiplexing. It does need predictable behavior and explicit
failure modes.

## Stakeholders
- **Primary Users**: Developers and AI-assisted tools that want to trigger captures or recordings programmatically.
- **Secondary Users**: Project maintainers who want one supported integration surface instead of custom shell wrappers.
- **Technical Team**: Rust maintainers working in `main`, `capture`, `selection`, `record`, and a new `mcp` module.
- **Business Owners**: Project maintainers defining RustShot as both a desktop app and an automation-friendly tool.

## Success Criteria
- [ ] `rustshot --mcp` starts a server mode without tray or hotkey initialization.
- [ ] MCP clients can initialize, list tools, and execute supported tools successfully.
- [ ] Structured errors are returned for invalid parameters, unsupported platforms, and duplicate recording state changes.
- [ ] Recording start/stop works within a single MCP process lifecycle.
- [ ] Documentation and `.mcp.json` examples match the implemented interface.
- [ ] Tests cover request parsing, dispatch, and error mapping.

## Constraints
### Technical Constraints
- RustShot remains a single binary.
- Existing capture and recording logic should be reused rather than duplicated.
- Interactive region selection still depends on the existing desktop overlay flow.
- Server mode must coexist with current CLI, tray, and internal child-process modes.

### Business Constraints
- The initial MCP surface should stay small and practical.
- The project should not introduce a large protocol framework if a focused implementation is sufficient.
- Server mode should remain easy to document and debug.

## Assumptions
- MCP clients will invoke the server as a subprocess using stdin/stdout.
- A minimal JSON-RPC implementation is acceptable for the first delivery.
- Interactive tools are acceptable if clearly labeled and documented.
- One active recording session per MCP process is sufficient for v1.

## Solution Approaches

### Approach 1: Embed MCP Mode in the Existing Binary
**Description**: Add `--mcp` to `rustshot`, create a focused `src/mcp.rs`
module, and route tool calls into existing RustShot functions.

**Pros**:
- Single deployment artifact.
- Shared logic and fewer integration seams.
- Matches the existing `.mcp.json` and README direction.

**Cons**:
- `main.rs` gains another top-level mode.
- Care is required to keep tray initialization out of server mode.

**Estimated Complexity**: Medium
**Risk Level**: Low

### Approach 2: Build a Separate Sidecar MCP Binary
**Description**: Create a second executable dedicated to MCP and let it call
the main binary or library functions.

**Pros**:
- Cleaner separation of concerns.
- Server mode cannot accidentally start tray mode.

**Cons**:
- More packaging complexity.
- More documentation and release surface area.
- Requires a stronger internal API boundary first.

**Estimated Complexity**: Medium
**Risk Level**: Medium

### Approach 3: Keep Only Documentation and Shell Wrappers
**Description**: Do not build a real server; rely on external wrappers around
CLI and tray behavior.

**Pros**:
- Lowest short-term effort.

**Cons**:
- Does not actually satisfy MCP integration.
- Leaves repo docs inaccurate.
- Makes structured tool use and recording lifecycle management awkward.

**Estimated Complexity**: Low
**Risk Level**: High

## Open Questions

### Critical (Blocks Progress)
- [ ] None.

### Important (Affects Design)
- [ ] Should interactive tools return immediately with a pending/result token, or block until the selection is complete?
- [ ] Should `start_recording` accept optional output path and FPS overrides, or rely only on defaults in v1?

### Nice-to-Know (Optimization)
- [ ] Should future MCP versions expose screenshot data directly in responses, or remain file-path based?

## Performance Requirements
- **Server Startup**: MCP mode should initialize quickly and stay below 500ms on a typical development machine.
- **Request Handling**: Non-interactive tool dispatch should add minimal overhead over direct function invocation.
- **Resource Usage**: Idle MCP mode should not start tray UI or background hotkey listeners.
- **Availability**: Invalid requests must fail gracefully without crashing the server.

## Security Considerations
- MCP mode must not execute arbitrary shell commands.
- Tool arguments should be validated before they reach file or recording operations.
- File system operations such as open-folder should be limited to supported RustShot paths.
- Recording and interactive capture should fail clearly when not supported on the current platform.

## Test Scenarios
### Functional Tests
1. Start `rustshot --mcp`, initialize, and list tools.
2. Call `list_screens` and verify monitor information is returned.
3. Call `screenshot` with and without explicit monitor index and verify a file path is returned.
4. Call `start_recording`, then `stop_recording`, and verify a saved MP4 path is returned on Windows.
5. Call `open_screenshots_folder` and verify success response shape.

### Non-Functional Tests
1. Send malformed JSON-RPC input and verify a structured error response.
2. Attempt duplicate `start_recording` or `stop_recording` calls and verify deterministic errors.
3. Verify `--mcp` mode does not register tray or hotkeys.

## Dependencies
- **External Services**: MCP-compatible clients using stdin/stdout subprocess execution.
- **Internal Systems**: `main`, `capture`, `selection`, `record`, open-folder helpers, settings defaults if present.
- **Libraries/Frameworks**: Standard library I/O, optional lightweight JSON handling, existing RustShot modules.

## References
- [001-feature-roadmap umbrella plan](../plans/001-feature-roadmap.md)
- `mcp-server/README.md`
- `.mcp.json`
- `src/main.rs`

## Risks and Mitigation
| Risk | Probability | Impact | Mitigation Strategy |
|------|-------------|--------|---------------------|
| Server mode accidentally boots tray or hotkeys | Medium | High | Split top-level mode routing early and test it directly |
| Interactive tools do not fit neatly into request/response flow | Medium | Medium | Document blocking semantics explicitly and keep response shapes simple |
| Recording session state becomes inconsistent | Medium | High | Limit v1 to one active session per process and return deterministic state errors |

## Expert Consultation
**Date**: 2026-03-13
**Models Consulted**: GPT-5 Codex
**Sections Updated**:
- **Desired State**: Clarified that v1 MCP is request/response only.
- **Solution Approaches**: Chose embedded binary mode as the preferred design.
- **Security Considerations**: Added validation and shell-execution constraints.

Note: All consultation feedback has been incorporated directly into the relevant sections above.

## Approval
- [ ] Technical Lead Review
- [ ] Product Owner Review
- [ ] Stakeholder Sign-off
- [ ] Expert AI Consultation Complete

## Notes
- This spec is intentionally file-path oriented for outputs; returning raw image bytes is out of scope for v1.

---

## Amendments

This section tracks all TICK amendments to this specification. TICKs are
lightweight changes that refine an existing spec rather than creating a new one.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
