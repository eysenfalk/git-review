#!/bin/bash
# Hook Test Suite - Comprehensive testing of all enforcement hooks

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
PASS=0
FAIL=0
SKIP=0

# Project root
PROJECT_ROOT="/home/feysen/dev/personal/repos/git-review"
HOOKS_DIR="$PROJECT_ROOT/.claude/hooks"

# Temp directory for test artifacts
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

# Helper: Create a JSON input for a tool call
create_tool_input() {
    local tool_name="$1"
    local tool_input="$2"
    local transcript_path="${3:-}"

    cat <<EOF
{
  "tool_name": "$tool_name",
  "tool_input": $tool_input,
  "transcript_path": "$transcript_path",
  "session_id": "test-session"
}
EOF
}

# Helper: Create a mock transcript with agent spawn evidence
create_agent_transcript() {
    local transcript_path="$1"
    cat <<'EOF' > "$transcript_path"
{"name":"Task","type":"tool_use"}
{"name":"TeamCreate","type":"tool_use"}
EOF
}

# Helper: Create an empty transcript
create_empty_transcript() {
    local transcript_path="$1"
    touch "$transcript_path"
}

# Helper: Create a transcript with source edits but no memory_store
create_edit_transcript_no_memory() {
    local transcript_path="$1"
    cat <<'EOF' > "$transcript_path"
{"tool_name":"Write","tool_input":{"file_path":"src/main.rs","content":"..."}}
{"tool_name":"Edit","tool_input":{"file_path":"tests/test.rs"}}
EOF
}

# Helper: Create a transcript with source edits AND memory_store
create_edit_transcript_with_memory() {
    local transcript_path="$1"
    cat <<'EOF' > "$transcript_path"
{"tool_name":"Write","tool_input":{"file_path":"src/main.rs","content":"..."}}
{"name":"mcp__claude-flow__memory_store","type":"tool_use"}
EOF
}

# Helper: Test a hook
test_hook() {
    local test_name="$1"
    local hook_script="$2"
    local input_json="$3"
    local expected_decision="$4"  # "deny", "ask", "allow", "advisory"

    # Run hook and capture output
    local output
    output=$(echo "$input_json" | bash "$hook_script" 2>&1 || true)

    # Check result
    if [[ "$expected_decision" == "allow" ]]; then
        # For allow, we expect NO output or empty hookSpecificOutput
        if [[ -z "$output" ]]; then
            echo -e "${GREEN}✓ PASS${NC}: $test_name (allowed with no output)"
            PASS=$((PASS + 1))
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $test_name"
            echo "  Expected: allow (no output)"
            echo "  Got output: $output"
            FAIL=$((FAIL + 1))
            return 1
        fi
    elif [[ "$expected_decision" == "advisory" ]]; then
        # For advisory, expect additionalContext but no permissionDecision
        if echo "$output" | jq -e '.hookSpecificOutput.additionalContext' &>/dev/null && \
           ! echo "$output" | jq -e '.hookSpecificOutput.permissionDecision' &>/dev/null; then
            local context
            context=$(echo "$output" | jq -r '.hookSpecificOutput.additionalContext')
            echo -e "${GREEN}✓ PASS${NC}: $test_name (advisory: $context)"
            PASS=$((PASS + 1))
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $test_name"
            echo "  Expected: advisory (additionalContext only)"
            echo "  Got: $output"
            FAIL=$((FAIL + 1))
            return 1
        fi
    else
        # For deny/ask, check permissionDecision field
        local decision
        decision=$(echo "$output" | jq -r '.hookSpecificOutput.permissionDecision // empty')

        if [[ "$decision" == "$expected_decision" ]]; then
            local reason
            reason=$(echo "$output" | jq -r '.hookSpecificOutput.permissionDecisionReason // empty')
            echo -e "${GREEN}✓ PASS${NC}: $test_name ($decision: $reason)"
            PASS=$((PASS + 1))
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $test_name"
            echo "  Expected: $expected_decision"
            echo "  Got: $decision"
            echo "  Full output: $output"
            FAIL=$((FAIL + 1))
            return 1
        fi
    fi
}

echo "=========================================="
echo "Hook Test Suite"
echo "=========================================="
echo ""

# =========================================
# Test 1: enforce-orchestrator-delegation-v2.sh
# =========================================
echo "Testing: enforce-orchestrator-delegation-v2.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/enforce-orchestrator-delegation-v2.sh"

