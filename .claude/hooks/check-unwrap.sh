#!/bin/bash
# Hook: Check for unwrap() in library code
# Matcher: ^(Write|Edit|MultiEdit)$

set -euo pipefail

# Read JSON input from stdin
INPUT=$(cat)

# Extract file_path and content
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
CONTENT=$(echo "$INPUT" | jq -r '.tool_input.content // .tool_input.new_string // empty')

# If no file_path or content, allow
if [[ -z "$FILE_PATH" ]] || [[ -z "$CONTENT" ]]; then
  exit 0
fi

# Only check src/ files, but NOT tests
if [[ "$FILE_PATH" =~ ^src/ ]] && [[ ! "$FILE_PATH" =~ (test|tests\.rs|_test\.rs) ]]; then
  # Check if content contains unwrap()
  if echo "$CONTENT" | grep -q '\.unwrap()'; then
    # Warn but don't block (they might be writing tests or have a good reason)
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"additionalContext\": \"⚠️  Warning: unwrap() detected in library code at $FILE_PATH. Consider using proper error handling with Result<> and the ? operator instead. unwrap() should only be used in tests.\"
  }
}" >&1
    exit 0
  fi
fi

# Allow operation
exit 0
