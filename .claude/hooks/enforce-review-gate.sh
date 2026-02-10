#!/bin/bash
# Hook: Block PR creation and merges unless user has reviewed via git-review
# Matcher: ^Bash$

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

# Block: gh pr create, gh pr merge
if [[ "$FIRST_LINE" =~ gh[[:space:]]+pr[[:space:]]+(create|merge) ]]; then
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"PR creation and merges require user review first. The user must: 1) run git-review main..<branch> to review all hunks, 2) mark all hunks as reviewed, 3) confirm git-review gate check passes. Ask the user to review first.\"
  }
}"
  exit 0
fi

# Block git merge into main
if [[ "$FIRST_LINE" =~ git[[:space:]]+merge ]]; then
  CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
  if [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Merges to main require user review first. The user must run git-review to review all hunks before merging. Ask the user to review and confirm.\"
  }
}"
    exit 0
  fi
fi

# Allow operation
exit 0
