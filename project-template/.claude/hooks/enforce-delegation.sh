#!/bin/bash
# Hook: Remind orchestrators to delegate implementation to agents
# Matcher: ^(Edit|Write)$
#
# CUSTOMIZATION:
# - Update source file patterns to match your project structure (e.g., src/, lib/, app/)
# - Add/remove exclusion patterns based on what orchestrators CAN edit
# - Modify the agent suggestions in the message to match your agent roster
# - Adjust env var checks if using different agent identification

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract file_path from tool_input
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# If no file_path, allow (nothing to check)
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# CUSTOMIZATION: Update these patterns to match your source file structure
# Check if this is a source/test/config file that should be delegated
if [[ "$FILE_PATH" =~ ^(src/|lib/|app/|tests?/|Cargo\.toml|package\.json|go\.mod|requirements\.txt) ]]; then
  # CUSTOMIZATION: Exclude patterns - files orchestrators CAN edit directly
  if [[ "$FILE_PATH" =~ (\.claude/|README\.md|plans/|docs/) ]]; then
    exit 0
  fi

  # CUSTOMIZATION: Update env var names if using different agent identification
  # Check if we're a spawned agent (heuristic: CLAUDE_SPAWNED_BY or CLAUDE_AGENT_NAME)
  # If these env vars exist, we're likely a spawned agent, so skip the warning
  if [[ -n "${CLAUDE_SPAWNED_BY:-}" ]] || [[ -n "${CLAUDE_AGENT_NAME:-}" ]]; then
    exit 0
  fi

  # CUSTOMIZATION: Update agent suggestions to match your agent roster
  # Not a spawned agent — remind orchestrator to delegate
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"additionalContext\": \"⚠️  ORCHESTRATOR REMINDER: You are editing $FILE_PATH directly. Consider delegating implementation work to spawned agents instead:\\n\\n- Use 'coder' agent for standard implementation tasks\\n- Use 'senior-coder' agent for complex refactoring or design patterns\\n- Use 'tester' agent for test implementation\\n\\nDelegation improves parallelism and specialization. If you're fixing a quick typo or this is a special case, you may proceed.\"
  }
}" >&1
  exit 0
fi

# Allow operation (not a source file)
exit 0