# Test 1.1: DENY - Write to src/main.rs with no transcript
test_hook \
    "1.1: Deny Write to src/main.rs (no transcript)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/main.rs","content":"code"}' '')" \
    "deny"

# Test 1.2: DENY - Write to tests/test.rs with empty transcript
EMPTY_TRANSCRIPT="$TEST_DIR/empty_transcript.jsonl"
create_empty_transcript "$EMPTY_TRANSCRIPT"
test_hook \
    "1.2: Deny Write to tests/test.rs (empty transcript)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"tests/test.rs","content":"test"}' "$EMPTY_TRANSCRIPT")" \
    "deny"

# Test 1.3: DENY - Write to Cargo.toml with no agents
test_hook \
    "1.3: Deny Write to Cargo.toml (no agents)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"Cargo.toml","content":"[package]"}' '')" \
    "deny"

# Test 1.4: DENY - Write to absolute path with no agents (bypass fix)
test_hook \
    "1.4: Deny Write to absolute path (bypass fix)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"/home/user/project/src/main.rs","content":"code"}' '')" \
    "deny"

# Test 1.5: DENY - Bash sed editing src/main.rs with no agents
test_hook \
    "1.5: Deny Bash sed on src/main.rs (no agents)" \
    "$HOOK" \
    "$(create_tool_input 'Bash' '{"command":"sed -i \"s/x/y/\" src/main.rs"}' '')" \
    "deny"

# Test 1.6: ALLOW - Write to src/main.rs with agent spawn in transcript
AGENT_TRANSCRIPT="$TEST_DIR/agent_transcript.jsonl"
create_agent_transcript "$AGENT_TRANSCRIPT"
test_hook \
    "1.6: Allow Write to src/main.rs (agents spawned)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/main.rs","content":"code"}' "$AGENT_TRANSCRIPT")" \
    "allow"

# Test 1.7: ALLOW - Write to .claude/hooks/test.sh (whitelisted)
test_hook \
    "1.7: Allow Write to .claude/hooks/test.sh (whitelisted)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":".claude/hooks/test.sh","content":"#!/bin/bash"}' '')" \
    "allow"

# Test 1.8: ALLOW - Write to README.md (not src/tests)
test_hook \
    "1.8: Allow Write to README.md (not enforced)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"README.md","content":"# Readme"}' '')" \
    "allow"

# Test 1.9: ALLOW - Write to docs/workflow.md (whitelisted)
test_hook \
    "1.9: Allow Write to docs/workflow.md (whitelisted)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"docs/workflow.md","content":"# Workflow"}' '')" \
    "allow"

# Test 1.10: DENY - Write to src/main.rs with missing transcript file (fail-secure)
test_hook \
    "1.10: Deny Write with missing transcript (fail-secure)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/main.rs","content":"code"}' '/nonexistent/transcript.jsonl')" \
    "deny"

echo ""

# =========================================
# Test 2: enforce-visible-agents.sh
# =========================================
echo "Testing: enforce-visible-agents.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/enforce-visible-agents.sh"

# Test 2.1: DENY - Task with subagent_type but no team_name
test_hook \
    "2.1: Deny Task without team_name" \
    "$HOOK" \
    "$(create_tool_input 'Task' '{"subagent_type":"coder","prompt":"Write code"}')" \
    "deny"

# Test 2.2: ALLOW - Task with both subagent_type and team_name
test_hook \
    "2.2: Allow Task with team_name" \
    "$HOOK" \
    "$(create_tool_input 'Task' '{"subagent_type":"coder","team_name":"my-team","prompt":"Write code"}')" \
    "allow"

# Test 2.3: ALLOW - Task with no subagent_type (non-agent Task)
test_hook \
    "2.3: Allow Task without subagent_type" \
    "$HOOK" \
    "$(create_tool_input 'Task' '{"prompt":"Some task"}')" \
    "allow"

echo ""

# =========================================
# Test 3: enforce-serena-usage.sh
# =========================================
echo "Testing: enforce-serena-usage.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/enforce-serena-usage.sh"

# Test 3.1: ADVISORY - Read of src/main.rs
test_hook \
    "3.1: Advisory Read of src/main.rs (suggest Serena)" \
    "$HOOK" \
    "$(create_tool_input 'Read' '{"file_path":"src/main.rs"}')" \
    "advisory"

