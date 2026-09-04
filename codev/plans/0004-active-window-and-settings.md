# Construction Design: Active Window Capture and Settings Readability

<!--
AIDLC Construction logical-design artifact.

This is a retrospective draft because the user requested AIDLC documentation
after implementation started and the repository has no runnable AIDLC driver.
It describes the design to validate; it does not assert prior design approval.
-->

## Metadata
- **ID**: design-2026-09-03-active-window-and-settings
- **Status**: draft (retrospective; design approval pending)
- **Inception**: [Active Window Capture and Settings Readability](../specs/0004-active-window-and-settings.md)
- **Created**: 2026-09-03
- **Protocol**: aidlc

## Logical Architecture

The active-window path is split at three boundaries. `src/app/tray_app.rs` observes user intent at the hotkey or tray callback, attaches the foreground-target result to the event, and keeps expensive capture work off the event loop. `src/capture.rs` exposes the target snapshot/capture and already-captured-image save seams. `src/capture/platform.rs` owns Win32 identity validation and both Windows Graphics Capture and GDI fallback, so raw native identity never escapes into an unvalidated backend call.

Hotkey events snapshot the foreground target when the pressed event is delivered. Tray invocation needs a different boundary: `TrayIconEvent` records the target on mouse-up, before the tray library foregrounds its hidden popup owner; `MenuEvent` consumes that recorded result exactly once. Mouse-down clears any stale result. Both paths carry failure as data rather than looking up whichever window happens to be foreground later.

After one image is captured, `src/app/runtime.rs` copies a borrowed RGBA image to the clipboard and `src/capture.rs` saves that same image using the existing naming and format policy. `src/app/tray_app.rs` coordinates both destinations and combines their outcomes. The CLI continues through its file-only facade.

The native Settings presentation remains within `src/settings_dialog.rs` and `src/settings_dialog/controls.rs`. Central layout constants widen the top-level window and all relevant fields as one layout. A custom Segoe UI font is applied to every child control; its handle is stored on the dialog and deleted during final window destruction, with the stock GUI font as a non-owned fallback. `src/settings_dialog/tests.rs` locks the chosen baseline geometry and type size without changing the persisted `Settings` model.

`README.md` and `docs/TUTORIAL.md` describe clipboard-plus-file interactive window capture, the unchanged file-only CLI route, and the Windows yellow system indicator. `src/lib.rs` remains the architecture entry point; no public module boundary changes. The durable rationale belongs in rustdoc on `WindowTarget`, its validation/capture functions, the active-window output seam, and the Settings font lifecycle.

Because code existed before this document, each bolt below describes the intended design and required evidence. Red-first evidence may be claimed only where recorded output exists; it must not be reconstructed after the fact.

## Domain Model

| Type | Kind | Invariant it enforces |
|---|---|---|
| `WindowTarget` | struct | A raw `HWND` is inseparable from the owning PID and TID observed at selection time; a backend may capture only while all three still match |
| `RustShotEvent` | enum | A pressed active-window action carries the target snapshot or the snapshot error through the event loop, preventing a later foreground lookup from silently changing intent |
| `PendingTrayWindowTarget` | one-shot synchronized slot | At most one pre-popup target result is paired with one menu selection; a new click clears stale state and consumption removes it |
| `RgbaImage` | existing owned image buffer | One captured RGBA buffer is the source for both clipboard and file destinations, so the outputs cannot come from different foreground windows |

The documentation unit and Settings presentation unit deliberately add no persisted domain type. The yellow border is an operating-system fact, while dialog dimensions and font resource ownership are view policy; representing either as user configuration would create unsupported state.

### Error handling

All new fallible seams use `Result<_, String>` with contextual, human-readable messages. Foreground snapshot failures distinguish no foreground window from a window that closed before its identity could be recorded. Capture rejects a target whose PID/TID no longer match its `HWND`, explicitly saying that the target closed or the handle was reused. Missing hotkey or pre-popup tray snapshots remain errors and never trigger a fallback lookup of the current foreground window.

Clipboard and file delivery are both attempted after capture. A clipboard-only failure reports that the file was saved and includes its path; a file-only failure reports that clipboard delivery succeeded; two failures report both causes. These messages currently go through tray-mode logging rather than adding a new notification surface. Settings resource-allocation failure falls back to the stock system font; existing user-facing settings validation messages are unchanged.

## Phases (Machine Readable)

```json
{
  "phases": [
    {"id": "u1", "title": "U1: Stable interactive window identity"},
    {"id": "u2", "title": "U2: One capture, two outputs"},
    {"id": "u3", "title": "U3: Windows capture-indicator explanation"},
    {"id": "u4", "title": "U4: Readable Settings layout"}
  ]
}
```

## Bolts

### Bolt u1 — Stable interactive window identity

- **Unit of work**: U1
- **Changes**: `src/capture/platform.rs` introduces the PID/TID-bound window target and validates it before and after WGC/GDI work; `src/capture.rs` exposes the internal target seams; `src/app/tray_app.rs` enriches hotkey/menu events and implements one-shot pre-popup tray pairing; `src/main_tests/infra.rs` and `src/capture/tests/platform_helpers.rs` exercise timing, missing-target, one-shot, and handle-reuse rules.
- **Test first**: Add failing tests proving a pressed hotkey keeps the original target after simulated focus change, a missing press-time target never captures a later window, one tray snapshot is consumed once, and PID/TID mismatch rejects a recycled handle.
- **Done when**: Both invocation paths preserve their pre-focus-change target, identity mismatch is terminal for both backends, Windows manual checks capture the intended app, and the repository compile/test/format/lint checks pass.

