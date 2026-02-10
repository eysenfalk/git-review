# Agent Orchestration Setup Guide

This guide walks you through deploying the AI orchestration system in a new project.

## Prerequisites

### Required Tools

- **Claude Code** — Anthropic's official CLI with agent teams enabled
- **tmux** — Terminal multiplexer for spawning visible agent panes
- **bwrap** — Bubblewrap sandbox for isolating agent execution
- **Linear** — Project management and ticket tracking

### MCP Servers

Install and configure these MCP servers:

- **context7** — Library documentation lookup
- **claude-mem** — Persistent cross-session memory
- **Linear** — Ticket tracking and status updates

Verify MCP servers are available:
```bash
claude mcp list
```

### Environment Variables

Set this in your shell profile (`~/.zshrc` or `~/.bashrc`):

```bash
export CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1
```

Verify tmux is available inside the sandbox:
```bash
command -v tmux
```

## Step 1: Copy Template Files

Copy the template files from `project-template/` into your project:

```bash
cd your-project/
cp -r /path/to/project-template/.claude .
cp /path/to/project-template/CLAUDE.md .
```

This copies:
- `.claude/agents/` — 11 agent specifications
- `.claude/hooks/` — Enforcement hooks
- `.claude/settings.json` — Claude Code configuration
- `CLAUDE.md` — Project orchestration rules

## Step 2: Customize CLAUDE.md

Edit `CLAUDE.md` to match your project:

### Update Project Info

```markdown
# your-project-name

Brief description of what the project does.
Linear ticket: ENG-XXX (your starting ticket).
```

### Adapt File Organization

Update the file organization section to reflect your project structure:

```markdown
## File Organization

- `/src/module1/` — Description
- `/src/module2/` — Description
- `/tests/` — Test files
```

### Customize Build Commands

Replace Rust-specific commands with your language/framework:

```bash
# Rust example (from git-review)
cargo build
cargo test
cargo clippy

# Node.js example
npm install
npm test
npm run lint

# Python example
pip install -r requirements.txt
pytest
ruff check
```

### Adjust TDD Enforcement

Keep the red-green-refactor workflow, but adapt the tooling section:

- Rust: `thiserror` + `anyhow`
- Node: Jest, Mocha, etc.
- Python: pytest, unittest

### Language-Specific Hooks

Remove or adapt hooks that are language-specific:

- `check-unwrap.sh` — Rust-specific, checks for `.unwrap()` calls
- Adapt or remove based on your language

Create new hooks for your language's anti-patterns:
- JavaScript: `console.log` in production code
- Python: bare `except:` clauses
- Go: ignored error returns

### Update MCP Server Instructions

Keep the context7, claude-mem, and Linear sections — these are universal.

## Step 3: Configure Hooks

Hooks enforce rules by intercepting tool calls. Edit `.claude/settings.json`:

```json
{
  "hooks": {
    "preToolUse": [
      {
        "command": ".claude/hooks/enforce-review-gate.sh"
      },
      {
        "command": ".claude/hooks/enforce-delegation.sh"
      },
      {
        "command": ".claude/hooks/enforce-plan-files.sh"
      }
    ]
  }
}
```

### Review Gate Hook

Blocks PR creation and merges unless all hunks are reviewed via `git-review`.

**If not using git-review:** Remove this hook or replace with your own gate.

### Delegation Hook

Reminds the orchestrator to delegate implementation work to specialized agents.

**Keep this** — it prevents the orchestrator from writing code directly.

### Plan Files Hook

Restricts plan/spec/requirements files to the planner agent only.

**Keep this** — enforces the data workflow where Linear is the source of truth.

### Language-Specific Hooks

Add hooks for your language's best practices:

```json
{
  "command": ".claude/hooks/check-forbidden-patterns.sh",
  "description": "Block dangerous patterns in production code"
}
```

## Step 4: Set Up Linear Project

### Create Your Project

1. Log in to Linear
2. Create a new project
3. Note the project key (e.g., `ENG`, `PROJ`)

### Configure Linear MCP

Test Linear access from Claude Code:

```bash
# Search for issues
ToolSearch("+linear list issues")

# Get issue details
mcp__plugin_linear_linear__get_issue
```

### Create Initial Tickets

Create tickets following the agent pipeline:

1. **ENG-1**: Requirements gathering and clarification
2. **ENG-2**: Research and exploration
3. **ENG-3**: Architecture design
4. **ENG-4**: Implementation
5. **ENG-5**: Documentation

## Step 5: Configure bwrap Sandbox

The sandbox MUST be configured correctly for tmux-based agent spawning.

### Required bwrap Flags

In your Claude Code sandbox configuration (usually `~/.config/claude/sandbox.conf` or similar):

```bash
# Share /tmp between host and sandbox (critical for tmux sockets)
--bind /tmp /tmp

# Do NOT unshare the process namespace (breaks tmux)
# Remove or comment out: --unshare-pid

# Pass through tmux environment variables
--setenv TMUX "$TMUX"
--setenv TMUX_PANE "$TMUX_PANE"
```