# Test 3.2: ADVISORY - Grep for simple identifier in src/
test_hook \
    "3.2: Advisory Grep for symbol (suggest find_symbol)" \
    "$HOOK" \
    "$(create_tool_input 'Grep' '{"pattern":"MyStruct","path":"src/"}')" \
    "advisory"

# Test 3.3: ALLOW - Read of README.md (non-source)
test_hook \
    "3.3: Allow Read of README.md (non-source)" \
    "$HOOK" \
    "$(create_tool_input 'Read' '{"file_path":"README.md"}')" \
    "allow"

# Test 3.4: ALLOW - Read of .claude/settings.json (non-source)
test_hook \
    "3.4: Allow Read of .claude/settings.json" \
    "$HOOK" \
    "$(create_tool_input 'Read' '{"file_path":".claude/settings.json"}')" \
    "allow"

# Test 3.5: ADVISORY - Bash cat on src/main.rs
test_hook \
    "3.5: Advisory Bash cat on src/main.rs" \
    "$HOOK" \
    "$(create_tool_input 'Bash' '{"command":"cat src/main.rs"}')" \
    "advisory"

echo ""

# =========================================
# Test 4: check-claude-flow-memory.sh
# =========================================
echo "Testing: check-claude-flow-memory.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/check-claude-flow-memory.sh"

# Test 4.1: ADVISORY - Stop with edits but no memory_store
EDIT_NO_MEM="$TEST_DIR/edit_no_memory.jsonl"
create_edit_transcript_no_memory "$EDIT_NO_MEM"
test_hook \
    "4.1: Advisory Stop with edits, no memory_store" \
    "$HOOK" \
    "$(create_tool_input 'Stop' '{}' "$EDIT_NO_MEM")" \
    "advisory"

# Test 4.2: ALLOW - Stop with edits AND memory_store
EDIT_WITH_MEM="$TEST_DIR/edit_with_memory.jsonl"
create_edit_transcript_with_memory "$EDIT_WITH_MEM"
test_hook \
    "4.2: Allow Stop with memory_store called" \
    "$HOOK" \
    "$(create_tool_input 'Stop' '{}' "$EDIT_WITH_MEM")" \
    "allow"

# Test 4.3: ALLOW - Stop with no edits
NO_EDIT_TRANSCRIPT="$TEST_DIR/no_edits.jsonl"
echo '{"tool_name":"Read","tool_input":{"file_path":"README.md"}}' > "$NO_EDIT_TRANSCRIPT"
test_hook \
    "4.3: Allow Stop with no source edits" \
    "$HOOK" \
    "$(create_tool_input 'Stop' '{}' "$NO_EDIT_TRANSCRIPT")" \
    "allow"

echo ""

# =========================================
# Test 5: protect-hooks.sh
# =========================================
echo "Testing: protect-hooks.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/protect-hooks.sh"

# Test 5.1: ASK - Write to .claude/hooks/test.sh
test_hook \
    "5.1: Ask Write to .claude/hooks/test.sh" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":".claude/hooks/test.sh","content":"#!/bin/bash"}')" \
    "ask"

# Test 5.2: ASK - Edit to .claude/hooks/enforce-delegation.sh
test_hook \
    "5.2: Ask Edit to hook file" \
    "$HOOK" \
    "$(create_tool_input 'Edit' '{"file_path":".claude/hooks/enforce-delegation.sh","old_string":"x","new_string":"y"}')" \
    "ask"

# Test 5.3: ASK - Bash sed on hook file
test_hook \
    "5.3: Ask Bash sed on hook file" \
    "$HOOK" \
    "$(create_tool_input 'Bash' '{"command":"sed -i \"s/x/y/\" .claude/hooks/test.sh"}')" \
    "ask"

# Test 5.4: ASK - Bash cp to hooks directory
test_hook \
    "5.4: Ask Bash cp to hooks directory" \
    "$HOOK" \
    "$(create_tool_input 'Bash' '{"command":"cp file.sh .claude/hooks/new.sh"}')" \
    "ask"

# Test 5.5: ALLOW - Write to src/main.rs (not a hook)
test_hook \
    "5.5: Allow Write to src/main.rs (not hook)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/main.rs","content":"code"}')" \
    "allow"

