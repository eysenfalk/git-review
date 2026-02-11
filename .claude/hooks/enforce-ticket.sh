#!/bin/bash
# Hook: Enforce no-ticket-no-work policy on every prompt
# Matcher: UserPromptSubmit

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract cwd
CWD=$(echo "$INPUT" | jq -r '.cwd // empty')

# If no cwd, fail open (allow)
if [[ -z "$CWD" ]]; then
  exit 0
fi

# Check if cwd is a git repo
cd "$CWD"
if ! git rev-parse --is-inside-work-tree &>/dev/null; then
  # Not a git repo, allow
  exit 0
fi

# Get current branch
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "")

# Skip detached HEAD (e.g., during rebase)
if [[ -z "$CURRENT_BRANCH" ]]; then
  exit 0
fi

# Block if on main or master
if [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
  echo "{
  \"decision\": \"block\",
  \"reason\": \"You're on $CURRENT_BRANCH. Create a feature branch with a ticket ID first (e.g., feat/eng-18-description).\"
}"
  exit 0
fi

# Check for ticket ID pattern: (eng|anx)-[0-9]+ (case-insensitive)
if ! echo "$CURRENT_BRANCH" | grep -qiE '(eng|anx)-[0-9]+'; then
  echo "{
  \"decision\": \"block\",
  \"reason\": \"Branch '$CURRENT_BRANCH' has no ticket ID. Create a Linear ticket and use format: <type>/<ticket-id>-<description> (e.g., feat/eng-18-add-hook).\"
}"
  exit 0
fi

# Allow: branch has ticket ID
exit 0