### Match tmux Binaries

The tmux binary path must be consistent between host and sandbox.

Check host tmux:
```bash
which tmux
```

If using Homebrew (`/home/linuxbrew/.linuxbrew/bin/tmux`), ensure the sandbox mounts Homebrew:

```bash
--ro-bind /home/linuxbrew/.linuxbrew /home/linuxbrew/.linuxbrew
```

**Common mistake:** Host uses `/usr/bin/tmux`, sandbox uses `/home/linuxbrew/.linuxbrew/bin/tmux` → version mismatch → "server exited unexpectedly"

### Clean Stale Sockets

If you see "server exited unexpectedly":

```bash
rm -f /tmp/tmux-1000/default
```

Then restart your tmux session.

### Verify Sandbox Setup

Test tmux inside the sandbox:

```bash
# Inside a Claude Code session (which runs in sandbox)
echo $TMUX
tmux display-message -p '#{session_name}'
```

If both commands succeed, your sandbox is configured correctly.

## Step 6: First Session Walkthrough

### Start Your First Agent Team

1. Start Claude Code in a tmux session
2. Create your first team:

```
TeamCreate(team_name="my-project", description="Initial feature implementation")
```

3. Spawn your first agent:

```
Task(
  subagent_type="requirements-interviewer",
  team_name="my-project",
  name="requirements-gatherer",
  prompt="Gather requirements for ENG-1: user authentication system"
)
```

The agent will appear as a new tmux pane.

### Follow the Pipeline

Use the 11-agent pipeline in order:

1. **requirements-interviewer** — Ask clarifying questions
2. **explorer** — Research libraries and approaches
3. **architect** — Design module boundaries
4. **planner** — Write implementation plan (local file)
5. **red-teamer** — Critique the plan
6. **coder** (or **senior-coder**) — Implement with TDD
7. **reviewer** — Code review
8. **documentation** — Update README, doc comments
9. **explainer** — Explain the code (optional)
10. **optimizer** — Audit workflow (run after major tasks)

### Coordinate via Tasks

Create tasks for agents:

```
TaskCreate(
  subject="Implement authentication module",
  description="Build auth module according to plan in plans/auth-plan.md",
  activeForm="Implementing authentication module"
)
```

Assign tasks to agents:

```
TaskUpdate(taskId="1", owner="coder-1", status="in_progress")
```

### Graceful Shutdown

When work is complete:

```
SendMessage(
  type="shutdown_request",
  recipient="coder-1",
  content="Task complete, shutting down"
)
```

After all agents shut down:

```
TeamDelete()
```

## Troubleshooting

### tmux: "server exited unexpectedly"

**Cause:** tmux version mismatch between host and sandbox, or stale socket.

**Fix:**
1. Check tmux versions: `tmux -V` on host and in sandbox
2. Clean stale sockets: `rm -f /tmp/tmux-1000/default`
3. Ensure bwrap mounts match: host uses Homebrew → sandbox must mount Homebrew

### Agents spawn but aren't visible

**Cause:** `TeamCreate` was not called, or `team_name` was omitted from `Task`.

**Fix:** Always call `TeamCreate` first, then pass `team_name` to every `Task` call.

### SSH not available in sandbox

**Cause:** SSH agent socket not passed through bwrap.

**Fix:** Add to bwrap config:
```bash
--bind $SSH_AUTH_SOCK $SSH_AUTH_SOCK
--setenv SSH_AUTH_SOCK "$SSH_AUTH_SOCK"
```

### gh auth fails in sandbox

**Cause:** GitHub CLI config not mounted.

**Fix:** Add to bwrap config:
```bash
--bind ~/.config/gh ~/.config/gh
```

### Hook blocks legitimate file creation

**Cause:** Hook pattern matching too broad (e.g., `enforce-plan-files.sh` blocking agent spec files).

**Fix:** Edit the hook to exclude directories like `.claude/` and `project-template/`:

```bash
# Exclude template and config directories
if [[ "$FILE_PATH" =~ ^\.claude/ || "$FILE_PATH" =~ ^project-template/ ]]; then
  exit 0
fi
```

### Heredoc false positives

**Cause:** Hooks reading heredoc content instead of just the command.

**Fix:** In hooks, extract only the first line of the command:

```bash
FIRST_LINE=$(echo "$COMMAND" | head -1)
```

## Next Steps

- Read `capabilities.md` to understand what the system can do
- Create your first Linear ticket
- Spawn your first agent team
- Follow the 11-agent pipeline
- Run the optimizer agent after major tasks to improve your workflow

## References

- git-review: Reference implementation using this orchestration system
- `.claude/agents/`: Agent specifications with detailed instructions
- `.claude/hooks/`: Hook implementations with inline comments
- `MEMORY.md`: Persistent memory showing how the git-review project used this system
