# Fix Phase Prompt (BUGFIX)

You are in the Fix phase of BUGFIX protocol.

## Your Mission

Apply the fix for the diagnosed bug. Keep changes minimal and focused.

## Input Context

1. `codev/status/{project-id}-*.md` - Diagnosis results
2. GitHub issue for context

## Workflow

### 1. Review Diagnosis

From status file, confirm:
- Root cause is identified
- Affected files are listed
- Proposed fix is documented

### 2. Implement Fix

Apply the minimal fix:
1. Change ONLY what's necessary
2. Follow existing code patterns
3. Add comments explaining non-obvious changes

### 3. Verify Fix Compiles

```bash
cargo check
```

If build fails:
- Fix build errors
- Output: `<signal>FIX_FAILED</signal>` if can't resolve

### 4. Commit Fix

```bash
git add <files>
git commit -m "fix: {brief description}

Fixes #{issue-number}"
```

### 5. Signal Completion

When fix is applied and builds:
- Output: `<signal>FIX_APPLIED</signal>`

## Constraints

- **Minimal changes only** - fix the bug, nothing else
- DO NOT refactor surrounding code
- DO NOT add unrelated improvements
- Keep the diff small and focused
