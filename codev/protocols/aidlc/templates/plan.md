# Construction Design: [Title]

<!--
AIDLC Construction artifact, front half: the logical design the team validates
BEFORE any code is written.

`## Domain Model` and the `"phases":` JSON block are enforced by protocol
checks. One plan phase per unit of work from Inception.
-->

## Metadata
- **ID**: design-[YYYY-MM-DD]-[short-name]
- **Status**: draft
- **Inception**: [Link to codev/specs/[title].md]
- **Created**: [YYYY-MM-DD]
- **Protocol**: aidlc

## Logical Architecture

[How the pieces fit. Which existing modules change, which are added. Name the seam each new piece sits behind. Reference src/lib.rs — this project keeps design rationale in rustdoc, so say which rustdoc will carry this explanation.]

## Domain Model

<!--
The types and the invariants they make unrepresentable. Concrete Rust: which
newtypes, which enums replace stringly-typed state, which construction paths
are made impossible.

"A struct holding the config" is not a domain model. "SelectionRect, which
cannot be constructed with zero area, so the overlay guard becomes
unreachable" is.
-->

| Type | Kind | Invariant it enforces |
|---|---|---|
| `[Name]` | newtype / enum / struct | [What becomes unrepresentable] |

### Error handling

[This project uses `Result<_, String>` throughout, with human-readable messages. State the messages this design introduces, and which are user-facing.]

## Phases (Machine Readable)

<!-- REQUIRED: porch uses this JSON to track bolt progress. One phase per unit of work. -->

```json
{
  "phases": [
    {"id": "u1", "title": "U1: [Unit name from Inception]"},
    {"id": "u2", "title": "U2: [Unit name from Inception]"}
  ]
}
```

## Bolts

### Bolt u1 — [Unit name]

- **Unit of work**: U1
- **Changes**: [Files touched, with why each]
- **Test first**: [The test to write and watch fail, before the fix]
- **Done when**: [Observable condition, plus the four checks passing]

### Bolt u2 — [Unit name]

- **Unit of work**: U2
- **Changes**: [...]
- **Test first**: [...]
- **Done when**: [...]

## Platform Considerations

<!-- This project is platform-sensitive. Address each row or state "unaffected". -->

| Platform | Consideration |
|---|---|
| Windows | Tray and hotkeys are easy to break accidentally; recording is Windows-only |
| Linux | Requires X11 libraries; tray behaviour differs |
| macOS | [Applicability] |

## Rejected Alternatives

| Alternative | Why rejected |
|---|---|
| [Approach] | [Reason — this is what stops the question being reopened next bolt] |

## Verification Strategy

[Which units need pure-logic unit tests, which need Gherkin features in tests/features/ run by tests/bdd.rs, which need benchmarks. Avoid heavy GUI tests unless unavoidable.]