# Test 5.6: ALLOW - Bash ls .claude/hooks/ (read only)
test_hook \
    "5.6: Allow Bash ls .claude/hooks/ (read)" \
    "$HOOK" \
    "$(create_tool_input 'Bash' '{"command":"ls .claude/hooks/"}')" \
    "allow"

echo ""

# Helper: Test a blocking hook (uses exit codes, not JSON)
test_blocking_hook() {
    local test_name="$1"
    local hook_script="$2"
    local input_json="$3"
    local expected_result="$4"  # "allow" or "block"

    local exit_code
    local stderr
    stderr=$(echo "$input_json" | bash "$hook_script" 2>&1)
    exit_code=$?

    if [[ "$expected_result" == "allow" ]]; then
        if [[ $exit_code -eq 0 ]]; then
            echo -e "${GREEN}✓ PASS${NC}: $test_name (allowed)"
            PASS=$((PASS + 1))
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $test_name"
            echo "  Expected: allow (exit 0), Got: exit $exit_code"
            FAIL=$((FAIL + 1))
            return 1
        fi
    elif [[ "$expected_result" == "block" ]]; then
        if [[ $exit_code -eq 2 ]]; then
            echo -e "${GREEN}✓ PASS${NC}: $test_name (blocked)"
            PASS=$((PASS + 1))
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $test_name"
            echo "  Expected: block (exit 2), Got: exit $exit_code"
            FAIL=$((FAIL + 1))
            return 1
        fi
    fi
}

# Helper: Create TaskCompleted event JSON
create_task_completed_input() {
    local task_subject="$1"
    local cwd="$2"
    cat <<EOF
{
  "task_id": "test-1",
  "task_subject": "$task_subject",
  "cwd": "$cwd",
  "hook_event_name": "TaskCompleted"
}
EOF
}

# Helper: Create TeammateIdle event JSON
create_teammate_idle_input() {
    local cwd="$1"
    cat <<EOF
{
  "teammate_name": "test-agent",
  "cwd": "$cwd",
  "hook_event_name": "TeammateIdle"
}
EOF
}

# =========================================
# Test 6: enforce-task-quality.sh
# =========================================
echo "Testing: enforce-task-quality.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/enforce-task-quality.sh"
NON_RUST_DIR="$TEST_DIR/non-rust-project"
mkdir -p "$NON_RUST_DIR"

# Test 6.1: ALLOW - Task in non-Rust project (no Cargo.toml)
test_blocking_hook \
    "6.1: Allow task in non-Rust project" \
    "$HOOK" \
    "$(create_task_completed_input 'Implement feature' "$NON_RUST_DIR")" \
    "allow"

# Test 6.2: ALLOW - Documentation task (skipped)
test_blocking_hook \
    "6.2: Allow documentation task" \
    "$HOOK" \
    "$(create_task_completed_input 'Update docs' "$PROJECT_ROOT")" \
    "allow"

# Test 6.3: ALLOW - Plan task (skipped)
test_blocking_hook \
    "6.3: Allow plan task" \
    "$HOOK" \
    "$(create_task_completed_input 'Create plan' "$PROJECT_ROOT")" \
    "allow"

# Test 6.4: ALLOW - Research task (skipped)
test_blocking_hook \
    "6.4: Allow research task" \
    "$HOOK" \
    "$(create_task_completed_input 'Research patterns' "$PROJECT_ROOT")" \
    "allow"

echo ""

# =========================================
# Test 7: enforce-idle-quality.sh
# =========================================
echo "Testing: enforce-idle-quality.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/enforce-idle-quality.sh"

# Test 7.1: ALLOW - Idle in non-Rust project
test_blocking_hook \
    "7.1: Allow idle in non-Rust project" \
    "$HOOK" \
    "$(create_teammate_idle_input "$NON_RUST_DIR")" \
    "allow"

# Test 7.2: ALLOW - Idle with no source changes
RUST_NO_CHANGES="$TEST_DIR/rust-no-changes"
mkdir -p "$RUST_NO_CHANGES"
echo '[package]' > "$RUST_NO_CHANGES/Cargo.toml"
(cd "$RUST_NO_CHANGES" && git init -q && git config user.email "test@test.com" && git config user.name "Test" && git add . && git commit -q -m "init")

