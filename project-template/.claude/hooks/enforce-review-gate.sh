#!/bin/bash
# Hook: Block PR creation and merges unless user has reviewed changes
# Matcher: ^Bash$
#
# CUSTOMIZATION:
# - Update find_git_review() to find your review tool binary
# - Modify is_review_green() to check your review status (git-review specific)
# - Adjust the review status check logic if using a different review tool
# - Change error messages to reflect your review process

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

# CUSTOMIZATION: Update this function to find your review tool
# Helper: find the git-review binary (or your custom review tool)
find_git_review() {
  if command -v git-review &>/dev/null; then
    echo "git-review"
  elif [[ -x "./target/release/git-review" ]]; then
    echo "./target/release/git-review"
  elif [[ -x "./target/debug/git-review" ]]; then
    echo "./target/debug/git-review"
  fi
}

# CUSTOMIZATION: Update this function to check your review status
# Helper: check if all changes are reviewed for a branch vs main
is_review_green() {
  local BRANCH="$1"
  local GIT_REVIEW
  GIT_REVIEW=$(find_git_review)

  if [[ -z "$GIT_REVIEW" ]]; then
    return 1
  fi

  local STATUS_OUTPUT
  STATUS_OUTPUT=$($GIT_REVIEW status "main..${BRANCH}" 2>/dev/null || echo "")

  # CUSTOMIZATION: Update this check based on your review tool's output
  if echo "$STATUS_OUTPUT" | grep -q "All hunks reviewed"; then
    return 0
  fi

  return 1
}

# Helper: resolve the branch to check for a gh pr command
resolve_pr_branch() {
  local CMD_LINE="$1"
  # If gh pr merge <number>, look up the PR's head branch
  local PR_NUM
  PR_NUM=$(echo "$CMD_LINE" | grep -oP 'gh\s+pr\s+(create|merge)\s+\K[0-9]+' || echo "")

  if [[ -n "$PR_NUM" ]]; then
    gh pr view "$PR_NUM" --json headRefName -q '.headRefName' 2>/dev/null || echo ""
    return
  fi

  # Otherwise use current branch
  git rev-parse --abbrev-ref HEAD 2>/dev/null || echo ""
}

# Check: gh pr create, gh pr merge
if [[ "$FIRST_LINE" =~ gh[[:space:]]+pr[[:space:]]+(create|merge) ]]; then
  BRANCH=$(resolve_pr_branch "$FIRST_LINE")
  if [[ -n "$BRANCH" && "$BRANCH" != "HEAD" ]] && is_review_green "$BRANCH"; then
    exit 0
  fi
  # CUSTOMIZATION: Update error message to reflect your review process
  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"PR creation and merges require user review first. The user must review all changes and confirm review status shows complete. Ask the user to review first.\"
  }
}"
  exit 0
fi

# Check: git merge into main
if [[ "$FIRST_LINE" =~ git[[:space:]]+merge ]]; then
  CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
  if [[ "$CURRENT_BRANCH" =~ ^(main|master)$ ]]; then
    MERGE_BRANCH=$(echo "$FIRST_LINE" | grep -oP 'git\s+merge\s+\K\S+' || echo "")
    if [[ -n "$MERGE_BRANCH" ]] && is_review_green "$MERGE_BRANCH"; then
      exit 0
    fi
    # CUSTOMIZATION: Update error message to reflect your review process
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"permissionDecision\": \"deny\",
    \"permissionDecisionReason\": \"Merges to main require user review first. The user must review all changes before merging. Ask the user to review and confirm.\"
  }
}"
    exit 0
  fi
fi

# Allow operation
exit 0
