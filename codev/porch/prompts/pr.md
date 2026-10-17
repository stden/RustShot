# PR Phase Prompt (BUGFIX)

You are in the PR phase of BUGFIX protocol.

## Your Mission

Create a pull request for the bug fix.

## Input Context

1. `codev/status/{project-id}-*.md` - All bug fix details
2. GitHub issue number

## Workflow

### 1. Final Verification

Ensure:
```bash
cargo check  # Must pass
cargo test       # Must pass
git status     # No uncommitted changes
```

### 2. Create Pull Request

```bash
gh pr create \
  --title "fix: {brief description}" \
  --body "$(cat <<'EOF'
## Summary

Fixes #{issue-number}

## Root Cause

{Brief explanation of the bug cause}

## Fix

{Brief explanation of the fix}

## Test Plan

- [x] Added regression test
- [x] All existing tests pass
- [x] Manually verified fix

## Changes

- `file1.rs` - {what changed}
- `test.rs` - Added regression test
EOF
)"
```

### 3. Link PR to Issue

The PR title with "Fixes #N" will auto-link when merged.

### 4. Signal Completion

When PR is created:
1. Output the PR URL
2. Output: `<signal>PR_CREATED</signal>`

## PR Quality Checklist

- [ ] Title is clear and starts with "fix:"
- [ ] Body explains root cause
- [ ] Test plan is documented
- [ ] Issue is referenced
- [ ] Diff is minimal and focused

## Output Format

```
<signal>PR_CREATED</signal>

PR: {url}
Fixes: #{issue-number}

Ready for review.
```
