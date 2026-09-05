# Operations: Active Window Capture and Settings Readability

<!--
AIDLC Operations artifact.

This is intentionally incomplete. It was requested while implementation and
verification were still in progress, and RustShot has no runnable AIDLC driver.
Pending results and commits must be replaced with observed evidence only.
-->

## Metadata

- **ID**: operations-2026-09-03-active-window-and-settings
- **Status**: draft (implementation/verification in progress)
- **Inception**: [Active Window Capture and Settings Readability](../specs/0004-active-window-and-settings.md)
- **Design**: [Active Window Capture and Settings Readability](../plans/0004-active-window-and-settings.md)
- **Created**: 2026-09-03
- **Protocol**: aidlc

## Summary

Four linked bolts document reliable interactive current-window targeting, one-image clipboard-plus-file delivery, an
explanation of Windows' yellow capture indicator, and a wider/larger-font Settings window. This Operations record is a
handoff draft, not a shipping assertion: final checks, Windows visual/runtime verification, commit IDs, push state, and
human approval remain pending.

## Bolts Completed

| Bolt | Unit                                      | Outcome                                                           | Commits |
|------|-------------------------------------------|-------------------------------------------------------------------|---------|
| u1   | U1: Stable interactive window identity    | Implementation present; final verification pending                | pending |
| u2   | U2: One capture, two outputs              | Implementation present; final verification pending                | pending |
| u3   | U3: Windows capture-indicator explanation | Documentation present; final review pending                       | pending |
| u4   | U4: Readable Settings layout              | Implementation present; automated and visual verification pending | pending |

## Acceptance Criteria

- [ ] **AC1**: Press-time `Alt+PrintScreen` target survives later focus changes. (bolt u1; pending final evidence)
- [ ] **AC2**: Tray capture uses the pre-popup foreground target. (bolt u1; pending final evidence)
- [ ] **AC3**: Closed/recycled native identities are rejected without capturing a replacement window. (bolt u1; pending
  final evidence)
- [ ] **AC4**: One capture supplies byte-identical pixels to one clipboard write and one file save. (bolt u2; pending
  final evidence)
- [ ] **AC5**: Both destinations are attempted and partial failures describe both outcomes. (bolt u2; pending final
  evidence)
- [ ] **AC6**: README/tutorial accurately explain the yellow Windows indicator without claiming removal. (bolt u3;
  pending final review)
- [ ] **AC7**: Settings fields are wider without overlap. (bolt u4; pending Windows visual verification)
- [ ] **AC8**: Settings text is larger and unclipped. (bolt u4; pending Windows visual verification)
- [ ] **AC9**: Setting semantics and file-only CLI `--window` behavior remain unchanged. (bolts u2/u4; pending
  regression verification)

## Release Readiness

| Check          | Command                      | Result                                              |
|----------------|------------------------------|-----------------------------------------------------|
| Compile        | `cargo check --all-targets`  | pending                                             |
| Tests          | `cargo test`                 | pending; do not copy partial counts here            |
| Format         | `cargo fmt --all --check`    | pending                                             |
| Lint           | `cargo lint`                 | pending                                             |
| Release binary | `cargo build --release`      | pending                                             |
| Benchmarks     | not required for this change | no budget changed; performance evidence not claimed |
| Commit         | `git commit`                 | pending                                             |
| Push           | `git push`                   | pending                                             |

### Platform verification

| Platform | Verified how                                                                                                                                   | Result                      |
|----------|------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------|
| Windows  | Pending manual hotkey/tray capture, clipboard/file comparison, yellow-border observation, and Settings inspection at available display scaling | pending                     |
| Linux    | Shared compile/test coverage only; Windows features are not runtime-applicable                                                                 | pending compile/test result |
| macOS    | Shared compile/test coverage only; Windows features are not runtime-applicable                                                                 | pending compile/test result |

## Context Updates

- **Rustdoc**: Internal capture rustdoc now explains the PID/TID-bound `WindowTarget`, validation around asynchronous
  WGC/GDI work, same-image output delivery, and custom Settings-font ownership. `src/lib.rs` and public architecture
  remain unchanged. Confirm this description against the final diff.
