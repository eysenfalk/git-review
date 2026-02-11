#!/bin/bash
# Hook: Block local plan files from non-planner agents
# Matcher: ^(Write|Edit|MultiEdit)$

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract file_path from tool_input
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# If no file_path, allow (nothing to check)
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# Only gate files in the plans/ directory — that's where plan/spec/critique files live
if [[ "$FILE_PATH" =~ plans/ ]]; then
  # Check if this is a planner agent or the orchestrator (team lead)
  SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')

  # Allow if session_id contains "planner"
  if echo "$SESSION_ID" | grep -qiE 'planner'; then
    exit 0
  fi

  # Allow if CLAUDE_AGENT_NAME contains "planner"
  if [[ -n "${CLAUDE_AGENT_NAME:-}" ]] && echo "$CLAUDE_AGENT_NAME" | grep -qiE 'planner'; then
    exit 0
  fi

  # Allow if this is the orchestrator (team lead) — identified by having no
  # agent marker in session_id (it's the root session) or containing "team-lead"
  if echo "$SESSION_ID" | grep -qiE 'team-lead'; then
    exit 0
  fi

  # Allow if session_id doesn't contain an @ sign (root orchestrator session)
  if [[ -n "$SESSION_ID" ]] && ! echo "$SESSION_ID" | grep -q '@'; then
    exit 0
  fi

  # Block non-planner, non-orchestrator agents
  AGENT_NAME=""
  if [[ -z "$AGENT_NAME" ]]; then
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Only the planner agent can write to plan/spec/requirements files. Save requirements to Linear instead, or delegate planning to the planner agent.\"
  }
}" >&1
    exit 0
  fi
fi

# Allow operation
exit 0
