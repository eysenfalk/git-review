# Claude-Flow + Native Teams: Integration Analysis

## Background

We have two systems running simultaneously:

1. **Our custom workflow** — 11 agent specs, 8 enforcement hooks, TeamCreate/Task for tmux-visible agents, Linear for tracking, claude-mem for memory
2. **Claude-Flow V3** — An MCP server + CLI tool by ruvnet that adds 200+ tools for orchestration, memory, learning, and security on top of Claude Code

This document explains what each system does, where they overlap, and how to combine them.

---

## How Claude-Flow Works

Claude-Flow is **not** a replacement for Claude Code. It's an intelligence layer that sits on top:

```
┌─────────────────────────────────────────┐
│  Claude-Flow (MCP Server)               │
│  - Memory & learning                    │
│  - Model routing                        │
│  - Hook intelligence                    │
│  - Background workers                   │
├─────────────────────────────────────────┤
│  Claude Code (Native)                   │
│  - TeamCreate / Task / SendMessage      │
│  - tmux pane visualization              │
│  - TaskCreate / TaskUpdate              │
│  - Edit / Write / Bash tools            │
├─────────────────────────────────────────┤
│  Our Custom Layer                       │
│  - 11 agent specs (.claude/agents/)     │
│  - 8 enforcement hooks (.claude/hooks/) │
│  - Linear integration                   │
│  - claude-mem cross-session memory      │
│  - Review gate (git-review)             │
└─────────────────────────────────────────┘
```

Claude-Flow never executes work directly — it always delegates to Claude Code's tools. It adds intelligence, memory, and automation around the edges.

---

## Feature-by-Feature Comparison

### 1. Agent Spawning

**Our system:**
- `TeamCreate` creates a named team
- `Task` with `team_name` spawns agents as visible tmux panes
- Agents communicate via `SendMessage`
- You can watch agents work in real-time

**Claude-Flow:**
- `swarm_init` creates a swarm with topology (mesh, hierarchical, ring, etc.)
- `agent_spawn` creates agents within the swarm
- Supports consensus algorithms (Raft, Byzantine FT, CRDT)
- No tmux visualization — agents are invisible

**Why we should keep ours:** You explicitly want visible tmux panes. Claude-Flow's swarm system is more sophisticated (topologies, consensus) but invisible. Our system gives you direct observation and control.

**What Claude-Flow adds:** Topology optimization and consensus mechanisms for large-scale coordination. Not needed for 3-5 agent teams, but could matter at 10+ agents.

**Recommendation:** Keep TeamCreate as primary. Ignore Claude-Flow's swarm system unless we scale beyond 10 concurrent agents.

---

### 2. Task Management

**Our system:**
- `TaskCreate` / `TaskUpdate` / `TaskList` / `TaskGet`
- Three states: pending → in_progress → completed
- Dependency tracking with blocks/blockedBy
- Stored in `~/.claude/tasks/{team-name}/`
- Session-scoped (lost when team is deleted — we hit this bug)

**Claude-Flow:**
- `task_create` / `task_list` / `task_status` / `task_cancel`
- `task_orchestrate` for automated multi-agent task assignment
- Cross-session persistence
- Agent assignment and retry

**Why we should keep ours:** Tight integration with tmux teams and SendMessage. Simple, works well for our 3-5 agent workflows. You can see task status in the conversation.

**What Claude-Flow adds:** Cross-session persistence (tasks survive TeamDelete — fixes the bug we hit). Automated orchestration. Retry failed tasks.

**Recommendation:** Keep ours as primary. Consider using Claude-Flow's task persistence as a backup to survive crashes/TeamDelete. This is a real gap in our system that Claude-Flow solves.

---

### 3. Memory

**claude-mem (our current):**
- Manual `save_memory` calls
- Search → timeline → get_observations (3-layer workflow)
- Human-readable observations
- Cross-session persistence
- Good for: decisions, preferences, debugging insights

