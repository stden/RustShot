# Verify Phase Prompt (TICK)

You are in the Verify phase of TICK protocol.

## Your Mission

Verify that the amendment implementation is complete and correct. Run tests and build to ensure nothing is broken.

## Input Context

Read these files:
1. `codev/specs/{project-id}-*.md` - Spec with amendment
2. `codev/status/{project-id}-*.md` - Implementation notes

## Workflow

### 1. Run Build

```bash
cargo check
```

If build fails:
- Output: `<signal>VERIFICATION_FAILED</signal>`
- Include error details in output

### 2. Run Tests

```bash
cargo test
```

If tests fail:
- Output: `<signal>VERIFICATION_FAILED</signal>`
- Include which tests failed

### 3. Quick Manual Check

Verify:
- [ ] Amendment matches the request
- [ ] No unintended side effects
- [ ] Code follows project conventions

### 4. Signal Completion

When all checks pass:
1. Update status file with verification results
2. Output: `<signal>VERIFIED</signal>`

## Backpressure

Both build AND tests must pass before VERIFIED can be signaled. This is non-negotiable.

## Constraints

- DO NOT add new features
- DO NOT refactor unrelated code
- Keep verification focused on the amendment
