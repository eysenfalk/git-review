#!/bin/bash
# Hook: Check for unwrap() in library code (Rust-specific)
# Matcher: ^(Write|Edit|MultiEdit)$
#
# CUSTOMIZATION:
# - This hook is Rust-specific. For other languages, check for similar panic-prone patterns:
#   - Python: bare `raise` or `assert` in production code
#   - JavaScript: `throw` without try/catch in library code
#   - Go: bare `panic()` in library code
# - Update file path patterns to match your source structure
# - Modify the pattern to check for your language's panic-prone constructs
# - Change from warning to denial if you want to enforce stricter rules

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

# CUSTOMIZATION: Update file path patterns to match your source structure
# Only check src/ files, but NOT tests
if [[ "$FILE_PATH" =~ ^src/ ]] && [[ ! "$FILE_PATH" =~ (test|tests\.rs|_test\.rs) ]]; then
  # CUSTOMIZATION: Update pattern to check for your language's panic-prone constructs
  # Rust: unwrap()
  # Python: bare raise/assert (consider: raise\s+\w+|assert\s+\w+)
  # JavaScript: throw (consider: throw\s+new)
  # Go: panic() (consider: panic\()
  if echo "$CONTENT" | grep -q '\.unwrap()'; then
    # CUSTOMIZATION: Change to "permissionDecision": "deny" for stricter enforcement
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
