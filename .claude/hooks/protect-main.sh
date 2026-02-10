#!/bin/bash
# Hook: Block direct commits/pushes to main
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

# Get current branch
CURRENT_BRANCH=$(git -C "${PWD}" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

# Check for git push to main/master
if [[ "$FIRST_LINE" =~ git[[:space:]]+push ]] && [[ "$FIRST_LINE" =~ (main|master) ]]; then
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Direct pushes to main/master are not allowed. Create a feature branch and open a PR instead.\"
  }
}"
  exit 0
fi

# Check for git commit on main/master branch
if [[ "$FIRST_LINE" =~ git[[:space:]]+commit ]] && [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Cannot commit directly to $CURRENT_BRANCH. Create a feature branch first: git checkout -b feature/your-feature-name\"
  }
}"
  exit 0
fi

# Allow git checkout/switch to main (read-only operations)
# This is OK - we only block commits/pushes

# Allow operation
exit 0
