#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

message="${1:-Save}"

git add -A

if git diff --cached --quiet; then
    echo "No changes to commit."
else
    git commit -m "$message"
fi

git pull --rebase
git push