test_blocking_hook \
    "7.2: Allow idle with no source changes" \
    "$HOOK" \
    "$(create_teammate_idle_input "$RUST_NO_CHANGES")" \
    "allow"

echo ""

# =========================================
# Test 8: scan-secrets.sh
# =========================================
echo "Testing: scan-secrets.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/scan-secrets.sh"

# Test 8.1: ADVISORY - AWS key detected
test_hook \
    "8.1: Advisory on AWS key detection" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/config.rs","content":"let key = \"AKIAIOSFODNN7EXAMPLE1\";"}')" \
    "advisory"

# Test 8.2: ADVISORY - Password assignment detected
test_hook \
    "8.2: Advisory on password assignment" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/config.rs","content":"password = \"super_secret_password_123\""}')" \
    "advisory"

# Test 8.3: ADVISORY - Private key header detected
test_hook \
    "8.3: Advisory on private key header" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/config.rs","content":"-----BEGIN RSA PRIVATE KEY-----"}')" \
    "advisory"

# Test 8.4: ADVISORY - Database connection string detected
test_hook \
    "8.4: Advisory on database connection string" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/db.rs","content":"let url = \"postgres://admin:secret@localhost/db\";"}')" \
    "advisory"

# Test 8.5: ALLOW - Normal code (no secrets)
test_hook \
    "8.5: Allow normal code (no secrets)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/main.rs","content":"fn main() { println!(\"hello\"); }"}')" \
    "allow"

# Test 8.6: ALLOW - Test file with password (skipped)
test_hook \
    "8.6: Allow test file with password (skipped)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"tests/auth_test.rs","content":"password = \"test_password_123\""}')" \
    "allow"

# Test 8.7: ALLOW - Markdown file (skipped)
test_hook \
    "8.7: Allow markdown file (skipped)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"docs/setup.md","content":"password = \"example_pass\""}')" \
    "allow"

# Test 8.8: ADVISORY - Token assignment via Edit tool
test_hook \
    "8.8: Advisory on token assignment via Edit" \
    "$HOOK" \
    "$(create_tool_input 'Edit' '{"file_path":"src/auth.rs","old_string":"old","new_string":"token = \"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9abcdef\""}')" \
    "advisory"

# Test 8.9: ALLOW - Image file (skipped)
test_hook \
    "8.9: Allow image file (skipped)" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"assets/logo.png","content":"binary_data"}')" \
    "allow"

# Test 8.10: ALLOW - Variable named password (no assignment)
test_hook \
    "8.10: Allow bare password variable name" \
    "$HOOK" \
    "$(create_tool_input 'Write' '{"file_path":"src/auth.rs","content":"fn validate_password(input: &str) -> bool { true }"}')" \
    "allow"

echo ""

# =========================================
# Test 9: enforce-ticket.sh (UserPromptSubmit)
# =========================================
echo "Testing: enforce-ticket.sh"
echo "------------------------------------------"

HOOK="$HOOKS_DIR/enforce-ticket.sh"

# Helper: Create UserPromptSubmit event JSON
create_user_prompt_input() {
    local cwd="$1"
    cat <<EOF
{
  "prompt": "test prompt",
  "cwd": "$cwd",
  "hook_event_name": "UserPromptSubmit"
}
EOF
}

# Helper: Test UserPromptSubmit hook (uses decision/reason JSON, not exit codes)
test_prompt_hook() {
    local test_name="$1"
    local hook_script="$2"
    local input_json="$3"
    local expected_result="$4"  # "allow" or "block"

    local output
    output=$(echo "$input_json" | bash "$hook_script" 2>/dev/null || true)

    if [[ "$expected_result" == "allow" ]]; then
        # Allow = no output or no "decision":"block"
        if [[ -z "$output" ]] || ! echo "$output" | jq -e '.decision == "block"' &>/dev/null; then
            echo -e "${GREEN}✓ PASS${NC}: $test_name (allowed)"
            PASS=$((PASS + 1))
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $test_name"
            echo "  Expected: allow, Got: $output"
            FAIL=$((FAIL + 1))
            return 1
        fi
    elif [[ "$expected_result" == "block" ]]; then
        if echo "$output" | jq -e '.decision == "block"' &>/dev/null; then
            local reason
            reason=$(echo "$output" | jq -r '.reason // empty')
            echo -e "${GREEN}✓ PASS${NC}: $test_name (blocked: $reason)"
            PASS=$((PASS + 1))
            return 0
        else
            echo -e "${RED}✗ FAIL${NC}: $test_name"
            echo "  Expected: block, Got: $output"
            FAIL=$((FAIL + 1))
            return 1
        fi
    fi
}

