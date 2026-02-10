# Workflow Documentation

Complete visualization of the git-review workflow including hook execution, agent pipeline, memory architecture, and enforcement boundaries.

---

## Table of Contents

1. [Hook Execution Flow](#hook-execution-flow)
2. [Agent Pipeline](#agent-pipeline)
3. [Memory Architecture](#memory-architecture)
4. [Three-System Integration](#three-system-integration)
5. [Enforcement Matrix](#enforcement-matrix)
6. [What's 100% Enforced vs Not](#whats-100-enforced-vs-not)

---

## Hook Execution Flow

### Write/Edit/MultiEdit Tools

```mermaid
flowchart TD
    Start[Write/Edit/MultiEdit Called] --> H1[protect-hooks.sh]
    H1 --> H1Check{Modifying .claude/hooks/?}
    H1Check -->|Yes| H1Ask[ASK: User approval required]
    H1Check -->|No| H2[enforce-orchestrator-delegation-v2.sh]
    H1Ask --> H2

    H2 --> H2Check{Editing src/tests/Cargo.toml?}
    H2Check -->|No| H3[enforce-plan-files.sh]
    H2Check -->|Yes| H2Transcript{Agent spawned in transcript?}
    H2Transcript -->|No| H2Deny[DENY: Orchestrator must delegate]
    H2Transcript -->|Yes| H3
    H2Transcript -->|Transcript missing| H2Deny

    H3 --> H3Check{Writing to plans/?}
    H3Check -->|No| H4[check-unwrap.sh]
    H3Check -->|Yes| H3Agent{Is planner agent?}
    H3Agent -->|No| H3Deny[DENY: Only planner writes plans]
    H3Agent -->|Yes| H4

    H4 --> H4Check{unwrap() in src/ non-test?}
    H4Check -->|No| H5[Claude-Flow pre-edit]
    H4Check -->|Yes| H4Warn[ADVISORY: unwrap() warning]
    H4Warn --> H5

    H5 --> H5Result{continueOnError: true}
    H5Result --> Allow[Allow: Tool executes]

    H2Deny --> Block[Block: Tool denied]
    H3Deny --> Block
    H1Ask --> AskResult{User response}
    AskResult -->|Approved| H2
    AskResult -->|Denied| Block
```

### Bash Commands

```mermaid
flowchart TD
    Start[Bash Command Called] --> H1[protect-hooks.sh]
    H1 --> H1Check{Writing to .claude/hooks/?}
    H1Check -->|Yes| H1Ask[ASK: User approval required]
    H1Check -->|No| H2[enforce-orchestrator-delegation-v2.sh]
    H1Ask --> H2

    H2 --> H2Check{sed/awk/perl on src/?}
    H2Check -->|No| H3[protect-main.sh]
    H2Check -->|Yes| H2Transcript{Agent spawned in transcript?}
    H2Transcript -->|No| H2Deny[DENY: Orchestrator must delegate]
    H2Transcript -->|Yes| H3
    H2Transcript -->|Transcript missing| H2Deny

    H3 --> H3CheckPush{git push to main?}
    H3CheckPush -->|Yes| H3DenyPush[DENY: No direct push to main]
    H3CheckPush -->|No| H3CheckCommit{git commit on main?}
    H3CheckCommit -->|Yes| H3DenyCommit[DENY: No commits on main]
    H3CheckCommit -->|No| H4[enforce-branch-naming.sh]

    H4 --> H4Check{git commit/push?}
    H4Check -->|No| H5[enforce-review-gate.sh]
    H4Check -->|Yes| H4Branch{On main?}
    H4Branch -->|Yes| H5
    H4Branch -->|No| H4Pattern{Branch matches pattern?}
    H4Pattern -->|No| H4Deny[DENY: Invalid branch name]
    H4Pattern -->|Yes| H5

    H5 --> H5Check{gh pr create/merge OR git merge?}
    H5Check -->|No| H6[Claude-Flow pre-command]
    H5Check -->|Yes| H5Review{git-review passed?}
    H5Review -->|No| H5Deny[DENY: Review required first]
    H5Review -->|Yes| H6

    H6 --> H6Result{continueOnError: true}
    H6Result --> Allow[Allow: Command executes]

    H2Deny --> Block[Block: Command denied]
    H3DenyPush --> Block
    H3DenyCommit --> Block
    H4Deny --> Block
    H5Deny --> Block

    H1Ask --> AskResult{User response}
    AskResult -->|Approved| H2
    AskResult -->|Denied| Block
```

### Task Tool (Agent Spawning)

```mermaid
flowchart TD
    Start[Task Called] --> H1[enforce-visible-agents.sh]
    H1 --> H1Check{Spawning agent?}
    H1Check -->|No subagent_type| Allow[Allow: Not spawning]
    H1Check -->|Has subagent_type| H1Team{Has team_name?}
    H1Team -->|No| H1Deny[DENY: Agents must be visible]
    H1Team -->|Yes| H2[Claude-Flow pre-task]

    H2 --> H2Result{continueOnError: true}
    H2Result --> Allow2[Allow: Agent spawns]

    H1Deny --> Block[Block: Task denied]
```

### Read/Grep/Bash (Source Files)

```mermaid
flowchart TD
    Start[Read/Grep/Bash Called] --> H1[enforce-serena-usage.sh]
    H1 --> H1Check{Reading src/tests source?}
    H1Check -->|Yes| H1Advisory[ADVISORY: Suggest Serena]
    H1Check -->|No| Allow[Allow: Tool executes]
    H1Advisory --> Allow
```

### Stop (Session End)

```mermaid
flowchart TD
    Start[Session Ending] --> H1[enforce-memory.sh]
    H1 --> H1Check{claude-mem used?}
    H1Check -->|No| H1Advisory[ADVISORY: Remind about claude-mem]
    H1Check -->|Yes| H2[check-claude-flow-memory.sh]
    H1Advisory --> H2

    H2 --> H2Check{Code modified?}
    H2Check -->|No| Allow[Allow: Session ends]
    H2Check -->|Yes| H2Saved{Claude-Flow memory saved?}
    H2Saved -->|Yes| Allow
    H2Saved -->|No| H2Advisory[ADVISORY: Suggest saving patterns]
    H2Advisory --> Allow
```

---

## Agent Pipeline

```mermaid
flowchart LR
    User[User Request] --> Interview[requirements-interviewer<br/>Sonnet]
    Interview --> Explore[explorer<br/>Sonnet<br/>Research APIs, libraries]
    Explore --> Architect[architect<br/>Sonnet<br/>Design structure]
    Architect --> Plan[planner<br/>Opus<br/>Write plan files]
    Plan --> RedTeam[red-teamer<br/>Opus<br/>Critique plan]

    RedTeam --> CodeDecision{Task<br/>Complexity?}
    CodeDecision -->|Standard| Coder[coder<br/>Sonnet<br/>Implementation]
    CodeDecision -->|Complex/Cross-cutting| SeniorCoder[senior-coder<br/>Opus<br/>Hard problems]

    Coder --> Review[reviewer<br/>Opus<br/>Code review]
    SeniorCoder --> Review

    Review --> QA[qa<br/>Sonnet<br/>Test execution]
    QA --> Docs[documentation<br/>Sonnet<br/>Update docs]
    Docs --> Explain[explainer<br/>Sonnet<br/>Explain changes]
    Explain --> Optimize[optimizer<br/>Sonnet<br/>Workflow audit]

    Optimize --> Done[Complete]

    style Plan fill:#ffcccc
    style RedTeam fill:#ffcccc
    style SeniorCoder fill:#ffcccc
    style Review fill:#ffcccc
    style Coder fill:#ccffcc
    style Explore fill:#ccffcc
    style Architect fill:#ccffcc
    style Interview fill:#ccffcc
    style QA fill:#ccffcc
    style Docs fill:#ccffcc
    style Explain fill:#ccffcc
    style Optimize fill:#ccffcc
```

**Legend:**
- 🔴 Red (Opus): Quality gates requiring deep reasoning (planner, red-teamer, reviewer, senior-coder)
- 🟢 Green (Sonnet): Implementation and research agents (coder, explorer, architect, qa, documentation, etc.)

---

## Memory Architecture

```mermaid
flowchart TD
    subgraph Sources["Data Sources"]
        User[User Input]
        Code[Code Changes]
        Tickets[Linear Tickets]
        Sessions[Agent Sessions]
    end

    subgraph Memory["4-Layer Memory System"]
        Linear[(Linear<br/>Project Management)]
        ClaudeMem[(claude-mem<br/>Human Decisions)]
        ClaudeFlow[(Claude-Flow<br/>Agent Patterns)]
        Serena[(Serena<br/>Code Structure)]
    end

    subgraph Outputs["Outputs"]
        Requirements[Requirements/<br/>Acceptance Criteria]
        Decisions[Architectural<br/>Decisions]
        Patterns[Learned<br/>Patterns]
        Symbols[Symbol<br/>Relationships]
    end

    User --> Linear
    Tickets --> Linear
    Linear --> Requirements

    User --> ClaudeMem
    Sessions --> ClaudeMem
    ClaudeMem --> Decisions

    Sessions --> ClaudeFlow
    Code --> ClaudeFlow
    ClaudeFlow --> Patterns

    Code --> Serena
    Serena --> Symbols

    Requirements -.->|Read only| ClaudeMem
    Requirements -.->|Never duplicate| ClaudeFlow

    Decisions -.->|Supplements| Linear
    Patterns -.->|Different scope| ClaudeMem
    Symbols -.->|Code-specific| ClaudeFlow

    style Linear fill:#e1f5ff
    style ClaudeMem fill:#fff3e1
    style ClaudeFlow fill:#f3e1ff
    style Serena fill:#e1ffe1
```

**Memory Boundaries (ENFORCED - No Overlap):**

| System | Stores | Examples | Access Pattern |
|--------|--------|----------|----------------|
| **Linear** | Requirements, acceptance criteria, ticket status | "ENG-2: Build TUI tool", "Test: All hunks mark reviewed" | Single source of truth for features |
| **claude-mem** | User preferences, architectural decisions, debugging insights | "User wants visible tmux agents", "Use Sonnet for coders" | 3-layer: search → timeline → get_observations |
| **Claude-Flow** | Agent coordination patterns, model performance, optimization history | "Opus takes 3x longer on parser tasks", "senior-coder better at refactors" | HNSW vector search, automatic learning |
| **Serena** | Symbol locations, codebase relationships, project structure | "mark_reviewed() in src/state/db.rs:45", "7 callers of ReviewDb::new()" | LSP-based semantic queries |

---

## Three-System Integration

```mermaid
flowchart TB
    subgraph Custom["Our Custom System (Blocking Enforcement)"]
        Hooks1[8 Enforcement Hooks<br/>protect-main.sh<br/>enforce-branch-naming.sh<br/>enforce-review-gate.sh<br/>enforce-orchestrator-delegation-v2.sh<br/>enforce-plan-files.sh<br/>protect-hooks.sh<br/>check-unwrap.sh<br/>enforce-visible-agents.sh]

        Teams1[Native Agent Teams<br/>TeamCreate + Task<br/>Visible tmux panes<br/>SendMessage coordination]

        Memory1[claude-mem<br/>Human decisions<br/>Cross-session memory]

        Linear1[Linear Integration<br/>Single source of truth<br/>Requirements + status]
    end

    subgraph ClaudeFlow["Claude-Flow V3 (Learning & Advisory)"]
        Hooks2[17 Advisory Hooks<br/>pre-edit, post-edit<br/>pre-command, post-command<br/>pre-task, post-task<br/>model-route, model-outcome<br/>session-start, session-end<br/>All continueOnError: true]

        Workers[10 Background Workers<br/>audit, optimize, map<br/>consolidate, testgaps<br/>ultralearn, deepdive<br/>document, refactor, benchmark]

        Memory2[HNSW Memory<br/>Vector search<br/>Automatic pattern learning]

        Features[Production Features<br/>DDD domain tracking<br/>ADR generation<br/>Security scanning CVE]
    end

    subgraph Serena["Serena MCP (Semantic Intelligence)"]
        Tools[LSP-Based Tools<br/>find_symbol<br/>find_referencing_symbols<br/>get_symbols_overview<br/>replace_symbol_body<br/>50-75% token savings]

        Memory3[Project Memory<br/>Symbol locations<br/>Code structure<br/>Relationships]
    end

    User[User Action] --> Hooks1
    Hooks1 -->|BLOCKS if rule violated| Stop[Action Denied]
    Hooks1 -->|ALLOWS if rules pass| Hooks2
    Hooks2 -->|Records outcome| Memory2
    Hooks2 -->|Suggests improvements| Advisory[Advisory Context]
    Hooks2 -->|Never blocks| Execute[Action Executes]

    Execute -->|Code change| Workers
    Workers -->|Scans, optimizes, documents| Memory2

    Execute -->|Read/navigate code| Tools
    Tools -->|Symbol queries| Memory3
    Memory3 -->|Fast lookups| Result[Efficient Result]

    Hooks1 -.->|Uses| Memory1
    Hooks1 -.->|Checks| Linear1
    Teams1 -.->|Coordinates via| Linear1

    Memory1 -.->|No overlap| Memory2
    Memory2 -.->|No overlap| Memory3
    Memory3 -.->|No overlap| Linear1

    style Custom fill:#ffe6e6
    style ClaudeFlow fill:#e6f3ff
    style Serena fill:#e6ffe6
    style Stop fill:#ff9999
    style Execute fill:#99ff99
```

**Integration Priorities:**

1. **Blocking First**: Our hooks run first, block if rules violated
2. **Learning Second**: Claude-Flow hooks run after, learn from outcomes (never block)
3. **Intelligence Always**: Serena tools available for token-efficient code navigation
4. **Memory Separation**: No data duplication across 4 memory systems
5. **Fail-Safe**: Claude-Flow hooks have `continueOnError: true` to prevent workflow breakage

---

## Enforcement Matrix

| Rule/Behavior | Enforcement Level | Hook/Mechanism | Bypass Vectors | Notes |
|---------------|-------------------|----------------|----------------|-------|
| **Git Workflow** | | | | |
| No direct commits to main | **DENY** | `protect-main.sh` | None known | Checks `git commit` on main branch, blocks 100% |
| No direct pushes to main | **DENY** | `protect-main.sh` | None known | Checks `git push` with main/master, blocks 100% |
| Branch naming format | **DENY** | `enforce-branch-naming.sh` | User can override branch after commit | Validates `<type>/<ticket-id>-<desc>` on commit/push |
| git-review completion required | **DENY** | `enforce-review-gate.sh` | User can skip git-review tool if not installed | Blocks `gh pr create/merge` until all hunks reviewed |
| | | | | |
| **Agent Coordination** | | | | |
| Agents must be visible (team_name) | **DENY** | `enforce-visible-agents.sh` | None known | Blocks Task tool without team_name parameter |
| Orchestrator delegates to agents | **DENY** | `enforce-orchestrator-delegation-v2.sh` | Manual edits in whitelist dirs (.claude/, docs/, README.md) | Checks transcript for agent spawn before allowing src/ edits |
| Only planner writes plan files | **DENY** | `enforce-plan-files.sh` | Detection relies on agent name in session/env | Blocks non-planner writes to plans/ directory |
| | | | | |
| **Code Quality** | | | | |
| No unwrap() in library code | **ADVISORY** | `check-unwrap.sh` | None - this is a warning only | Warns on unwrap() in src/ non-test files, doesn't block |
| | | | | |
| **Hook Protection** | | | | |
| Hook modifications require approval | **ASK** | `protect-hooks.sh` | User can approve any change | Prompts user before modifying .claude/hooks/ |
| | | | | |
| **Tool Optimization** | | | | |
| Use Serena for source navigation | **ADVISORY** | `enforce-serena-usage.sh` | None - this is a suggestion | Suggests Serena tools for Read/Grep on src/tests files |
| | | | | |
| **Memory Usage** | | | | |
| Use claude-mem for session insights | **ADVISORY** | `enforce-memory.sh` | None - this is a reminder | Reminds on session end if claude-mem not used |
| Save patterns to Claude-Flow | **ADVISORY** | `check-claude-flow-memory.sh` | None - this is a reminder | Reminds on session end if code modified but no patterns saved |
| | | | | |
| **Data Workflow** | | | | |
| Linear is source of truth | **NONE** | Documented in CLAUDE.md | No technical enforcement | Convention only - agents should read/write Linear |
| claude-mem never duplicates Linear | **NONE** | Documented in CLAUDE.md | No technical enforcement | Convention only - manual compliance |
| Plan files only from planner | **DENY** | `enforce-plan-files.sh` | As above | Enforced for plans/ directory |
| Requirements go to Linear not files | **NONE** | Documented in CLAUDE.md | No technical enforcement | Convention only - agents can write local files |
| | | | | |
| **TDD & Testing** | | | | |
| Write tests first (red-green-refactor) | **NONE** | Documented in CLAUDE.md | No technical enforcement | Convention only |
| Run tests before committing | **NONE** | Documented in CLAUDE.md | No technical enforcement | Manual step in Definition of Done |
| All tests must pass | **NONE** | Documented in CLAUDE.md | No technical enforcement | CI should check, but no pre-commit hook |
| | | | | |
| **Security** | | | | |
| No secrets in commits | **NONE** | Documented in CLAUDE.md | No technical enforcement | Manual review required |
| Sanitize file paths | **NONE** | Documented in CLAUDE.md | No technical enforcement | Code review responsibility |
| Validate git refs | **NONE** | Documented in CLAUDE.md | No technical enforcement | Code review responsibility |
| | | | | |
| **Architecture** | | | | |
| No storage of absolute paths | **NONE** | Documented in CLAUDE.md | No technical enforcement | Convention only |
| No UTF-8 assumptions | **NONE** | Documented in CLAUDE.md | No technical enforcement | Convention only |
| Use thiserror in lib, anyhow in bin | **NONE** | Documented in CLAUDE.md | No technical enforcement | Convention only |
| | | | | |
| **Model Routing** | | | | |
| Quality gates use Opus | **NONE** | Agent spec model field | No enforcement - advisory from Claude-Flow | Agent specs are authoritative, but can be overridden |
| Implementation uses Sonnet | **NONE** | Agent spec model field | No enforcement - advisory from Claude-Flow | Agent specs are authoritative, but can be overridden |

---

## What's 100% Enforced vs Not

### 🔴 100% Hard Enforcement (DENY - Blocks Action)

These rules are **technically enforced** and **cannot be bypassed** (or have very limited bypass scenarios):

1. **No direct commits to main branch** (`protect-main.sh`)
   - Detection: Checks `git rev-parse --abbrev-ref HEAD`
   - Bypass: None
   - Enforcement: Blocks `git commit` when on main

2. **No direct pushes to main branch** (`protect-main.sh`)
   - Detection: Regex matches `git push` with main/master in command
   - Bypass: None
   - Enforcement: Blocks `git push` to main/master

3. **Branch naming convention** (`enforce-branch-naming.sh`)
   - Detection: Regex validates `<type>/<ticket-id>-<description>`
   - Bypass: Can rename branch after commit completes
   - Enforcement: Blocks commit/push on invalid branch names

4. **git-review completion before PR** (`enforce-review-gate.sh`)
   - Detection: Runs `git-review status main..<branch>`, checks for "All hunks reviewed"
   - Bypass: If git-review tool not installed, hook fails open
   - Enforcement: Blocks `gh pr create`, `gh pr merge`, `git merge` to main

5. **Agents must be visible in tmux** (`enforce-visible-agents.sh`)
   - Detection: Checks Task tool for `team_name` parameter
   - Bypass: None
   - Enforcement: Blocks agent spawn without team_name

6. **Orchestrator must delegate implementation** (`enforce-orchestrator-delegation-v2.sh`)
   - Detection: Checks transcript for Task/TeamCreate before allowing edits to src/tests/Cargo.toml
   - Bypass: Can edit whitelisted directories (.claude/, docs/, README.md, plans/, project-template/)
   - Bypass: If transcript is unavailable, denies by default (fail-secure)
   - Enforcement: Blocks orchestrator from writing code directly

7. **Only planner writes plan files** (`enforce-plan-files.sh`)
   - Detection: Checks session_id or CLAUDE_AGENT_NAME for "planner"
   - Bypass: Detection is fragile - depends on agent name being present
   - Enforcement: Blocks non-planner writes to plans/ directory

### 🟡 Prompted Enforcement (ASK - Requires Approval)

These rules **ask for user confirmation** before proceeding:

1. **Hook modifications require approval** (`protect-hooks.sh`)
   - Detection: Checks if file_path contains `.claude/hooks/`
   - Bypass: User can always approve
   - Enforcement: Prompts user with reason before allowing Write/Edit/Bash on hooks

### 🔵 Advisory Enforcement (Suggestions Only)

These rules **provide context but don't block**:

1. **No unwrap() in library code** (`check-unwrap.sh`)
   - Detection: Regex matches `.unwrap()` in src/ non-test files
   - Enforcement: Warning message only, doesn't block

2. **Use Serena for source navigation** (`enforce-serena-usage.sh`)
   - Detection: Checks if Read/Grep/Bash targets src/tests source files
   - Enforcement: Suggestion message only, doesn't block

3. **Use claude-mem for session insights** (`enforce-memory.sh`)
   - Detection: Checks transcript for `mcp__plugin_claude-mem` on session end
   - Enforcement: Reminder message only

4. **Save patterns to Claude-Flow memory** (`check-claude-flow-memory.sh`)
   - Detection: Checks transcript for code modifications + `mcp__claude-flow__memory_store` on session end
   - Enforcement: Reminder message only

### ⚪ No Technical Enforcement (Documentation Only)

These rules are **documented in CLAUDE.md but not enforced by hooks**:

**Git Workflow:**
- Merge BEFORE starting dependent work
- One branch per Linear ticket
- Short-lived branches (1-2 days max)
- No stacking features on one branch
- No force push without approval
- Clean up worktrees after merging

**TDD & Testing:**
- Write tests first (red-green-refactor)
- Run `cargo test` after code changes
- Run `cargo check` before committing
- Run `cargo clippy` before opening PRs
- All tests must pass in Definition of Done

**Data Workflow:**
- Linear is single source of truth for requirements
- claude-mem never duplicates Linear content
- Requirements/specs/critiques go to Linear comments, not local files
- Only planner writes local plan files (enforced for plans/ directory only)

**Code Quality:**
- No empty catch blocks
- Propagate errors with `?` operator
- Add context with `.context()`
- Public APIs have doc comments
- Error cases handled, not ignored

**Security:**
- No secrets, credentials, or .env files in commits
- Sanitize all file paths (prevent directory traversal)
- Validate git refs before shell commands
- Never pass unsanitized input to `std::process::Command`
- No hardcoded API keys or credentials

**Architecture:**
- No absolute paths in SQLite database
- No UTF-8 assumptions for diff content
- No blocking TUI event loop with sync I/O
- Use `thiserror` in lib, `anyhow` in bin
- `unwrap()` reserved for tests only (advisory warning exists)

**Agent & Model Routing:**
- Quality gates (planner, red-teamer, reviewer) use Opus
- Implementation agents (coder, explorer, qa) use Sonnet
- Complex tasks route to senior-coder (Opus)
- Agent specs are authoritative for model selection

**Build & Formatting:**
- Code must be formatted (`cargo fmt --check`)
- No clippy warnings (`cargo clippy -- -D warnings`)
- No hardcoded paths or credentials

### 📊 Enforcement Summary

| Category | Count | Percentage |
|----------|-------|------------|
| **Hard Enforcement (DENY)** | 7 rules | ~17% |
| **Prompted Enforcement (ASK)** | 1 rule | ~2% |
| **Advisory Enforcement (warnings)** | 4 rules | ~10% |
| **No Technical Enforcement** | ~30 rules | ~71% |

**Key Takeaway:** The majority of workflow rules (~71%) rely on documentation and manual compliance. Only 19% have any form of technical enforcement (7 hard denials + 1 ask prompt). The remaining 10% are advisory suggestions.

### Known Limitations

1. **Transcript Dependency**: Several hooks (`enforce-orchestrator-delegation-v2.sh`) rely on transcript availability. If transcript is missing or inaccessible, hooks fail-secure (deny by default).

2. **Tool Binary Dependency**: `enforce-review-gate.sh` requires git-review binary to be installed. If missing, the hook fails open (allows PR creation).

3. **Agent Name Detection**: `enforce-plan-files.sh` detects planner agent by looking for "planner" in session_id or environment. This is fragile if agent names change.

4. **Post-Action Bypass**: `enforce-branch-naming.sh` checks branch name on commit, but user can rename branch after commit succeeds.

5. **Whitelist Bypass**: `enforce-orchestrator-delegation-v2.sh` allows orchestrator to edit certain directories (.claude/, docs/, README.md, plans/, project-template/). This is intentional but could be abused.

6. **Manual Steps**: Many critical rules (running tests, checking for secrets, code review quality) have no enforcement and rely entirely on human diligence.

### Honest Assessment

**What works:**
- Git workflow protection (main branch, PR gating) is solid
- Agent visibility enforcement is 100% effective
- Hook protection prevents accidental overwrites
- Advisory hooks provide useful reminders without being annoying

**What's weak:**
- TDD enforcement (no test-first verification)
- Security checks (no secret scanning, no path sanitization checks)
- Code quality (clippy/fmt must be run manually)
- Data workflow (Linear as source of truth is convention only)
- Most architecture rules are documentation only

**What's missing:**
- Pre-commit test running
- Automatic secret detection
- Code coverage requirements
- Dependency vulnerability scanning (unless using Claude-Flow security features)

**Philosophy:**

This system prioritizes **critical safety rails** (no direct main commits, no invisible agents, review gate) while trusting engineers for **quality and conventions** (TDD, clean code, proper testing). The 17% hard enforcement focuses on preventing workflow breakage and protecting shared state. The 71% documentation-only rules require discipline and code review to maintain.
