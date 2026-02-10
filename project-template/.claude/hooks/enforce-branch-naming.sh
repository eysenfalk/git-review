#!/bin/bash
# Hook: Enforce branch naming convention on git commit/push
# Matcher: ^Bash$
#
# CUSTOMIZATION:
# - Update PATTERN regex to match your branch naming convention
# - Modify the types list (feat, fix, etc.) to match your team's prefixes
# - Change ticket ID pattern (eng|anx) to match your ticket system (e.g., jira, linear, github)
# - Adjust error message to reflect your convention

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

# Only check on git commit or git push
if [[ ! "$FIRST_LINE" =~ git[[:space:]]+(commit|push) ]]; then
  exit 0
fi

# Get current branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

# Skip if on main (protect-main.sh handles that)
if [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
  exit 0
fi

# Skip detached HEAD
if [[ "$CURRENT_BRANCH" == "HEAD" ]]; then
  exit 0
fi

# CUSTOMIZATION: Update this pattern to match your branch naming convention
# Format: <type>/<ticket-id>-<description>
# Types: feat, fix, refactor, docs, test, chore
# Ticket ID pattern: your ticket system prefix (e.g., eng, jira, lin, gh)
PATTERN="^(feat|fix|refactor|docs|test|chore)/(eng|anx|jira|lin|gh)-[0-9]+-[a-z0-9-]+$"

if ! echo "$CURRENT_BRANCH" | grep -qE "$PATTERN"; then
  # CUSTOMIZATION: Update error message to reflect your naming convention
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Branch name '$CURRENT_BRANCH' does not follow convention: <type>/<ticket-id>-<description> (e.g. feat/eng-4-add-feature). Types: feat, fix, refactor, docs, test, chore. Every branch must have a ticket.\"
  }
}"
  exit 0
fi

# Allow operation
exit 0
