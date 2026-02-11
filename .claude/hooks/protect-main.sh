#!/bin/bash
# Hook: Block direct modifications to main/master
# Matcher: ^Bash$
# Catches: push, commit, merge, update-ref, reset, rebase on main

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract command
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# If no command, allow
if [[ -z "$COMMAND" ]]; then
  exit 0
fi

# Extract only the actual command (first line, before heredoc/pipe content)
FIRST_LINE=$(echo "$COMMAND" | head -1)

# Get current branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

deny() {
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"$1\"
  }
}"
  exit 0
}

# 1. Block git push with explicit main/master target
if [[ "$FIRST_LINE" =~ git[[:space:]]+push ]] && [[ "$FIRST_LINE" =~ (main|master) ]]; then
  deny "Direct pushes to main/master are not allowed. Create a feature branch and open a PR instead."
fi

# 2. Block git push when ON main (even without explicit branch name)
if [[ "$FIRST_LINE" =~ git[[:space:]]+push ]] && [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
  deny "Cannot push while on $CURRENT_BRANCH. Create a feature branch and open a PR instead."
fi

# 3. Block git commit on main/master branch
if [[ "$FIRST_LINE" =~ git[[:space:]]+commit ]] && [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
  deny "Cannot commit directly to $CURRENT_BRANCH. Create a feature branch first."
fi

# 4. Block git update-ref targeting main/master refs
if [[ "$FIRST_LINE" =~ git[[:space:]]+update-ref ]] && [[ "$FIRST_LINE" =~ refs/heads/(main|master) ]]; then
  deny "Cannot modify main/master ref directly via update-ref. Use a feature branch and PR instead."
fi

# 5. Block git reset on main/master branch
if [[ "$FIRST_LINE" =~ git[[:space:]]+reset ]] && [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
  deny "Cannot reset $CURRENT_BRANCH. Create a feature branch for changes."
fi

# 6. Block git rebase on main/master branch (advancing main via rebase)
if [[ "$FIRST_LINE" =~ git[[:space:]]+rebase ]] && [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
  deny "Cannot rebase while on $CURRENT_BRANCH. Use a feature branch and PR instead."
fi

# Allow read-only operations: checkout, switch, log, diff, status, pull
exit 0
