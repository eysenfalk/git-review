# Agent Orchestration System Capabilities

This document explains what the orchestration system can do and how it works.

## Overview

The orchestration system coordinates multiple AI agents working together on software projects. It provides:

- **11-agent pipeline** for structured development workflow
- **Parallel execution** via tmux for independent tasks
- **Enforcement chain** ensuring rules are followed
- **Data workflow** across Linear, claude-mem, and context7
- **Git workflow** with branch-per-ticket and review gate
- **Self-improvement** through the optimizer agent
- **Cost optimization** via model routing (Sonnet/Opus/Haiku)

## 11-Agent Pipeline

The system provides 11 specialized agents that work together through a structured workflow.

### 1. requirements-interviewer

**Purpose:** Gather and clarify requirements from the user.

**When to use:** Start of every feature, when requirements are unclear.

**Workflow:**
- Asks clarifying questions
- Explores edge cases
- Produces clear, unambiguous requirements
- Saves requirements to Linear ticket

**Tools:** Read, Glob, Grep, WebSearch, WebFetch, AskUserQuestion, ToolSearch

### 2. explorer

**Purpose:** Research libraries, APIs, prior art, and technical approaches.

**When to use:** Before architectural decisions, when evaluating libraries.

**Workflow:**
- Searches documentation via context7
- Researches approaches via WebSearch
- Compares trade-offs
- Produces actionable findings with code snippets

**Tools:** Read, Glob, Grep, WebSearch, WebFetch, ToolSearch

### 3. architect

**Purpose:** Design module boundaries, data flow, type definitions, integration strategies.

**When to use:** Before implementation, for new modules or significant changes.

**Workflow:**
- Reads existing code
- Designs module structure
- Defines interfaces and data flow
- Documents architectural decisions

**Tools:** Read, Glob, Grep, WebSearch, WebFetch, ToolSearch

### 4. planner

**Purpose:** Write step-by-step implementation plan.

**When to use:** After architecture is defined, before coding.

**Workflow:**
- Reads architecture from Linear
- Breaks work into steps
- Writes detailed plan to local file (`plans/<feature>-plan.md`)
- **ONLY agent that writes local plan files**

**Tools:** Read, Glob, Grep, Write, ToolSearch

### 5. red-teamer

**Purpose:** Critique plans and designs before implementation.

**When to use:** After planner finishes, before coder starts.

**Workflow:**
- Reads the plan
- Finds bugs, edge cases, performance risks
- Identifies over-engineering
- Posts critique to Linear comments
- Can BLOCK implementation if critical issues found

**Tools:** Read, Glob, Grep, Write, WebSearch, ToolSearch

### 6. coder (Sonnet)

**Purpose:** Standard implementation with TDD discipline.

**When to use:** Single-module changes, straightforward features, bug fixes.

**Workflow:**
- Reads plan and red-team critique from Linear
- Looks up library docs via context7
- Implements with TDD (red-green-refactor)
- Runs tests, linter, formatter
- Posts results to Linear

**Model:** Sonnet (cost-effective for standard tasks)

**Tools:** All tools available

### 7. senior-coder (Opus)

**Purpose:** Complex, high-stakes, or cross-cutting implementation.

**When to use:**
- Cross-module refactors
- Performance-critical paths
- Subtle/intermittent bugs
- Architecture-sensitive changes
- Tasks a coder failed at

**Workflow:** Same as coder, but with deeper reasoning

**Model:** Opus (expensive, but necessary for complex tasks)

**Tools:** All tools available

### 8. reviewer

**Purpose:** Code review after implementation.

**When to use:** After coder/senior-coder finishes.

**Workflow:**
- Reads the changes
- Checks against plan and requirements
- Identifies bugs, style issues, security problems
- Posts review to Linear
- Requests changes if needed

**Tools:** All tools available

### 9. documentation

**Purpose:** Update README, doc comments, guides.