**Claude-Flow memory:**
- `memory_store` / `memory_search` / `memory_retrieve`
- HNSW vector indexing (150x-12,500x faster search)
- Automatic pattern learning from task outcomes
- Shared namespaces for agent coordination
- Local embeddings (no API calls, 75x faster than remote)
- Good for: agent coordination, learned patterns, semantic search

**Why keep claude-mem:** It's our established workflow. Human-readable. Linear supplements it as source of truth. All agent specs reference it.

**What Claude-Flow adds:** Semantic vector search (find related memories even with different wording). Automatic learning (agents share what worked). Agent coordination memory (shared state between spawned agents).

**Recommendation:** Keep both. They serve different purposes:
- **claude-mem** = human decisions and preferences (you write it)
- **Claude-Flow memory** = agent coordination and patterns (automatic)
No conflict — different namespaces, different use cases.

---

### 4. Hooks

**Our hooks (8):**

| Hook | Purpose | Blocking? |
|------|---------|-----------|
| `protect-main.sh` | Block commits/pushes to main | Yes |
| `enforce-branch-naming.sh` | Enforce `<type>/<ticket-id>-<desc>` | Yes |
| `enforce-review-gate.sh` | Block PR create/merge until git-review green | Yes (conditional) |
| `enforce-delegation.sh` | Warn orchestrator when editing src/ | No (reminder) |
| `enforce-plan-files.sh` | Block non-planner writes to plans/ | Yes |
| `check-unwrap.sh` | Warn on unwrap() in library code | No (warning) |
| `enforce-memory.sh` | Remind to save to claude-mem on session end | No (reminder) |
| `workflow-reminder.sh` | Inject workflow rules on every prompt | No (context) |

**Claude-Flow hooks (17):**

| Hook | Purpose |
|------|---------|
| `pre-edit` / `post-edit` | File operation context + learning |
| `pre-command` / `post-command` | Command risk assessment + outcome recording |
| `pre-task` / `post-task` | Agent suggestions + learning |
| `session-start` / `session-end` / `session-restore` | State persistence |
| `route` | Q-Learning router for agent selection |
| `pretrain` | Bootstrap learning from repository |
| `build-agents` | Generate optimized agent configs |
| `model-route` / `model-outcome` / `model-stats` | Smart model selection |
| `intelligence` | Pattern search + attention + trajectory tracking |

**How they interact in settings.json:**
```
PreToolUse on Write/Edit:
  1. npx @claude-flow/cli hooks pre-edit  ← Claude-Flow (continueOnError)
  2. enforce-delegation.sh                ← Ours
  3. enforce-plan-files.sh                ← Ours
  4. check-unwrap.sh                      ← Ours

PreToolUse on Bash:
  1. npx @claude-flow/cli hooks pre-command  ← Claude-Flow (continueOnError)
  2. protect-main.sh                         ← Ours
  3. enforce-branch-naming.sh                ← Ours
  4. enforce-review-gate.sh                  ← Ours
```

**Key insight:** Claude-Flow hooks run FIRST but with `continueOnError: true`, so they can't block our enforcement hooks. They add intelligence (learning from outcomes, suggesting agents) without interfering with our rules.

**Potential issue:** Each `npx @claude-flow/cli` call adds latency (~1-3 seconds). If this becomes noticeable, we can move Claude-Flow hooks after ours or increase timeouts.

**Recommendation:** Keep both. Our hooks enforce rules (blocking). Claude-Flow hooks add learning (non-blocking). They're complementary. Monitor latency — if slow, reorder.

---

### 5. Model Routing

**Our approach:**
- Manual: coder agent = Sonnet, senior-coder = Opus
- Documented in CLAUDE.md and agent specs
- Orchestrator decides which agent to spawn

**Claude-Flow approach:**
- Automatic: `model-route` hook analyzes task complexity
- Routes to Haiku (simple), Sonnet (standard), Opus (complex)
- Tracks outcomes via `model-outcome` to improve routing
- Reports stats via `model-stats`

