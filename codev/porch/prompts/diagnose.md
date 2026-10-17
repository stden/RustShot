# Diagnose Phase Prompt (BUGFIX)

You are in the Diagnose phase of BUGFIX protocol.

## Your Mission

Identify the root cause of the bug reported in the GitHub issue.

## Input Context

1. **GitHub Issue**: Read the issue details (number provided in status file)
2. `codev/status/{project-id}-*.md` - Bug tracking state

## Workflow

### 1. Understand the Bug

From the GitHub issue:
- What is the expected behavior?
- What is the actual behavior?
- Steps to reproduce
- Any error messages or logs

### 2. Reproduce the Bug

If possible:
```bash
# Run the reproduction steps
# Document what you observe
```

### 3. Identify Root Cause

Analyze the codebase to find:
- Which file(s) contain the bug?
- What is the specific cause?
- Why does it happen?

### 4. Document Findings

Update status file:
```markdown
## Bug Diagnosis

**Issue**: #{issue-number}
**Root Cause**: {description}
**Affected Files**:
- path/to/file.rs (line X-Y)

**Analysis**:
{detailed explanation of why the bug occurs}

**Proposed Fix**:
{high-level description of the fix}
```

### 5. Signal Completion

If root cause found:
- Output: `<signal>ROOT_CAUSE_FOUND</signal>`

If more info needed from issue author:
- Add comment to GitHub issue
- Output: `<signal>NEEDS_MORE_INFO</signal>`

## Constraints

- DO NOT start fixing yet
- Document your findings clearly
- If can't reproduce, signal NEEDS_MORE_INFO
