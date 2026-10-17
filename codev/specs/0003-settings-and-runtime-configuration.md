# Specification: RustShot Settings and Runtime Configuration

## Metadata
- **ID**: spec-2026-03-13-settings-and-runtime-configuration
- **Status**: draft
- **Created**: 2026-03-13

## Clarifying Questions Asked
1. **Q**: Does RustShot need a full native settings window right now?
   **A**: No. A lightweight settings flow is acceptable if it is practical and stable.
2. **Q**: Which settings matter first?
   **A**: Screenshot directory, recording directory, default image format, recording FPS, and auto-start on Windows.
3. **Q**: Should CLI flags still win over persisted defaults?
   **A**: Yes. CLI input is the explicit user command and must override saved settings.
4. **Q**: Should settings apply cross-platform?
   **A**: Persistence should be cross-platform where possible, but Windows-only behavior such as auto-start remains platform-specific.

## Problem Statement
RustShot has started to grow beyond hardcoded defaults, but there is no complete
runtime configuration contract yet. The repository already contains
`src/settings.rs`, yet the application still behaves primarily as if defaults
are compiled in. This creates several issues:

- Screenshot and recording defaults are not consistently sourced from one place.
- The tray "Settings" action is not yet the gateway to a complete, intentional
  configuration experience.
- Windows auto-start behavior risks diverging from what the user has actually
  configured.
- Future integrations such as MCP will have no stable source of default values.

Without a clear settings specification, feature work will keep hardcoding
behavior in multiple modules and make later cleanup harder.

## Current State
Today the repository has an initial `settings.rs` module that:
- Defines a `Settings` struct with key user preferences.
- Loads and saves a JSON file in the app-config directory.
- Opens the settings file in a default editor instead of rendering a native UI.
- Uses a manual JSON parser rather than `serde`.

Current limitations:
- Settings are not yet the canonical runtime defaults across the application.
- Auto-start behavior is not fully specified as an enable/disable contract.
- There is no documented precedence model for CLI flags versus persisted config.
- Validation boundaries for invalid or partial settings files are not explicit.

## Desired State
RustShot should have a small but complete settings model that:
- Loads automatically at startup.
- Supplies default screenshot and recording behavior when the user does not
  provide explicit CLI input.
- Persists reliably across restarts.
- Exposes a tray-accessible edit flow that is stable even before a native UI
  exists.
- Treats Windows auto-start as an actual user preference, not a one-way side
  effect.

The v1 settings experience does not need a custom window. It does need a clean
contract that later UI work can sit on top of without changing semantics.

## Stakeholders
- **Primary Users**: Desktop users who want RustShot to remember output locations and capture defaults.
- **Secondary Users**: Developers invoking CLI mode or future MCP mode who rely on predictable default behavior.
- **Technical Team**: Rust maintainers responsible for `main`, `capture`, `record`, and Windows startup behavior.
- **Business Owners**: Project maintainers deciding what RustShot should remember between launches.

## Success Criteria
- [ ] RustShot loads settings on startup and falls back to safe defaults when the file is missing or malformed.
- [ ] Persisted settings apply to screenshot and recording defaults when explicit CLI options are absent.
- [ ] CLI arguments override persisted settings deterministically.
- [ ] Windows auto-start can be both enabled and disabled through the persisted settings model.
- [ ] Settings round-trip tests cover complete, partial, and malformed JSON input.
- [ ] Documentation explains where settings live and which values are platform-specific.

## Constraints
### Technical Constraints
- RustShot remains a single-binary application with no background service or database.
- The current codebase uses `Result<_, String>` and favors lightweight helper logic.
- The existing settings module already uses manual JSON parsing; replacing it is optional, not mandatory.
- Windows startup registration depends on the user-level Run registry key.

### Business Constraints
- The project should remain lightweight and easy to build.
- Introducing a heavy settings framework is out of scope for this iteration.
- Any UI expansion should not block shipping a stable settings contract first.

## Assumptions
- Users are comfortable editing a settings file if the path is easy to open.
- Cross-platform config directories are available via `dirs`.
- The first release of settings can remain text-file based without harming usability.
- Future GUI work should reuse the same underlying settings semantics.

## Solution Approaches

### Approach 1: Lightweight File-Based Settings Contract
**Description**: Keep the JSON file, keep the tray action, and make the settings
module the canonical source of runtime defaults across the app.

**Pros**:
- Minimal architectural risk.
- Easy to test with pure helper functions.
- Compatible with current repository direction.