**Why this matters:** Token costs. Opus is ~15x more expensive than Haiku. If Claude-Flow correctly routes simple tasks to Haiku, significant savings.

**Current config:**
```json
"modelPreferences": {
  "default": "claude-opus-4-6",
  "routing": "claude-3-5-haiku-20241022"
}
```
This means: use Opus by default, use Haiku for the routing decision itself (meta-level).

**Recommendation:** Enable this. Let Claude-Flow suggest models, but keep our manual override (agent specs specify model). Our specs are the authority; Claude-Flow routing is a suggestion layer.

---

### 6. Background Workers

Claude-Flow runs 10 daemon workers on schedules:

| Worker | Schedule | What it does |
|--------|----------|-------------|
| `audit` | Every 1h (critical) | Security and compliance checks |
| `optimize` | Every 30m (high) | Performance optimization |
| `consolidate` | Every 2h (low) | Memory consolidation |
| `document` | Every 1h (normal) | Auto-documentation on API changes |
| `deepdive` | Every 4h (normal) | Deep analysis on complex changes |
| `ultralearn` | Every 1h (normal) | Pattern learning |
| `map` | On demand | Repository mapping |
| `testgaps` | On demand | Find untested code |
| `refactor` | On demand | Suggest refactoring |
| `benchmark` | On demand | Performance benchmarks |

**Why this could be useful:**
- `audit` catches security issues automatically
- `optimize` finds performance improvements
- `testgaps` identifies untested code paths
- `map` keeps a current picture of the codebase

**Why this could be wasteful:**
- 10 workers running means CPU/memory overhead
- Most workers produce output nobody reads
- `document` might conflict with our documentation agent
- `deepdive` and `ultralearn` might not provide value for a small Rust project

**Recommendation:** Keep only 3 workers:
- `audit` (security checks — valuable)
- `optimize` (performance — valuable)
- `map` (codebase understanding — valuable)

Disable the rest to reduce resource usage.

---

### 7. Status Line

Current display:
```
🏗️ DDD Domains [○○○○○] 0/5  ⚡ target: 150x-12500x
🤖 Swarm ◉ [1/15] 👥 0  🪝 6/17  🔴 CVE 0/3  💾 16MB  🧠 60%
🔧 Architecture ADRs ●0/0 │ DDD ● 0% │ Security ●PENDING
📊 AgentDB Vectors ●0 │ Size 0KB │ Tests ●0 │ MCP ●1/1
```

**What each metric means:**
- **DDD Domains 0/5** — Bounded context tracking. We don't do DDD. Always 0.
- **Swarm 1/15** — 1 active agent out of 15 max. Useful.
- **Hooks 6/17** — 6 of 17 Claude-Flow hooks active. Informational.
- **CVE 0/3** — Security vulnerability scan. Not useful for us.
- **Memory 16MB** — AgentDB size. Informational.
- **Brain 60%** — Learning capacity used. Informational.
- **ADRs 0/0** — Architecture Decision Records. We use Linear, not ADRs.
- **Security PENDING** — Security scan status. Not critical for us.
- **AgentDB Vectors 0** — No vectors stored yet (embeddings not trained).

**Recommendation:** The status line is mostly noise for our workflow. Useful metrics: Swarm count, Memory size, Hook count. The rest (DDD, ADRs, CVE, Security) track features we don't use.

Options:
1. **Simplify it** — Customize to show only relevant metrics
2. **Disable it** — Set `statusLine.enabled: false` in settings.json
3. **Replace it** — Write our own showing: branch, Linear ticket, agent count, test status

---

### 8. Features We Don't Use (and shouldn't)

**DDD (Domain-Driven Design) tracking:**
```json
"ddd": { "trackDomains": true, "validateBoundedContexts": true }
```
We have bounded contexts in CLAUDE.md (Parser, State, TUI, Gate, CLI) but don't need automated DDD tracking. This adds overhead for no benefit.

