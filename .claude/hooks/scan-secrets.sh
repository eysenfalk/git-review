#!/bin/bash
# Hook: Scan for potential secrets in code being written/edited
# Matcher: ^(Write|Edit|MultiEdit)$
# Type: ADVISORY (warns but does not block)

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# Get tool name
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Only process Write/Edit/MultiEdit tools
if [[ ! "$TOOL_NAME" =~ ^(Write|Edit|MultiEdit)$ ]]; then
  exit 0
fi

# Extract file path
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Skip if no file path
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

# Skip non-code files (documentation, images, etc.)
if [[ "$FILE_PATH" =~ \.(md|txt|rst|svg|png|jpg|jpeg|gif|ico|woff|ttf)$ ]]; then
  exit 0
fi

# Skip test files
if [[ "$FILE_PATH" =~ /tests/ ]] || \
   [[ "$FILE_PATH" =~ _test\. ]] || \
   [[ "$FILE_PATH" =~ \.test\. ]] || \
   [[ "$FILE_PATH" =~ test_ ]]; then
  exit 0
fi

# Extract content based on tool type
CONTENT=""
case "$TOOL_NAME" in
  Write)
    CONTENT=$(echo "$INPUT" | jq -r '.tool_input.content // empty')
    ;;
  Edit)
    CONTENT=$(echo "$INPUT" | jq -r '.tool_input.new_string // empty')
    ;;
  MultiEdit)
    # For MultiEdit, concatenate all new_string values
    CONTENT=$(echo "$INPUT" | jq -r '.tool_input.edits[]?.new_string // empty' | tr '\n' ' ')
    ;;
esac

# Skip if content is empty
if [[ -z "$CONTENT" ]]; then
  exit 0
fi

# Array to collect detected patterns
DETECTED_PATTERNS=()

# Pattern 1: AWS access keys (AKIA followed by 16 alphanumeric characters)
if echo "$CONTENT" | grep -qE 'AKIA[0-9A-Z]{16}'; then
  DETECTED_PATTERNS+=("AWS access key (AKIA...)")
fi

# Pattern 2: API key assignments (must look like assignment to reduce false positives)
if echo "$CONTENT" | grep -qiE '(api[_-]?key|apikey)\s*[:=]\s*["\047][A-Za-z0-9_-]{20,}'; then
  DETECTED_PATTERNS+=("API key assignment")
fi

# Pattern 3: Password assignments (must look like assignment)
if echo "$CONTENT" | grep -qiE '(password|passwd|pwd)\s*[:=]\s*["\047][^\"\047]{8,}'; then
  DETECTED_PATTERNS+=("password assignment")
fi

# Pattern 4: Private key headers (use -- to separate pattern from options)
if echo "$CONTENT" | grep -qE -- '-----BEGIN.*PRIVATE KEY-----'; then
  DETECTED_PATTERNS+=("private key header")
fi

# Pattern 5: Token/secret/credential assignments
if echo "$CONTENT" | grep -qiE '(token|secret|credential|auth)\s*[:=]\s*["\047][A-Za-z0-9+/=_-]{20,}'; then
  DETECTED_PATTERNS+=("token/secret/credential assignment")
fi

# Pattern 6: Database connection strings with embedded passwords
if echo "$CONTENT" | grep -qiE '(mysql|postgres|postgresql|mongodb|redis)://[^:]+:[^@]+@'; then
  DETECTED_PATTERNS+=("database connection string with embedded password")
fi

# If patterns detected, emit advisory warning
if [[ ${#DETECTED_PATTERNS[@]} -gt 0 ]]; then
  # Join detected patterns with ", "
  PATTERNS_STR=$(IFS=", "; echo "${DETECTED_PATTERNS[*]}")

  echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"additionalContext\": \"⚠️  Possible secret detected in $FILE_PATH: $PATTERNS_STR. Verify this is not a real credential before proceeding.\"
  }
}"
fi

# Always exit 0 (advisory only, never block)
exit 0
