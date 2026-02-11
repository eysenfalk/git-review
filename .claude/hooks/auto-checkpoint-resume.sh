#!/bin/bash
# Auto-resume: reads latest checkpoint on session start
set -euo pipefail

CHECKPOINT_FILE=".claude/checkpoints/latest.json"

# If no checkpoint exists, nothing to do
if [ ! -f "$CHECKPOINT_FILE" ]; then
  exit 0
fi

# Read checkpoint
CHECKPOINT=$(cat "$CHECKPOINT_FILE")
BRANCH=$(echo "$CHECKPOINT" | jq -r '.git.branch // "unknown"')
TICKET=$(echo "$CHECKPOINT" | jq -r '.git.ticket // "none"')
LAST_COMMIT=$(echo "$CHECKPOINT" | jq -r '.git.last_commit // "unknown"')
TIMESTAMP=$(echo "$CHECKPOINT" | jq -r '.timestamp // "unknown"')

# Count tasks by status
COMPLETED=$(echo "$CHECKPOINT" | jq '[.tasks[] | select(.status == "completed")] | length' 2>/dev/null || echo "0")
IN_PROGRESS=$(echo "$CHECKPOINT" | jq '[.tasks[] | select(.status == "in_progress")] | length' 2>/dev/null || echo "0")
PENDING=$(echo "$CHECKPOINT" | jq '[.tasks[] | select(.status == "pending")] | length' 2>/dev/null || echo "0")

# Count uncommitted files
UNCOMMITTED=$(echo "$CHECKPOINT" | jq '.git.uncommitted_files | length' 2>/dev/null || echo "0")
UNTRACKED=$(echo "$CHECKPOINT" | jq '.git.untracked_files | length' 2>/dev/null || echo "0")

# Build summary
SUMMARY="Last checkpoint: $TIMESTAMP | Branch: $BRANCH | Ticket: $TICKET | Commit: $LAST_COMMIT"
if [ "$COMPLETED" != "0" ] || [ "$IN_PROGRESS" != "0" ] || [ "$PENDING" != "0" ]; then
  SUMMARY="$SUMMARY | Tasks: ${COMPLETED} done, ${IN_PROGRESS} active, ${PENDING} pending"
fi
if [ "$UNCOMMITTED" != "0" ] || [ "$UNTRACKED" != "0" ]; then
  SUMMARY="$SUMMARY | Uncommitted: ${UNCOMMITTED} modified, ${UNTRACKED} new"
fi

echo "$SUMMARY"
exit 0