**ADR (Architecture Decision Records) auto-generation:**
```json
"adr": { "autoGenerate": true, "directory": "/docs/adr" }
```
We use Linear for decisions. Auto-generating ADR files would create files nobody reads and clutter the repo.

**Security auto-scanning:**
```json
"security": { "autoScan": true, "scanOnEdit": true, "cveCheck": true }
```
Scans every file edit for CVEs and security issues. Overkill for an internal dev tool. Adds latency to every edit.

**AI Defense (AIMDS):**
Prompt injection detection, PII scanning. Only useful if building user-facing agents handling untrusted input. We're not.

**Recommendation:** Disable all four. They add overhead and noise without value for our project.

---

## Proposed Hybrid Configuration

### What to ENABLE

| Feature | Why |
|---------|-----|
| **Memory (HNSW + hybrid)** | Semantic search, agent coordination, pattern learning |
| **Model routing** | Automatic Haiku/Sonnet/Opus selection for cost savings |
| **Learning (autoTrain)** | Learns from task outcomes to improve over time |
| **3 workers (audit, optimize, map)** | Security, performance, codebase awareness |
| **Pre/post hooks (learning only)** | Records outcomes for pattern improvement |

### What to DISABLE

| Feature | Why |
|---------|-----|
| **DDD tracking** | We don't do DDD |
| **ADR auto-generation** | We use Linear, not ADR files |
| **Security autoScan/CVE** | Overkill, adds latency |
| **AIMDS (AI Defense)** | Not handling untrusted input |
| **7 daemon workers** | Resource waste (keep only audit, optimize, map) |
| **Swarm system** | We use TeamCreate for visibility |
| **Claude-Flow task system** | We use native TaskCreate |
| **GitHub integration** | We use `gh` CLI |

### What to KEEP AS-IS

| Feature | Why |
|---------|-----|
| **Our 8 enforcement hooks** | Rules are enforced, battle-tested |
| **TeamCreate + tmux panes** | Visible agents (user requirement) |
| **Native TaskCreate/Update** | Session coordination works well |
| **claude-mem** | Human-readable cross-session memory |
| **Linear integration** | Source of truth, not changing |
| **11 agent specs** | Our pipeline, well-defined |

---

## Proposed settings.json Changes

```json
{
  "claudeFlow": {
    "version": "3.0.0",
    "enabled": true,
    "modelPreferences": {
      "default": "claude-opus-4-6",
      "routing": "claude-3-5-haiku-20241022"
    },
    "swarm": {
      "topology": "hierarchical-mesh",
      "maxAgents": 15
    },
    "memory": {
      "backend": "hybrid",
      "enableHNSW": true
    },
    "neural": {
      "enabled": true
    },
    "daemon": {
      "autoStart": true,
      "workers": ["audit", "optimize", "map"],    // ← CHANGED: removed 7 unused workers
      "schedules": {
        "audit": { "interval": "1h", "priority": "critical" },
        "optimize": { "interval": "30m", "priority": "high" }
      }
    },
    "learning": {
      "enabled": true,
      "autoTrain": true,
      "patterns": ["coordination", "optimization", "prediction"]
    },
    "adr": {
      "autoGenerate": false,                       // ← CHANGED: disabled
      "directory": "/docs/adr"
    },
    "ddd": {
      "trackDomains": false,                       // ← CHANGED: disabled
      "validateBoundedContexts": false             // ← CHANGED: disabled
    },
    "security": {
      "autoScan": false,                           // ← CHANGED: disabled
      "scanOnEdit": false,                         // ← CHANGED: disabled
      "cveCheck": false                            // ← CHANGED: disabled
    }
  }
}
```

**Final decision:** Keep ALL features enabled. This isn't just internal tooling — we work on large production projects. Stability and integrity over speed.