**When to use:** After implementation, when user-facing behavior changes.

**Workflow:**
- Reads the implementation
- Verifies accuracy via context7
- Updates README.md and doc comments
- Ensures docs stay in sync with code

**Tools:** Read, Glob, Grep, Write, Edit, Bash, ToolSearch

### 10. explainer

**Purpose:** Explain code at different expertise levels.

**When to use:** When reviewers need to understand AI-generated code, onboarding new team members.

**Workflow:**
- Reads code
- Explains at requested level (junior → staff → architect)
- Covers trade-offs, invariants, system-level implications

**Tools:** Read, Glob, Grep, WebSearch, WebFetch, AskUserQuestion, ToolSearch

### 11. optimizer

**Purpose:** Meta-workflow audit and process improvement.

**When to use:** After every major task completion.

**Workflow:**
- Audits agent pipeline usage
- Identifies inefficiencies (rework, duplication, wasted context)
- Analyzes tool usage (MCP servers, context7, claude-mem)
- Identifies missing capabilities
- Saves findings to claude-mem
- Creates Linear tickets for improvements

**Tools:** Read, Glob, Grep, WebSearch, WebFetch, AskUserQuestion, ToolSearch

## Parallel Execution via tmux

Agents run as visible tmux panes, enabling parallel work and visual monitoring.

### How it Works

1. **TeamCreate** — Creates a named team and shared task list
2. **Task** with `team_name` — Spawns agent as tmux pane
3. Agents appear as split panes in your tmux session
4. Work happens in parallel for independent tasks
5. **SendMessage** — Agents communicate with each other
6. **TeamDelete** — Clean up after work completes

### Example: Parallel Implementation

```
TeamCreate(team_name="feature-auth", description="User authentication")

# Spawn two agents for parallel work
Task(subagent_type="coder", team_name="feature-auth", name="backend-dev",
     prompt="Implement auth API endpoints")

Task(subagent_type="coder", team_name="feature-auth", name="frontend-dev",
     prompt="Implement auth UI components")
```

Both agents work simultaneously, visible in tmux panes.

### When to Parallelize

**Good candidates:**
- Backend + frontend for the same feature
- Multiple independent modules
- Tests + documentation
- Research + architecture (if researching multiple approaches)