**Cons**:
- Manual JSON parsing is less robust than typed serialization.
- The user experience is less polished than a native dialog.

**Estimated Complexity**: Low
**Risk Level**: Low

### Approach 2: Move Immediately to `serde` and a Native Settings UI
**Description**: Replace the lightweight editor-based flow with fully typed
serialization and a native window now.

**Pros**:
- Stronger parsing guarantees.
- Better user experience from the start.

**Cons**:
- Larger change set with more moving parts.
- More UI work before runtime semantics are even settled.

**Estimated Complexity**: Medium
**Risk Level**: Medium

### Approach 3: Keep Settings as Documentation Only for Now
**Description**: Do not wire runtime settings yet; continue using hardcoded
defaults and treat `settings.rs` as experimental.

**Pros**:
- No short-term implementation effort.

**Cons**:
- Increases drift across modules.
- Makes future MCP and tray work harder.
- Leaves auto-start behavior underspecified.

**Estimated Complexity**: Low
**Risk Level**: High

## Open Questions

### Critical (Blocks Progress)
- [ ] None.

### Important (Affects Design)
- [ ] Should invalid `default_format` values be normalized to `png` or rejected with an explicit user-facing warning?
- [ ] Should settings edits be applied only on next launch or also reloaded live from the tray action?

### Nice-to-Know (Optimization)
- [ ] Should a future native settings UI remain file-backed or move to a typed settings store?

## Performance Requirements
- **Startup Overhead**: Settings load should add negligible startup cost and stay well below 50ms on typical machines.
- **Persistence Cost**: Saving settings should be a single small file write.
- **Resource Usage**: No persistent background watcher is required in v1.
- **Availability**: If settings loading fails, RustShot must still run with defaults.

## Security Considerations
- Settings contain no secrets and should remain free of credentials or tokens.
- Auto-start writes must remain in `HKCU`, not require elevation, and not touch machine-wide startup locations.
- Malformed settings input must never crash the app.
- Settings paths should be treated as user-controlled input when used to create directories or spawn editor processes.

## Test Scenarios
### Functional Tests
1. Start RustShot with no settings file and verify defaults are used.
2. Save valid settings, restart, and verify screenshot and recording defaults come from the file.
3. Start CLI mode with explicit flags and verify they override saved defaults.
4. Toggle auto-start off and verify the Windows startup entry is removed or not re-created.
5. Open settings from tray and verify the backing file is created if missing.

### Non-Functional Tests
1. Load malformed JSON and verify fallback behavior is safe and fast.
2. Save paths with spaces and verify they survive round-trip serialization.
3. Verify platform-specific settings behavior on Windows and non-Windows systems.

## Dependencies
- **External Services**: None.
- **Internal Systems**: `main`, `capture`, `record`, tray menu handling, Windows autorun helpers.
- **Libraries/Frameworks**: `dirs`, standard library file APIs, optional future `serde` if required.

## References
- [0001-rustshot-screenshot-utility](0001-rustshot-screenshot-utility.md)
- [001-feature-roadmap umbrella plan](../plans/001-feature-roadmap.md)
- `src/settings.rs`
- `src/main.rs`

## Risks and Mitigation
| Risk | Probability | Impact | Mitigation Strategy |
|------|-------------|--------|---------------------|
| Manual parser accepts ambiguous JSON | Medium | Medium | Add narrow tests, normalize invalid fields, and fall back to defaults when unsure |
| Auto-start state becomes inconsistent with settings | Medium | High | Define one helper path for reading and applying auto-start state |
| CLI and settings precedence becomes unclear | Medium | High | Define and test a single effective-settings resolution path |

## Expert Consultation
**Date**: 2026-03-13
**Models Consulted**: GPT-5 Codex
**Sections Updated**:
- **Desired State**: Clarified that v1 does not require a native settings window.
- **Solution Approaches**: Chose a lightweight file-backed contract as the preferred path.
- **Security Considerations**: Added Windows auto-start safety constraints.

Note: All consultation feedback has been incorporated directly into the relevant sections above.

## Approval
- [ ] Technical Lead Review
- [ ] Product Owner Review
- [ ] Stakeholder Sign-off
- [ ] Expert AI Consultation Complete

## Notes
- This spec intentionally separates runtime settings semantics from any future settings UI project.
- The existing `src/settings.rs` file is treated as a starting point, not a completed implementation.

---

## Amendments

This section tracks all TICK amendments to this specification. TICKs are
lightweight changes that refine an existing spec rather than creating a new one.

<!-- When adding a TICK amendment, add a new entry below this line in chronological order -->
