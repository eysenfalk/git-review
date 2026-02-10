#!/bin/bash
# Hook: Suggest Serena for source file operations
# Matcher: ^(Read|Grep|Bash)$

set -euo pipefail

# Check if jq is available, fail open if not
if ! command -v jq &> /dev/null; then
  exit 0
fi

# Read JSON input from stdin
INPUT=$(cat)

# Get tool name
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Whitelist extensions (non-source files)
WHITELIST_EXTS="\.(md|json|toml|txt|yml|yaml|sh|lock)$"

# Handle Read tool
if [[ "$TOOL_NAME" == "Read" ]]; then
  FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

  # Check if it's a source file in src/ or tests/
  if [[ "$FILE_PATH" =~ ^(src/|tests/).*\.(rs|ts|js|py|go|java)$ ]]; then
    # Not whitelisted, suggest Serena
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"additionalContext\": \"💡 Consider using Serena get_symbols_overview for this file first (50-75% token savings). Use ToolSearch('+serena get_symbols') to load.\"
  }
}" >&1
    exit 0
  fi
fi

# Handle Grep tool
if [[ "$TOOL_NAME" == "Grep" ]]; then
  PATTERN=$(echo "$INPUT" | jq -r '.tool_input.pattern // empty')
  PATH_ARG=$(echo "$INPUT" | jq -r '.tool_input.path // empty')

  # Check if pattern looks like a simple symbol (alphanumeric + underscore)
  if [[ "$PATTERN" =~ ^[a-zA-Z_][a-zA-Z0-9_]*$ ]]; then
    # Check if searching in source directories
    if [[ -z "$PATH_ARG" ]] || [[ "$PATH_ARG" =~ ^(src/|tests/) ]]; then
      echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"additionalContext\": \"💡 Consider using Serena find_symbol instead of Grep for symbol lookup. Use ToolSearch('+serena find_symbol') to load.\"
  }
}" >&1
      exit 0
    fi
  fi
fi

# Handle Bash tool (cat/head/tail on source files)
if [[ "$TOOL_NAME" == "Bash" ]]; then
  COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

  # Extract first line (before heredoc/pipe)
  FIRST_LINE=$(echo "$COMMAND" | head -1)

  # Check for cat/head/tail on source files
  if [[ "$FIRST_LINE" =~ (cat|head|tail)[[:space:]]+(src/|tests/).*\.(rs|ts|js|py|go|java) ]]; then
    echo "{
  \"hookSpecificOutput\": {
    \"hookEventName\": \"PreToolUse\",
    \"additionalContext\": \"💡 Consider using Serena get_symbols_overview for this file first (50-75% token savings). Use ToolSearch('+serena get_symbols') to load.\"
  }
}" >&1
    exit 0
  fi
fi

# Allow operation
exit 0