**Bad candidates:**
- Sequential dependencies (Task B needs Task A's output)
- Shared files (risk of edit conflicts)

## Enforcement Chain

The system enforces rules through multiple layers.

### Layer 1: CLAUDE.md

Project-level instructions that override defaults:
- File organization
- Build commands
- TDD enforcement
- Error handling patterns
- Git workflow
- Agent routing

### Layer 2: Hooks

Bash scripts that intercept tool calls:

**enforce-review-gate.sh** — Blocks PR creation and merges unless all hunks reviewed via git-review

**enforce-delegation.sh** — Reminds orchestrator to delegate implementation to agents

**enforce-plan-files.sh** — Restricts plan/spec/requirements files to planner agent

**check-unwrap.sh** — Warns against `.unwrap()` in Rust library code (language-specific)

### Layer 3: Agent Specs

Agent-specific instructions:

- Model selection (Sonnet vs Opus vs Haiku)
- Available tools
- MCP server requirements
- Data workflow enforcement
- TDD discipline

### Layer 4: Optimizer

Post-execution audit:

- Checks if rules were followed
- Identifies violations and inefficiencies
- Creates tickets for enforcement improvements
- Saves patterns to claude-mem for future learning

## Data Workflow

The system uses three data stores with clear responsibilities.

### Linear (Source of Truth)

**Purpose:** Requirements, status, acceptance criteria.

**What goes here:**
- Feature requirements
- Ticket status updates
- Architecture decisions (as comments)
- Red-team critiques (as comments)
- Implementation results (as comments)
- Bug reports

**Workflow:**
- requirements-interviewer posts requirements to ticket description
- architect posts design to ticket comments
- red-teamer posts critique to ticket comments
- coder updates status and posts results to ticket comments

### claude-mem (Cross-Session Memory)

**Purpose:** Persistent memory across sessions.

**What goes here:**
- Workflow patterns and conventions
- Implementation decisions
- Non-obvious gotchas
- Optimization findings
- Debugging insights

**Workflow:**
- Search at session start for relevant context
- Save learnings after tasks complete
- Use 3-layer workflow: search → timeline → get_observations

### context7 (Library Documentation)

**Purpose:** Library and API documentation lookup.

**What goes here:** Nothing (read-only).

**Workflow:**
- coder looks up library docs before using unfamiliar APIs
- documentation verifies API references are accurate
- explorer researches library capabilities

### Local Plan Files

**Purpose:** Code-level implementation steps (too granular for Linear).

**What goes here:**
- Step-by-step implementation plan
- File-by-file changes
- Test cases to write

**Who writes:** ONLY the planner agent.

**Location:** `plans/<feature>-plan.md`

## Git Workflow

The system enforces a structured git workflow.

### Branch-Per-Ticket

**Rule:** One branch per Linear ticket, no stacking.

**Format:** `<type>/<ticket-id>-<short-description>`

**Types:** `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

**Examples:**
- `feat/eng-4-line-level-review`
- `fix/eng-6-parser-infinite-loop`

**Enforcement:** Branch name validated by hook before push.

### Review Gate

**Rule:** All hunks must be reviewed via git-review before PR creation or merge.

**Workflow:**
1. Agent finishes work, runs tests, pushes branch
2. User runs `git-review main..<branch>` to review hunks
3. User marks all hunks as reviewed in TUI
4. `git-review gate check` passes
5. PR can be created and merged

**Enforcement:** `enforce-review-gate.sh` hook blocks PRs/merges if gate fails.

### Main Branch Protection

**Rule:** Never commit or push directly to main.

**Enforcement:** Hook blocks direct commits to main, all work goes through feature branches.

### Merge Policy

**Rule:** Merge to main via PR after tests pass AND user review completes.

**Workflow:**
1. Agent pushes feature branch
2. User reviews via git-review
3. User creates PR (agent does NOT create PR)
4. CI runs tests
5. User merges (agent does NOT merge)

**Why:** User must approve changes before they hit main.

## Self-Improvement via Optimizer

The optimizer agent continuously improves the workflow.

### When Optimizer Runs

**Required:** After every major task completion.

**Why:** Catch inefficiencies early, before they become patterns.

### What Optimizer Audits

- Agent pipeline usage (all agents used? redundant agents?)
- Workflow efficiency (rework, duplication, wasted context)
- Git workflow (branches used correctly? merges timely?)
- Tool usage (MCP servers used? context7 before APIs?)
- Missing capabilities (what needs automation? missing agents? missing hooks?)

### Optimizer Outputs

1. **Findings** — Saved to claude-mem for cross-session learning
2. **Recommendations** — Priority-ordered, specific, measurable
3. **Tickets** — Linear issues for actionable improvements

### Example Optimization Cycle

**Session 1:**
- coder guesses at library API, introduces bug
- reviewer catches bug
- optimizer notes: "coder didn't use context7 before unfamiliar API"
- optimizer saves to claude-mem: "Always verify API signatures via context7"

**Session 2:**
- New coder starts task
- Searches claude-mem at session start
- Finds: "Always verify API signatures via context7"
- Looks up docs before coding → no bug introduced

## Cost Optimization

The system routes tasks to appropriate models to minimize costs.

### Model Tiers

**Haiku (cheapest):** Lightweight, fast, good for simple tasks
- Use for: Quick tests, file reads, simple validations

**Sonnet (balanced):** Standard implementation, good reasoning
- Use for: Most implementation work, code review, documentation

**Opus (expensive):** Deep reasoning, complex tasks
- Use for: Cross-module refactors, architecture-sensitive code, subtle bugs

### Agent Model Assignments

- **requirements-interviewer:** Sonnet
- **explorer:** Sonnet
- **architect:** Sonnet
- **planner:** Sonnet
- **red-teamer:** Sonnet
- **coder:** Sonnet (standard work)
- **senior-coder:** Opus (complex work)
- **reviewer:** Sonnet
- **documentation:** Sonnet
- **explainer:** Sonnet
- **optimizer:** Sonnet

### When to Use Haiku

Spawn temporary agents for:
- Running a single test
- Checking file existence
- Reading a specific file
- Simple validations

Example:
```
Task(subagent_type="general-purpose", model="haiku",
     prompt="Run pytest and report results")
```

### Cost Savings

**git-review project:**
- 3 Sonnet agents (coder) + 1 Opus orchestrator
- ~2000 lines of code, 26 tests
- Total cost: ~$15 (vs ~$50 if all Opus)

## Real Example: git-review

The git-review project used this orchestration system to build a complete Rust TUI tool.

### Project Scope

- **Input:** Linear ticket ENG-2 with high-level requirements
- **Output:** 2000 lines of Rust, 26 tests (19 unit + 7 integration), clippy clean, fmt clean
- **Duration:** ~8 hours of agent work (orchestrator + 3 coders)

### Agent Team

- **Orchestrator (Opus):** Coordination and task assignment
- **coder-1 (Sonnet):** Parser module implementation
- **coder-2 (Sonnet):** State module implementation
- **coder-3 (Sonnet):** TUI module implementation

### Workflow

1. **Requirements** — Already in Linear ticket ENG-2
2. **Architecture** — Orchestrator designed bounded contexts (parser, state, tui, gate, cli)
3. **Planning** — Orchestrator wrote `agent-tasks.md` with 5 tasks
4. **Task Assignment:**
   - Task 1: Setup (orchestrator)
   - Task 2: Parser (coder-1)
   - Tasks 3+4: State + TUI in parallel (coder-2, coder-3)
   - Task 5: Gate + CLI (coder-1)
5. **Implementation** — Agents followed TDD, looked up Rust docs via context7
6. **Verification** — All tests pass, clippy clean, fmt clean
7. **Optimization** — Optimizer ran, saved learnings to claude-mem

### Key Learnings

- **Sequential for setup, parallel for modules:** Task 2 (parser) ran first to establish patterns, then tasks 3+4 ran in parallel
- **Clear file ownership:** No two agents touched the same file → no conflicts
- **TaskCreate for coordination:** Agents used shared task list to know what to work on
- **agent-tasks.md for durable specs:** Survived context compaction, agents could always refer back
- **Linear for status:** Orchestrator updated ticket status, not local files
- **Sonnet sufficient for implementation:** Opus not needed for coders, only orchestrator

### Results

- **Tests:** 26 tests, 100% pass rate
- **Quality:** Clippy clean, fmt clean, no unwraps in library code
- **Cost:** ~$15 (vs ~$50 if all Opus)
- **Time:** ~8 hours (vs ~2 days manual coding)

## What the System Cannot Do

**The system does NOT:**

- Replace user judgment (user approves all PRs and merges)
- Write perfect code on first try (red-teamer catches issues, reviewer validates)
- Work without supervision (user guides requirements, reviews changes)
- Handle all tasks (some require human creativity or domain expertise)
- Run indefinitely (user decides when to stop and review)

**The system DOES:**

- Automate repetitive implementation work
- Enforce consistency through hooks and specs
- Coordinate multiple agents working in parallel
- Learn and improve over time via optimizer
- Free the user to focus on high-level decisions

## Next Steps

- Read `setup-guide.md` to deploy the system
- Create your first Linear project
- Spawn your first agent team
- Run the optimizer after major tasks
- Iterate and improve your workflow
