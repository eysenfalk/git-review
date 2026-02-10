#!/bin/bash
# Hook: Protect .claude/hooks/ from modification
# Matcher: ^(Write|Edit|MultiEdit|Bash)$

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# Get tool name to determine how to extract path
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Handle Write/Edit/MultiEdit tools
if [[ "$TOOL_NAME" =~ ^(Write|Edit|MultiEdit)$ ]]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

  if [[ -n "$FILE_PATH" ]] && [[ "$FILE_PATH" =~ \.claude/hooks/ ]]; then
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"ask\",
    \"permissionDecisionReason\": \"Modifying hook file — approve if you authorized this change.\"
  }
}"
    exit 0
  fi
fi

# Handle Bash tool (check for writes to .claude/hooks/)
if [[ "$TOOL_NAME" == "Bash" ]]; then
  COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

  # Check for various ways to write to hooks directory
  if [[ "$COMMAND" =~ (echo|cat|tee|sed|awk|perl|python).*(>|>>).*.claude/hooks/ ]] || \
     [[ "$COMMAND" =~ (mv|cp).*.claude/hooks/ ]]; then
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"ask\",
    \"permissionDecisionReason\": \"Modifying hook file — approve if you authorized this change.\"
  }
}"
    exit 0
  fi
fi

# Allow operation
exit 0