**Actual changes applied:**
- Hook ordering: our enforcement hooks run FIRST, Claude-Flow hooks LAST (stability priority)
- Attribution: removed Co-Authored-By line from commits
- All 10 workers: kept enabled
- DDD, ADR, Security: kept enabled (production use)
- Memory, learning, model routing: kept enabled

---

## Decisions Made

1. **Status line:** Customize to show relevant metrics (if possible), else disable
2. **Task persistence:** Use Claude-Flow's task system as supplement to native TaskCreate
3. **Model routing:** Agent specs are authoritative, Claude-Flow routing is advisory only
4. **Hook ordering:** Our blocking hooks run first, Claude-Flow non-blocking hooks run second
5. **All workers enabled:** audit, optimize, map, consolidate, testgaps, ultralearn, deepdive, document, refactor, benchmark
6. **All production features enabled:** DDD, ADR, Security scanning
7. **Enforcement:** 100% accuracy required for all workflow rules

---

## Serena MCP Server — Implemented Integration

### What Serena Is

Serena is a free, open-source (MIT, 19k+ stars) coding agent toolkit that provides **semantic code intelligence via LSP** (Language Server Protocol). It gives LLM agents IDE-like abilities: "Go to Definition", "Find All References", symbol-level editing — across 30+ languages.

### Implementation Status: OPERATIONAL

**Completed:**
- All 11 agent specs in `~/.claude/agents/` now include Serena MCP sections
- Project configuration (`project.yml`) configured with rust+bash+toml language support
- Onboarding complete with 4 project memories stored in Serena
- Tool routing guidelines documented in CLAUDE.md

### Why We Need It

| Without Serena | With Serena |
|---|---|
| `Grep "mark_reviewed"` → text matches with false positives | `find_symbol("mark_reviewed")` → exact definition |
| Read entire file (~10k tokens) to edit one function | `replace_symbol_body()` → ~500 tokens |
| No cross-file dependency tracking | `find_referencing_symbols()` → all callers |
| Language-specific parsing | LSP handles 30+ languages |

**Primary benefit:** 50-75% token reduction on code navigation. Agents stay in Opus longer.

### Key Tools

- `find_symbol` — locate functions/classes/variables by name (semantic, not text)
- `find_referencing_symbols` — find all callers/usages across the codebase
- `get_symbols_overview` — top-level symbols in a file
- `replace_symbol_body` — edit function implementation without reading whole file
- `insert_before_symbol` / `insert_after_symbol` — precise insertion
- `write_memory` / `read_memory` — project-specific code context memory

### Installation

```bash
# Per-project (MUST use --context claude-code to avoid tool conflicts)
claude mcp add serena -- \
  uvx --from git+https://github.com/oraios/serena \
  serena start-mcp-server \
  --context claude-code \
  --project "$(pwd)"

# Pre-index for large codebases
uvx --from git+https://github.com/oraios/serena serena project index
```

**Critical:** `--context claude-code` disables duplicate file tools that conflict with Claude Code's native Read/Write/Edit.

### Memory Boundaries (4-layer system)

| Memory System | Scope | What It Stores |
|---|---|---|
| **Linear** | Project management | Requirements, acceptance criteria, ticket status |
| **claude-mem** | Human decisions | Preferences, architectural choices, debugging insights |
| **Claude-Flow memory** | Agent patterns | Coordination patterns, model performance, optimization history |
| **Serena memory** | Code structure | Symbol locations, codebase relationships, project context |

### No Conflicts With Our Stack

- Doesn't touch hooks, agents, or task management
- Doesn't duplicate Linear, claude-mem, or Claude-Flow features
- Only tool overlap is file I/O (disabled via `--context claude-code`)
- Hooks still enforce all rules (Serena edits go through the same tool system)

### Phased Rollout

1. **Phase 1 (validation):** Install with `read_only: true`, test with explorer agent
2. **Phase 2 (limited):** Enable edits for coder agent, monitor token savings
3. **Phase 3 (full):** Enable for senior-coder, reviewer, architect agents