### Bolt u2 — One capture, two outputs

- **Unit of work**: U2
- **Changes**: `src/app/runtime.rs` provides a reusable borrowed-image clipboard writer; `src/capture.rs` separates capture from persistence; `src/app/tray_app.rs` orchestrates capture/clipboard/save and aggregates errors; `src/main_tests/infra.rs`, `tests/features/active_window.feature`, `tests/bdd.rs`, and `tests/bdd/requirements/active_window.rs` verify exact image reuse and output attempts; `README.md` and `docs/TUTORIAL.md` state interactive versus CLI behavior.
- **Test first**: Add a failing pure-logic test and Gherkin scenario whose simulated handlers count one capture, one clipboard write, and one save and compare every RGBA byte; add failure-path tests proving a clipboard error does not skip the file attempt and a save error does not erase clipboard success.
- **Done when**: Hotkey and tray paths use one buffer for both destinations, CLI remains file-only, partial failures are contextual, and the repository compile/test/format/lint checks pass.

### Bolt u3 — Windows capture-indicator explanation

- **Unit of work**: U3
- **Changes**: `README.md` explains the yellow system outline at first-use context; `docs/TUTORIAL.md` repeats it in troubleshooting and avoids promising suppression.
- **Test first**: Establish a documentation checklist requiring the actor (Windows Graphics Capture), timing (region-selection preparation), result (not embedded in the image), and constraint (borderless consent/package capability), then inspect both rendered documents against it before marking the unit complete.
- **Done when**: Both documents satisfy the checklist, code behavior is unchanged, links/rendering are sound, and the repository compile/test/format/lint checks pass.

### Bolt u4 — Readable Settings layout

- **Unit of work**: U4
- **Changes**: `src/settings_dialog.rs` centralizes a 590×320 dialog, wider field geometry, 11-point font baseline, and final font cleanup; `src/settings_dialog/controls.rs` applies 340-pixel path fields, a 150-pixel format field, a 100-pixel FPS field, taller fields/buttons, and the font to every control; `src/settings_dialog/tests.rs` fixes the intended minimums and conversion baseline.
- **Test first**: Add failing tests that require the dialog/fields to exceed the prior compact layout, use an 11-point (-15 pixel at 96 DPI) font baseline, and reserve at least 27 pixels for fields and 30 for buttons.
- **Done when**: Unit tests pass, existing settings validation/persistence tests remain green, manual Windows inspection finds no overlap or clipping at representative display scaling, and the repository compile/test/format/lint checks pass.

## Platform Considerations

| Platform | Consideration |
|---|---|
| Windows | All user-visible changes apply here. Hotkey and tray callbacks must snapshot before focus transfer; native handle identity must be revalidated around WGC/GDI; font ownership must match Win32 destruction order; Settings needs manual DPI inspection. |
| Linux | Active-window capture and the native Settings dialog remain unsupported/unaffected at runtime. Non-Windows stubs and shared tests must still compile, and existing X11/tray dependencies must not regress. |
| macOS | Active-window capture and the Win32 Settings dialog remain unsupported/unaffected; shared event/output logic must remain cfg-correct. |

## Rejected Alternatives

| Alternative | Why rejected |
|---|---|
| Resolve the foreground window inside the capture worker | Focus may already have moved, so asynchronous execution can capture the wrong app |
| Carry only a raw `HWND` | Windows can recycle a closed window's handle, allowing an unrelated replacement window to be captured |
| Resolve tray foreground after the popup closes | The tray helper owns focus while the popup is open, and focus after close is not a reliable record of pre-popup intent |
| Capture once for clipboard and again for file | The two lookups can select different windows and waste capture work; byte identity is no longer guaranteed |
| Abort the second output when the first fails | A recoverable file or clipboard result would be lost and users would receive less useful diagnostics |
| Suppress the yellow border in this change | It was not requested and requires a Windows permission/package-capability decision beyond a documentation fix |
| Add font size and layout as persisted user settings | The request is for a clearer default Settings window, not a new configuration surface or migration |
| Change CLI `--window` to write the clipboard | It expands a file-oriented CLI contract that the user did not ask to change |

## Verification Strategy

U1 uses pure-logic tests for event timing, one-shot tray state, and native identity comparison, with a Windows manual check for real callback ordering and focus changes. U2 uses injected capture/clipboard/save handlers for deterministic unit failure paths and a Gherkin scenario that compares exact pixel bytes without touching the desktop or clipboard. U3 uses a four-point documentation review because it introduces no executable behavior. U4 uses constant/resource-lifecycle unit tests plus visual Windows inspection at 100%, 150%, and 200% scaling where available.

Run the full RustShot suite after the units: `cargo check --all-targets`, `cargo test`, `cargo fmt --all --check`, and the repository-authoritative `cargo lint` command described in `CLAUDE.md`. Run `cargo build --release` for Operations. No benchmark budget changes are expected because the work does not alter a measured hot path; do not invent benchmark evidence.
