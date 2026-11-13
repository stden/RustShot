Stage the relevant changes, create a commit, and push to `origin`.

Steps:
1. Run `git status` and `git diff --stat` to review the current worktree.
2. Stage only the files relevant to the task. Do NOT include unrelated local changes,
   secrets, `.env` files, or credentials.
3. Write a commit message using the project convention:
   - `[+]` for additions
   - `[-]` for deletions
   - `[!]` for refactoring, fixes, or improvements
4. Create the commit.
5. Push to `origin`.
6. Report the commit hash and a short summary of what was pushed.

If there are no relevant staged changes to commit, say so and stop.
