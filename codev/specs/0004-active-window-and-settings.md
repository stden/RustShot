# Inception: Active Window Capture and Settings Readability

<!--
AIDLC Inception artifact (Mob Elaboration).

This artifact was requested after implementation had already begun. RustShot's
local AIDLC definition has no runnable driver, so no claim is made that the
intent-approval gate ran before code was written.
-->

## Metadata
- **ID**: inception-2026-09-03-active-window-and-settings
- **Status**: draft (retrospective; intent approval pending)
- **Created**: 2026-09-03
- **Protocol**: aidlc

## Business Intent

Make RustShot's interactive current-window capture behave reliably and visibly: it must capture the window the user meant at the moment they invoked the action, make that same screenshot immediately pasteable while retaining the saved file, and explain Windows' temporary yellow capture outline. At the same time, make the native Settings window easier to read by widening its fields and increasing its font size without changing what the settings mean.

## Clarifications

| Question | Answer | Asked of |
|---|---|---|
| Which settings surface should receive wider fields and larger text? | The native window titled **RustShot Settings** (the user clarified: "В окне Settings"). | User |

### Assumptions

- The user did not specify exact dimensions or a point size. Proceed with a modest, visibly larger Windows-consistent font and enough extra field width to show materially more of folder paths. If visual review finds clipping or excessive size, only presentation values should need adjustment.
- The question about the yellow outline asks for an explanation, not suppression. If border removal is desired, it requires a separately approved scope covering Windows consent and package capability.
- The tray command **Capture Active Window** is an equivalent interactive route and should receive the same target-selection and clipboard-plus-file behavior as `Alt+PrintScreen`. The CLI `--window` route remains file-only.

> **Process note:** The user requested AIDLC documentation during implementation. The repository explicitly states that no AIDLC driver exists, so formal `CLARIFICATION_NEEDED` pauses, consultation, and intent/design gates were not run. This document records the available intent honestly and must not be treated as retroactive gate approval.

## Requirements

- **R1**: Pressing `Alt+PrintScreen` captures the window that was foreground when the key press was delivered, even if focus changes before asynchronous image work starts.
- **R2**: Choosing **Capture Active Window** from the tray captures the window that was foreground immediately before the tray popup took focus, not the tray helper window.
- **R3**: If the selected window closes or Windows recycles its native window identity before capture completes, RustShot reports a readable error instead of capturing an unrelated window.
- **R4**: Each successful interactive current-window action captures image pixels exactly once.
- **R5**: The same captured pixels are copied to the clipboard and saved to the configured screenshots folder in the configured file format.
- **R6**: Clipboard and file delivery are both attempted after capture; a failure in either destination identifies what succeeded and what failed.
- **R7**: User documentation explains that the temporary yellow outline during `PrintScreen` region selection is Windows' system capture indicator, is absent from the resulting image, and is not currently suppressed by RustShot.
- **R8**: The native Settings window and its folder, format, and FPS fields are wider than the previous compact layout.
- **R9**: The Settings window uses a visibly larger font, and labels, fields, checkboxes, and buttons have enough height and spacing to avoid clipping.
- **R10**: Existing setting values, validation rules, persistence, and CLI current-window output behavior do not change as a side effect of the presentation work.

### Out of scope

- Removing or hiding Windows' yellow capture border, packaging RustShot solely to obtain a borderless-capture capability, or adding a new consent flow.
- Changing the region-selection capture backend.
- Copying CLI `--window` captures to the clipboard.
- Adding settings, changing defaults or validation, or redesigning the Settings information architecture.
- Adding current-window capture to platforms where RustShot does not already support it.

## Units of Work

### U1: Stable interactive window identity
- **Delivers**: Hotkey and tray actions retain the intended foreground window across focus changes and reject a closed or recycled target.
- **Requirements covered**: R1, R2, R3
- **Verified by**: Deterministic routing and identity tests plus Windows manual capture from both invocation paths.
- **Depends on**: none

### U2: One capture, two outputs
- **Delivers**: Interactive current-window capture copies and saves the exact same image, while reporting partial output failures clearly.
- **Requirements covered**: R4, R5, R6, R10
- **Verified by**: Pure-logic unit tests and a Gherkin scenario that compare exact pixel bytes and invocation counts.
- **Depends on**: U1

### U3: Windows capture-indicator explanation
- **Delivers**: Users can distinguish the Windows yellow capture indicator from content embedded in their screenshot and understand why RustShot leaves it enabled.
- **Requirements covered**: R7
- **Verified by**: Documentation review against the actual `PrintScreen` workflow and stated Windows capability constraint.
- **Depends on**: none

### U4: Readable Settings layout
- **Delivers**: The native Settings window has wider fields and larger, unclipped text while preserving all existing setting behavior.
- **Requirements covered**: R8, R9, R10
- **Verified by**: Layout-constant unit tests, the settings test suite, and Windows visual inspection at representative display scaling.
- **Depends on**: none

## Acceptance Criteria

- [ ] **AC1**: Changing focus immediately after `Alt+PrintScreen` does not change which window RustShot attempts to capture. (R1)
- [ ] **AC2**: The tray command captures the application that was foreground before the tray menu appeared. (R2)
- [ ] **AC3**: A closed or recycled native window identity produces a human-readable failure and never captures the replacement window. (R3)
- [ ] **AC4**: A successful interactive current-window action performs one capture, one clipboard write, and one file save using byte-identical source pixels. (R4, R5)
- [ ] **AC5**: Clipboard and file delivery are both attempted, and partial failures name the successful and failed destinations. (R6)
- [ ] **AC6**: README and tutorial troubleshooting text explain the yellow outline, state that it is not in the image, and do not promise border suppression. (R7)
- [ ] **AC7**: Folder-path fields display materially more path text, and format/FPS controls are wider without overlap. (R8)
- [ ] **AC8**: Settings text is visibly larger than before and no label, field, checkbox, or button text is clipped in the verified Windows configurations. (R9)
- [ ] **AC9**: Saving settings still applies the same values and validation, and CLI `--window` remains file-only. (R10)

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Foreground focus changes between UI callback and worker execution | A valid screenshot contains the wrong window and appears as if the shortcut failed | Snapshot the target at the invocation boundary and carry that result into asynchronous work |
| Windows recycles a native window handle after the original window closes | RustShot could capture an unrelated window | Bind the handle to its process/thread identity and validate around both capture backends |
| The tray library changes popup event ordering | The recorded target could be the tray helper rather than the user's app | Isolate one-shot tray-target pairing, test its state transitions, and manually verify on Windows |
| One output destination fails | The other successful result could be discarded or reported misleadingly | Attempt clipboard and file independently after the single capture and combine their results |
| Fixed geometry or font sizing behaves poorly under display scaling | Settings text may clip despite passing constant tests | Use Windows-native scaling behavior and perform visual checks at representative DPI values |
| Retrospective artifacts imply gates that never happened | Reviewers could overestimate process evidence | Keep all artifacts in draft, record the missing driver, and leave approvals/verification pending |