# Create temp git repos for testing
TICKET_TEST_DIR="$TEST_DIR/ticket-tests"
mkdir -p "$TICKET_TEST_DIR"

# Repo on main branch
MAIN_REPO="$TICKET_TEST_DIR/main-repo"
mkdir -p "$MAIN_REPO"
(cd "$MAIN_REPO" && git init -b main && git config user.email "test@test.com" && git config user.name "Test" && touch x && git add x && git commit -m "init") &>/dev/null

# Repo on master branch
MASTER_REPO="$TICKET_TEST_DIR/master-repo"
mkdir -p "$MASTER_REPO"
(cd "$MASTER_REPO" && git init -b master && git config user.email "test@test.com" && git config user.name "Test" && touch x && git add x && git commit -m "init") &>/dev/null

# Repo on branch without ticket ID
NO_TICKET_REPO="$TICKET_TEST_DIR/no-ticket-repo"
mkdir -p "$NO_TICKET_REPO"
(cd "$NO_TICKET_REPO" && git init -b main && git config user.email "test@test.com" && git config user.name "Test" && touch x && git add x && git commit -m "init" && git checkout -b feature-something) &>/dev/null

# Repo on branch with valid ticket ID
VALID_TICKET_REPO="$TICKET_TEST_DIR/valid-ticket-repo"
mkdir -p "$VALID_TICKET_REPO"
(cd "$VALID_TICKET_REPO" && git init -b main && git config user.email "test@test.com" && git config user.name "Test" && touch x && git add x && git commit -m "init" && git checkout -b feat/eng-18-add-hook) &>/dev/null

# Repo on branch with uppercase ticket ID
UPPER_TICKET_REPO="$TICKET_TEST_DIR/upper-ticket-repo"
mkdir -p "$UPPER_TICKET_REPO"
(cd "$UPPER_TICKET_REPO" && git init -b main && git config user.email "test@test.com" && git config user.name "Test" && touch x && git add x && git commit -m "init" && git checkout -b chore/ENG-42-something) &>/dev/null

# Non-git directory
NON_GIT_DIR="$TICKET_TEST_DIR/non-git"
mkdir -p "$NON_GIT_DIR"

# Test 9.1: BLOCK - on main branch
test_prompt_hook \
    "9.1: Block on main branch" \
    "$HOOK" \
    "$(create_user_prompt_input "$MAIN_REPO")" \
    "block"

# Test 9.2: BLOCK - on master branch
test_prompt_hook \
    "9.2: Block on master branch" \
    "$HOOK" \
    "$(create_user_prompt_input "$MASTER_REPO")" \
    "block"

# Test 9.3: BLOCK - branch without ticket ID
test_prompt_hook \
    "9.3: Block branch without ticket ID" \
    "$HOOK" \
    "$(create_user_prompt_input "$NO_TICKET_REPO")" \
    "block"

# Test 9.4: ALLOW - branch with valid ticket ID
test_prompt_hook \
    "9.4: Allow branch with valid ticket ID (eng-18)" \
    "$HOOK" \
    "$(create_user_prompt_input "$VALID_TICKET_REPO")" \
    "allow"

# Test 9.5: ALLOW - branch with uppercase ticket ID (case-insensitive)
test_prompt_hook \
    "9.5: Allow branch with uppercase ticket ID (ENG-42)" \
    "$HOOK" \
    "$(create_user_prompt_input "$UPPER_TICKET_REPO")" \
    "allow"

# Test 9.6: ALLOW - non-git directory (fail open)
test_prompt_hook \
    "9.6: Allow non-git directory (fail open)" \
    "$HOOK" \
    "$(create_user_prompt_input "$NON_GIT_DIR")" \
    "allow"

echo ""

# =========================================
# Summary
# =========================================
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "${GREEN}PASS: $PASS${NC}"
echo -e "${RED}FAIL: $FAIL${NC}"
echo -e "${YELLOW}SKIP: $SKIP${NC}"
echo ""

if [[ $FAIL -eq 0 ]]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
