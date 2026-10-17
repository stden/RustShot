# Test Phase Prompt (BUGFIX)

You are in the Test phase of BUGFIX protocol.

## Your Mission

Add a test that would have caught this bug, then verify all tests pass.

## Input Context

1. `codev/status/{project-id}-*.md` - Bug details and fix
2. GitHub issue for reproduction steps

## Workflow

### 1. Write Regression Test

Create a test that:
- Reproduces the original bug scenario
- Verifies the fix works
- Would fail if the bug was reintroduced

Test name should be descriptive:
```typescript
it('should handle [scenario] without [bug behavior]', () => {
  // Test implementation
});
```

### 2. Run All Tests

```bash
cargo test
```

If tests fail:
- Output: `<signal>TESTS_FAIL</signal>`
- Include which tests failed

### 3. Verify Coverage

Ensure:
- [ ] New test covers the bug scenario
- [ ] Existing tests still pass
- [ ] No flaky tests introduced

### 4. Commit Test

```bash
git add <test-files>
git commit -m "test: add regression test for #{issue-number}"
```

### 5. Signal Completion

When all tests pass:
- Output: `<signal>TESTS_PASS</signal>`

## Constraints

- Test MUST cover the specific bug scenario
- Keep test focused and minimal
- DO NOT add unrelated tests