- **Gherkin features**: `tests/features/active_window.feature` adds an active-window output-delivery scenario that
  requires one capture and byte-identical clipboard/file input. Final scenario count is pending the full test run.
- **Settings / CLI surface**: No new persisted setting or flag. Interactive `Alt+PrintScreen` and tray active-window
  capture copy and save; CLI `--window` remains file-only. Settings presentation uses wider controls and a larger
  default font while retaining existing validation and defaults.
- **Benchmark budgets**: None — no measured budget or performance-gate threshold changed.

## Bolt Retrospective

| What AI proposed                                                                  | What the team corrected                                                                                                                    | Root cause                                                                                                                                                        | What to do differently                                                                      |
|-----------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| Treat the `Alt+PrintScreen` report primarily as a hotkey/capture-backend failure  | Runtime evidence showed the hotkey fired and a file was captured; the missing user-visible clipboard result was the important contract gap | The prior documentation described file-only behavior, but the user's phrase “doesn't work” was not decomposed into target, clipboard, file, and feedback outcomes | During Inception, enumerate every externally visible output before diagnosing the backend   |
| Snapshot only the hotkey's raw foreground handle and resolve tray capture later   | Technical review extended invocation-time identity to the tray's pre-popup boundary and required one-shot pairing                          | The first design analyzed the hotkey callback but not the tray library's focus-transfer sequence                                                                  | Draw the event timeline for every entry point before choosing the snapshot boundary         |
| Carry a raw `HWND` into delayed work                                              | Technical review bound the handle to PID/TID and required revalidation around WGC and GDI                                                  | Native-handle reuse was omitted from the initial lifetime model                                                                                                   | For asynchronous native handles, document closure and reuse semantics during logical design |
| Interpret “make the fields wider” and “use a larger font” without a named surface | The user clarified that both changes belong in the Settings window                                                                         | The original wording could have referred to another form or overlay                                                                                               | Ask which window/control group is in scope before selecting geometry                        |

### Clarifications that paid off

The user's explicit “В окне Settings” clarification prevented layout changes from leaking into the capture overlay or
other UI. No formal `CLARIFICATION_NEEDED` signal was emitted because the AIDLC artifacts and driver were not active
during implementation; this record must not imply otherwise.

### Clarifications that were missed

- What exact field widths and font size does the user prefer, and at which Windows display-scaling values should
  acceptance be judged?
- Should “current window” parity apply to both tray and hotkey only, or should CLI `--window` also copy to the
  clipboard?
- Was the yellow-outline question purely explanatory, or should a separately packaged borderless-capture mode be
  considered later?
- Should partial clipboard/file failures remain log-only or gain a visible notification?

## Deviations from Design

- **All bolts**: The user requested these AIDLC artifacts after implementation was underway. Because the repository has
  no AIDLC driver, Mob Elaboration, model consultation, intent approval, and design approval did not precede code. The
  artifacts are retrospective drafts and cannot satisfy those historical gates retroactively.
- **Bolt u1**: The early implementation carried only a raw hotkey-time `HWND` and deferred tray lookup. Review
  strengthened it to a PID/TID-bound target and a pre-popup tray snapshot to close focus and handle-reuse races.
- **Bolt u4**: Exact geometry and font size were not user-specified. The design chose a 590×320 baseline with 11-point
  Segoe UI and wider controls; final Windows visual acceptance remains required.

## Follow-up Work

| Item                                     | Why deferred                                                                                      | Where tracked                                                            |
|------------------------------------------|---------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------|
| Borderless Windows capture mode          | Not requested; requires an explicit consent, packaging, and capability decision                   | A future Inception, only if the user requests removal                    |
| Further DPI-adaptive Settings layout     | Fixed baseline must first be visually evaluated; no problem should be invented before observation | This review's pending Windows verification, then a future bolt if needed |
| User-visible partial-output notification | Current scope preserves the existing logging surface; product behavior needs an explicit decision | A future Inception if log-only feedback proves insufficient              |
