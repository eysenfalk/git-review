#!/bin/bash
# Test suite for enforce-agent-cap.sh hook

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOK_SCRIPT="$SCRIPT_DIR/../../.claude/hooks/enforce-agent-cap.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Test helper functions
test_start() {
  echo -n "Test $1: $2 ... "
  TESTS_RUN=$((TESTS_RUN + 1))
}

test_pass() {
  echo -e "${GREEN}PASS${NC}"
  TESTS_PASSED=$((TESTS_PASSED + 1))
}

test_fail() {
  echo -e "${RED}FAIL${NC}"
  echo "  Expected: $1"
  echo "  Got: $2"
  TESTS_FAILED=$((TESTS_FAILED + 1))
}

# Test 1: First spawn allowed (no team configs exist)
test_1() {
  test_start 1 "First spawn allowed (no team configs exist)"

  # Create temp home directory
  TEMP_HOME=$(mktemp -d)
  trap "rm -rf $TEMP_HOME" EXIT

  # Create mock Task input JSON
  MOCK_INPUT='{"tool_input":{"prompt":"Test agent spawn","subagent_type":"coder"}}'

  # Run hook with temp HOME
  OUTPUT=$(echo "$MOCK_INPUT" | HOME="$TEMP_HOME" bash "$HOOK_SCRIPT" 2>&1 || true)
  EXIT_CODE=$?

  # Should allow (exit 0, no deny output)
  if [[ $EXIT_CODE -eq 0 ]] && [[ ! "$OUTPUT" =~ "permissionDecision" ]]; then
    test_pass
  else
    test_fail "exit 0 with no deny" "exit $EXIT_CODE, output: $OUTPUT"
  fi

  rm -rf "$TEMP_HOME"
  trap - EXIT
}

# Test 2: Spawn allowed when < 3 members
test_2() {
  test_start 2 "Spawn allowed when < 3 members"

  # Create temp home directory
  TEMP_HOME=$(mktemp -d)
  TEAMS_DIR="$TEMP_HOME/.claude/teams"
  mkdir -p "$TEAMS_DIR/team1"

  # Create mock team config with 2 members
  cat > "$TEAMS_DIR/team1/config.json" <<EOF
{
  "name": "team1",
  "members": [
    {"name": "agent1", "agentId": "id1", "agentType": "coder"},
    {"name": "agent2", "agentId": "id2", "agentType": "tester"}
  ]
}
EOF

  # Create mock Task input JSON
  MOCK_INPUT='{"tool_input":{"prompt":"Test agent spawn","subagent_type":"coder"}}'

  # Run hook with temp HOME
  OUTPUT=$(echo "$MOCK_INPUT" | HOME="$TEMP_HOME" bash "$HOOK_SCRIPT" 2>&1 || true)
  EXIT_CODE=$?

  # Should allow (exit 0, no deny output)
  if [[ $EXIT_CODE -eq 0 ]] && [[ ! "$OUTPUT" =~ "permissionDecision" ]]; then
    test_pass
  else
    test_fail "exit 0 with no deny" "exit $EXIT_CODE, output: $OUTPUT"
  fi

  rm -rf "$TEMP_HOME"
}

# Test 3: Spawn denied when >= 3 members
test_3() {
  test_start 3 "Spawn denied when >= 3 members"

  # Create temp home directory
  TEMP_HOME=$(mktemp -d)
  TEAMS_DIR="$TEMP_HOME/.claude/teams"
  mkdir -p "$TEAMS_DIR/team1"

  # Create mock team config with 3 members
  cat > "$TEAMS_DIR/team1/config.json" <<EOF
{
  "name": "team1",
  "members": [
    {"name": "agent1", "agentId": "id1", "agentType": "coder"},
    {"name": "agent2", "agentId": "id2", "agentType": "tester"},
    {"name": "agent3", "agentId": "id3", "agentType": "reviewer"}
  ]
}
EOF

  # Create mock Task input JSON
  MOCK_INPUT='{"tool_input":{"prompt":"Test agent spawn","subagent_type":"coder"}}'

  # Run hook with temp HOME
  OUTPUT=$(echo "$MOCK_INPUT" | HOME="$TEMP_HOME" bash "$HOOK_SCRIPT" 2>&1 || true)
  EXIT_CODE=$?

  # Should deny (exit 0 but with deny in output)
  if [[ $EXIT_CODE -eq 0 ]] && [[ "$OUTPUT" =~ "permissionDecision" ]] && [[ "$OUTPUT" =~ "deny" ]]; then
    test_pass
  else
    test_fail "exit 0 with deny output" "exit $EXIT_CODE, output: $OUTPUT"
  fi

  rm -rf "$TEMP_HOME"
}

# Test 4: Spawn allowed after a member is removed (simulating teammate shutdown)
test_4() {
  test_start 4 "Spawn allowed after a member is removed"

  # Create temp home directory
  TEMP_HOME=$(mktemp -d)
  TEAMS_DIR="$TEMP_HOME/.claude/teams"
  mkdir -p "$TEAMS_DIR/team1"

  # Create mock team config with 2 members (one was removed)
  cat > "$TEAMS_DIR/team1/config.json" <<EOF
{
  "name": "team1",
  "members": [
    {"name": "agent1", "agentId": "id1", "agentType": "coder"},
    {"name": "agent2", "agentId": "id2", "agentType": "tester"}
  ]
}
EOF

  # Create mock Task input JSON
  MOCK_INPUT='{"tool_input":{"prompt":"Test agent spawn","subagent_type":"reviewer"}}'

  # Run hook with temp HOME
  OUTPUT=$(echo "$MOCK_INPUT" | HOME="$TEMP_HOME" bash "$HOOK_SCRIPT" 2>&1 || true)
  EXIT_CODE=$?

  # Should allow (exit 0, no deny output)
  if [[ $EXIT_CODE -eq 0 ]] && [[ ! "$OUTPUT" =~ "permissionDecision" ]]; then
    test_pass
  else
    test_fail "exit 0 with no deny" "exit $EXIT_CODE, output: $OUTPUT"
  fi

  rm -rf "$TEMP_HOME"
}

# Run all tests
echo "================================"
echo "Agent Cap Hook Test Suite"
echo "================================"
echo

test_1
test_2
test_3
test_4

echo
echo "================================"
echo "Test Results"
echo "================================"
echo "Tests run: $TESTS_RUN"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
if [[ $TESTS_FAILED -gt 0 ]]; then
  echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
  exit 1
else
  echo -e "Tests failed: $TESTS_FAILED"
  echo
  echo -e "${GREEN}All tests passed!${NC}"
  exit 0
fi
