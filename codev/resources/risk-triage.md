# PR Review Risk Triage Guide

Quick reference for the architect's risk-based PR review process. For the full integration review workflow, see `codev/roles/architect.md` Section 4.

## Risk Assessment

When a builder PR arrives, assess risk before choosing review depth.

### Step 1: Check Size and Scope

```bash
gh pr diff --stat <N>
gh pr view <N> --json files | jq '.files[].path'
```

### Step 2: Determine Risk Level

**Precedence: highest factor wins.** If any single factor is high, the overall risk is high.

| Factor | Low | Medium | High |
|--------|-----|--------|------|
| **Lines changed** | < 100 | 100-500 | > 500 |
| **Files touched** | 1-3 | 4-10 | > 10 |
| **Subsystem** | Docs, tests, cosmetic | Features, commands, shared libs | Protocol, state mgmt, security |
| **Cross-cutting** | No shared interfaces | Some shared code | Core interfaces, APIs |

### Step 3: Execute Review

| Risk | Action | Cost |
|------|--------|------|
| **Low** | Read PR, summarize, tell builder to merge | $0 |
| **Medium** | `consult -m claude --type integration pr N` | ~$1-2 |
| **High** | 3-way CMAP (Gemini + Codex + Claude in parallel) | ~$4-5 |

## Subsystem Path Mappings

| Path Pattern | Subsystem | Risk |
|-------------|-----------|------|
| `src/record.rs` | Screen recording (Windows Media Foundation) | High |
| `src/selection.rs` | Region selection overlay (Winit/Softbuffer) | High |
| `src/main.rs` | CLI parsing, Tray loop, Hotkeys | Medium |
| `src/capture.rs` | Screenshot capture and file saving | Medium |
| `src/settings.rs` | Persisted settings and JSON workflow | Medium |
| `src/tray.rs` | Tray icon and menu constants | Low |
| `codev/protocols/` | Protocol definitions | High |
| `codev/specs/`, `codev/plans/`, `codev/reviews/` | Project artifacts | Low |
| `tests/` | Integration and unit tests | Low |
| `*.md` (not in `protocols/`) | Documentation | Low |

## Typical Mappings by Protocol

- **Low**: Most bugfixes, ASPIR features, documentation, UI tweaks, config updates
- **Medium**: SPIR features, new commands, refactors touching 2+ files, new utility modules
- **High**: Changes to recording or selection logic, core state management, new OS integrations

**Note:** Protocol type is a heuristic, not the primary signal. An ASPIR PR that changes core recording logic should still be high-risk. Always check the actual diff.

## Example Workflows

### Low Risk: Bugfix (12 lines, 1 file, tests/)

```bash
$ gh pr diff --stat 120
 tests/bench_functions.rs | 12 ++++++------
 1 file changed, 6 insertions(+), 6 deletions(-)

# Low risk — read and approve
gh pr comment 120 --body "## Architect Review

Low-risk test fix. Corrects assertion in tests.

---
Architect review"

af send 0042 "PR approved, please merge"
```

### Medium Risk: New Feature (180 lines, 2 files, src/)

```bash
$ gh pr diff --stat 121
 src/capture.rs | 120 +++++++++++++
 src/main.rs    |  60 +++++
 2 files changed, 180 insertions(+)

# Medium risk — single-model review
consult -m claude --type integration pr 121
```

### High Risk: Core Change (650 lines, 3 files, src/)

```bash
$ gh pr diff --stat 122
 src/record.rs    | 300 ++++++++++++------
 src/selection.rs | 250 ++++++++++----
 src/main.rs      | 100 +++++---
 3 files changed, 420 insertions(+), 230 deletions(-)

# High risk — full 3-way CMAP
consult -m gemini --type integration pr 122 &
consult -m codex --type integration pr 122 &
consult -m claude --type integration pr 122 &
wait
```
