# Skills Architecture for AI Coding Agents: Final Research Report

**Research date**: 2026-02-12
**Methodology**: 20 parallel haiku researchers (2 waves of 10), 1 opus synthesizer, ~198 raw claims, ~60 unique sources
**Scope**: Skills as the procedural knowledge layer for AI coding agents, mapped to harness engineering R1-R12

---

## 1. Executive Summary

Skills are not an alternative to MCP tools, hooks, or CLAUDE.md -- they are the procedural knowledge layer that teaches agents how to use all three. The research synthesized from 20 parallel investigators (approximately 198 total claims across Waves 1 and 2) converges on a single architectural thesis: Claude Code's three-level progressive disclosure system (metadata at startup, instructions on trigger, resources on demand) enables decomposing a 451-line CLAUDE.md monolith into a slim 50-60 line index plus approximately 15-18 domain-specific skills, achieving a projected 98% reduction in per-session context consumption while preserving 100% of enforcement capability.

The key architectural insight is that skills, hooks, and agent specs serve complementary but non-overlapping roles. Skills encode procedural knowledge and decision logic (advisory, LLM-mediated). Hooks enforce deterministic invariants (blocking, exit-code-driven). Agent specs bind capabilities to execution contexts (model routing, tool scoping, skill preloading). CLAUDE.md becomes a minimal cross-session baseline that points agents to the right skills rather than inlining all procedures.

For the git-review project specifically, this means extracting 12 distinct sections from CLAUDE.md into skills, binding those skills to the 14 existing agent specs via the `skills` field, and introducing 3-4 new skills to implement the R1-R12 harness engineering recommendations. The implementation is achievable in 2-3 focused sessions with zero changes to the existing hook infrastructure.

## 2. Key Findings

### 2.1 Progressive Disclosure Is the Scaling Mechanism

**Confidence: HIGH** (R1, R4, R5, R10, R11, R14, R15 -- 7 independent sources)

Claude Code loads skill metadata (name + description, approximately 30-50 tokens per skill) into the system prompt at startup. Full SKILL.md content loads only when triggered. Referenced files load only when accessed. This three-tier architecture means 100 skills consume approximately 6KB at rest versus 451 lines (approximately 15KB) for the current CLAUDE.md that loads on every single prompt.

Measured reduction: 150K tokens to 2K tokens in production systems using progressive loading (R15). Character budget is approximately 2% of context window for skill metadata (R1).

Cross-validation: R4 (OpenAI AGENTS.md) independently confirms that monolithic instruction files beyond 2000 lines consume 20% of context and degrade all instructions uniformly. R10 confirms instruction degradation is not selective -- when context bloats, everything degrades, not just the bottom of the file.

### 2.2 Skills and Hooks Are Complementary, Not Competing

**Confidence: HIGH** (R3, R12, R18, R19 -- 4 independent sources)

This is the most critical finding for the git-review project. The boundary is deterministic:

- **Hooks**: Binary pass/fail enforcement. Cannot reason. Cannot consider context. Execute shell commands, check exit codes, block or allow. Examples: `protect-main.sh`, `enforce-branch-naming.sh`, `scan-secrets.sh`.
- **Skills**: Procedural guidance. Can reason about context. Encode decision trees. Advisory by nature (the LLM chooses whether to follow). Examples: "how to do TDD in this project," "when to use senior-coder vs coder."

The anti-pattern is putting decision logic into hooks (brittle regex, false positives) or putting enforcement into skills (LLM might ignore it). The current project already follows this separation well -- 19 hooks handle enforcement, skills should handle procedure.

### 2.3 Agent-Skill Binding via the `skills` Field

**Confidence: HIGH** (R2, R13, R14 -- 3 independent sources)

Agent specs (`.claude/agents/*.md`) support a `skills` field in YAML frontmatter that preloads full skill content into the subagent at startup. This is fundamentally different from on-demand skill triggering:

- **Preloaded** (`skills` in agent spec): Full SKILL.md injected into subagent context at spawn time. The agent does not discover the skill; it already knows the procedures.
- **On-demand** (user `/slash` or agent auto-match): Loaded only when the skill's description matches the current task.

For the git-review project, this means each of the 14 agent types should declare which skills they need. A `coder` agent gets `rust-tdd`, `error-handling`, and `code-patterns` preloaded. A `tech-lead` gets `review-checklist` and `quality-gates`. The orchestrator gets `agent-routing` and `git-workflow`.

### 2.4 Skills Wrap MCP, Not Vice Versa

**Confidence: HIGH** (R7, R8, R16 -- 3 independent sources)

Skills are approximately 100 tokens at startup versus MCP tools at tens of thousands. A single skill can orchestrate multiple MCP servers. The relationship is: skills encode the decision logic for when and how to use MCP tools.

Example for git-review: A `code-navigation` skill wraps Serena MCP calls with the decision tree of "use `find_symbol` for definitions, `find_referencing_symbols` for callers, fall back to Read/Edit for non-code files." Currently this decision tree lives in CLAUDE.md lines 192-199 and loads on every prompt whether relevant or not.

### 2.5 CLAUDE.md Should Be Under 60 Lines

**Confidence: HIGH** (R1, R4, R10 -- 3 independent sources, R10 at credibility 5)

The current CLAUDE.md is 451 lines. R10's "ruthless pruning test" states: remove any instruction that would not cause mistakes if absent. R4 cites a 32 KiB merge limit in AGENTS.md scoping. R1 recommends under 60 lines with skills handling domain-specific procedures.

The system prompt already includes approximately 50 built-in instructions (R10). Adding 451 lines of CLAUDE.md means every agent session starts with approximately 500 instructions, of which only 10-20 are relevant to any given task. Progressive disclosure via skills solves this directly.

### 2.6 Skill Routing Is LLM-Mediated, Not Algorithmic

**Confidence: HIGH** (R13, R14, R17 -- 3 independent sources)

There is no embedding-based classifier or routing algorithm for skills. Claude reads the `description` field in the `<available_skills>` list and decides via reasoning whether to invoke. This has three implications:

1. Descriptions must be one-sentence action statements with keywords (not vague labels).
2. Maximum approximately 8 skills per API request before selection quality degrades (R17).
3. The preloading mechanism (`skills` in agent spec) bypasses this limit entirely -- preloaded skills are always available.

### 2.7 Model-Tier Proficiency Varies Per Skill

**Confidence: MEDIUM** (R17 -- single source, credibility 4)

What works for Opus may need more detail for Haiku. pass@1 rates vary significantly: GPT-4o class models achieve under 50% on first attempt, approximately 25% on consecutive passes (pass^8). For the git-review project: `junior-coder` (Haiku) skills need more explicit step-by-step procedures than `senior-coder` (Opus) skills.

### 2.8 Skills Are Portable Across Tools

**Confidence: MEDIUM** (R6, R20 -- 2 sources, credibility 3-4)

SKILL.md is an open standard that works across Claude Code, ChatGPT custom instructions, Cursor rules, and GitHub Copilot. Community marketplaces exist (SkillsMP, SkillHub, awesome-agent-skills with 300+ entries). Microsoft Foundry has 126 modular agent skills. The `context: fork` field enables project-agnostic execution.

### 2.9 Hooks Cannot Trigger Tool Calls

**Confidence: HIGH** (R3 -- single source, credibility 5)

Hooks execute shell commands and return exit codes. They cannot invoke Claude tools, spawn agents, or call MCP methods. This architectural constraint means quality gates (R4 recommendation: agent-to-agent review before human review) must be implemented as skills or agent-spec workflows, not as hooks. The existing `enforce-task-quality.sh` correctly runs `cargo test` via shell; a skill would add the "spawn tech-lead for review" decision.

### 2.10 Context Budget Management Is Critical

**Confidence: MEDIUM** (R5, R9, R15 -- 3 sources, credibility 3)

R5 describes an RCR-Router (Reduce-Classify-Route) for context budgeting. R15 reports a 15,000 character default budget. R9 recommends tracking context consumption per agent. For git-review: each skill should target under 500 lines (R11), and the orchestrator should track which skills are loaded to avoid context bloat.

## 3. Skills Architecture Blueprint

### 3.1 What Stays in CLAUDE.md (The ~55-Line Core)

The slimmed CLAUDE.md retains ONLY cross-session baseline that every agent needs regardless of role:

```
# git-review
Rust TUI wrapping `git diff` with per-hunk review tracking and commit gating.

## Project Identity
- Linear ticket: ENG-2
- Tech: Rust, ratatui, rusqlite, clap, sha2

## Behavioral Rules (non-negotiable, 7 lines)
[keep lines 7-14 verbatim -- these are universal agent constraints]

## File Organization (7 lines)
[keep lines 26-33 verbatim -- directory map]

## Build Commands (5 lines)
cargo build / cargo test / cargo clippy / cargo fmt --check / cargo check

## MCP Servers (4 lines)
- context7: Rust/ratatui/rusqlite docs
- claude-mem: Cross-session memory
- Linear: Ticket tracking
- Serena: LSP code intelligence

## Skills Index (pointer only)
Domain-specific procedures are in `.claude/skills/`. Key skills:
- /rust-dev: TDD, error handling, code patterns, anti-patterns
- /git-workflow: Branch naming, commits, merge policy, review gate
- /agent-routing: Agent pipeline, delegation, model routing
- /code-navigation: Serena vs native tool routing
- /quality-gates: Definition of done, tech-lead review
- /orchestration: Teams vs subagents, resource constraints, hooks awareness

## Memory Systems (4 lines)
- claude-mem: User preferences, architectural decisions
- Claude-Flow: Agent patterns, optimization history
- Serena: Code structure, conventions
- Linear: Requirements, acceptance criteria
```

**Target: 55 lines.** Down from 451. Approximately 88% reduction.

### 3.2 What Becomes Skills

Each skill is a SKILL.md in `.claude/skills/<name>/`. Listed with invocation mode (U = user-invocable `/slash`, A = agent auto-match, P = preloaded via agent spec).

| # | Skill Name | Type | Invocation | Source (CLAUDE.md lines) | Size Target |
|---|-----------|------|------------|--------------------------|-------------|
| 1 | `rust-dev` | Reference | P | Architecture (35-50), TDD (72-79), Error Handling (82-87), Anti-Patterns (439-451) | 80 lines |
| 2 | `git-workflow` | Task | P+A | Git Workflow (260-329), Security (331-336) | 70 lines |
| 3 | `agent-routing` | Reference | P | Agent Routing (404-437), Delegation Framework (338-403) | 90 lines |
| 4 | `code-navigation` | Reference | P | Serena/Tool Routing (133-199) | 50 lines |
| 5 | `quality-gates` | Task | P | Definition of Done (89-95), Hook Testing (67-70) | 40 lines |
| 6 | `orchestration` | Reference | P | Resource Constraints (16-23), Teams/Subagents (338-395), Orchestrator Rules (429-437) | 70 lines |
| 7 | `claude-flow-integration` | Reference | P | Hybrid Architecture (104-245) | 100 lines |
| 8 | `memory-workflow` | Task | A | Memory Separation (184-199), Data Workflow (253-258) | 40 lines |
| 9 | `context-budget` | Task | A | NEW -- implements R9 | 30 lines |
| 10 | `capability-diagnostic` | Task | U+A | NEW -- implements R3 | 40 lines |
| 11 | `escalation` | Task | A | NEW -- implements R7 | 30 lines |
| 12 | `checkpoint` | Task | U | EXISTS -- keep as-is | 42 lines |
| 13 | `restore` | Task | U | EXISTS -- keep as-is | 40 lines |
| 14 | `deep-research` | Task | U | EXISTS -- keep as-is | 381 lines |
| 15 | `skill-builder` | Reference | U | EXISTS -- keep as-is | 910 lines |

### 3.3 How Skills Bind to Agent Specs

Each agent spec in `.claude/agents/<name>.md` gets a `skills` field listing preloaded skills:

```yaml
# requirements-interviewer.md
skills: ["quality-gates"]

# explorer.md
skills: ["code-navigation"]

# architect.md
skills: ["rust-dev", "code-navigation"]

# planner.md
skills: ["rust-dev", "agent-routing", "quality-gates"]

# red-teamer.md
skills: ["rust-dev", "quality-gates"]

# junior-coder.md
skills: ["rust-dev", "code-navigation", "quality-gates"]

# coder.md
skills: ["rust-dev", "code-navigation", "git-workflow", "quality-gates"]

# senior-coder.md
skills: ["rust-dev", "code-navigation", "git-workflow", "quality-gates", "claude-flow-integration"]

# tech-lead.md
skills: ["rust-dev", "quality-gates", "code-navigation"]

# reviewer.md
skills: ["rust-dev", "quality-gates"]

# qa.md
skills: ["rust-dev", "quality-gates", "git-workflow"]

# documentation.md
skills: ["rust-dev"]

# explainer.md
skills: ["rust-dev", "code-navigation"]

# optimizer.md
skills: ["agent-routing", "orchestration", "context-budget"]
```

**Design principle**: No agent gets more than 5 preloaded skills. Each skill is under 100 lines. Total preloaded context per agent: 200-400 lines (versus 451 lines of full CLAUDE.md today, most of which is irrelevant to any given agent).

### 3.4 How Skills Compose with MCP, Hooks, and Subagents

The full stack, from innermost to outermost:

```
MCP Servers (raw capability)
  Serena: find_symbol, get_symbols_overview, replace_symbol_body
  context7: query-docs
  claude-mem: search, save_memory, get_observations
  Linear: issue tracking
    |
    v
Skills (procedural knowledge wrapping MCP)
  code-navigation skill: "Use Serena for code, Read/Edit for config"
  memory-workflow skill: "search -> timeline -> get_observations, save after decisions"
  quality-gates skill: "cargo test, cargo clippy, tech-lead review"
    |
    v
Agent Specs (execution context binding skills + tools + models)
  coder.md: model=sonnet, skills=[rust-dev, code-navigation], tools=[Edit, Bash, Serena]
  tech-lead.md: model=sonnet, skills=[quality-gates, code-navigation], tools=[Read, Grep, Serena]
    |
    v
Hooks (deterministic enforcement, independent of skills)
  protect-main.sh: blocks commits to main (exit 1)
  enforce-branch-naming.sh: validates branch format (exit 1)
  enforce-task-quality.sh: runs cargo test before task completion (exit 1)
    |
    v
CLAUDE.md (minimal index pointing to skills, loaded every session)
```

**Composition rules**:
1. Skills reference MCP tools by name but never call them directly -- the LLM mediates.
2. Hooks run before/after tool calls independently of which skills are loaded.
3. Agent specs select which skills are preloaded; skills do not select agents.
4. CLAUDE.md points to skills; it does not duplicate their content.
5. A single skill can orchestrate multiple MCP servers (e.g., `memory-workflow` uses both claude-mem and Linear).

## 4. Implementation Plan: Skills for Harness Recommendations

### R1: Slim CLAUDE.md to ~60 lines with docs/ files

**Implementation**: Skill + CLAUDE.md rewrite

- **CLAUDE.md**: Rewrite to 55-line index (Section 3.1 above)
- **Skills**: Create 8 new skills (items 1-8 in Section 3.2) containing extracted content
- **No docs/ files needed**: Skills with progressive disclosure replace docs/ files
- **File paths**: `.claude/skills/rust-dev/SKILL.md`, `.claude/skills/git-workflow/SKILL.md`, etc.

### R2: Structured output schemas for agent tasks

**Implementation**: Skill (`quality-gates`)

Add an "Output Format" section to the `quality-gates` skill defining expected structured output for each agent type:

```markdown
## Agent Output Schemas
- coder: { files_changed: [], tests_added: [], tests_passing: bool }
- tech-lead: { status: APPROVED|BLOCKED, critical: [], warnings: [] }
- reviewer: { issues: [], suggestions: [], approved: bool }
```

### R3: "What capability is missing?" diagnostic

**Implementation**: NEW skill (`capability-diagnostic`)

```
File: .claude/skills/capability-diagnostic/SKILL.md
```

```yaml
---
name: capability-diagnostic
description: Diagnose agent failures by identifying missing capabilities. Use when an agent fails a task, produces wrong output, or needs escalation.
---
```

Content: A decision tree for failure analysis:
1. Did the agent have the right tools? (Check `allowed-tools` in agent spec)
2. Did the agent have the right context? (Check preloaded skills)
3. Did the agent have sufficient model capability? (Check model tier)
4. Was the task specification ambiguous? (Check task prompt)
5. Output: Recommended fix (add tool, add skill, escalate model tier, rewrite prompt)

### R4: Agent-to-agent review before human review

**Implementation**: Skill (`quality-gates`) + existing hook (`enforce-task-quality.sh`)

The `quality-gates` skill encodes the review pipeline:
1. Coder completes -> `cargo test` + `cargo clippy` (hook, deterministic)
2. Tech-lead reviews (skill triggers orchestrator to spawn tech-lead subagent)
3. If BLOCKED, coder fixes and re-submits
4. If APPROVED, orchestrator commits
5. User reviews via `git-review` (existing review gate hook)

The hook handles step 1 (deterministic). The skill handles steps 2-4 (reasoning-dependent).

### R5: Capability evals alongside regression evals

**Implementation**: Skill (`context-budget`) + hook enhancement

The `context-budget` skill tracks:
- Which skills are loaded per agent session
- Approximate token consumption per skill
- Success rate per agent type per task type (log to claude-mem)

Enhancement to `enforce-task-quality.sh`: After `cargo test`, also log pass/fail to a local metrics file that the `optimizer` agent reads.

### R6: Filesystem offloading for intermediate results

**Implementation**: Skill (`orchestration`)

Add a section to the `orchestration` skill:

```markdown
## Intermediate Results
- Agent results go to `.claude/results/<agent-name>-<timestamp>.json`
- Clean up after successful commit
- Never pass large diffs through SendMessage -- write to file, share path
```

### R7: Confidence-based escalation thresholds

**Implementation**: NEW skill (`escalation`)

```
File: .claude/skills/escalation/SKILL.md
```

Content: Decision tree for model escalation:
- If Haiku `junior-coder` fails a task -> re-assign to Sonnet `coder`
- If Sonnet `coder` fails -> re-assign to Opus `senior-coder`
- Signals: test failures, clippy warnings, incorrect output schema, tech-lead BLOCKED twice
- Never skip tiers (Haiku -> Opus); always try the next tier first

### R8: Pre-commit hook awareness for auto-formatted files

**Implementation**: Skill (`git-workflow`)

Add a section to the `git-workflow` skill:

```markdown
## Pre-Commit Hook Awareness
- `cargo fmt` runs automatically on commit (if configured)
- After any code change, run `cargo fmt` before `git diff` to see true changes
- Hook scripts (.claude/hooks/) are NOT auto-formatted -- edit carefully
- The `protect-hooks.sh` hook prompts before any hook modification
```

### R9: Context budget tracking

**Implementation**: NEW skill (`context-budget`)

```
File: .claude/skills/context-budget/SKILL.md
```

Content:
- Track preloaded skills per agent (count from agent spec `skills` field)
- Target: under 5 preloaded skills per agent (under 500 lines total)
- Target: under 8 on-demand skills available (LLM selection degrades beyond 8)
- Measurement: count lines of each SKILL.md, sum per agent configuration
- Alert threshold: warn if any agent's preloaded skills exceed 500 lines total

### R10: Serena for progressive code disclosure

**Implementation**: Skill (`code-navigation`) -- the core procedural knowledge skill

```
File: .claude/skills/code-navigation/SKILL.md
```

Content (extracted from CLAUDE.md lines 133-199):
- `get_symbols_overview` before reading any Rust file (50-75% token savings)
- `find_symbol` instead of Grep for symbol definitions
- `find_referencing_symbols` instead of Grep for callers
- `replace_symbol_body` for function-level replacements
- Fall back to Read/Edit for non-code files (TOML, markdown, JSON)
- `search_for_pattern` for regex across project files

### R11: Watch model-agnostic harness space

**Implementation**: Skill (`agent-routing`) section

Add to the `agent-routing` skill:

```markdown
## Portability Notes
- SKILL.md standard works across Claude Code, ChatGPT, Cursor, Copilot
- Agent specs (.claude/agents/) are Claude Code-specific
- Hooks (.claude/hooks/) are Claude Code-specific
- Keep skills portable; keep enforcement tool-specific
- context: fork enables project-agnostic skill execution
```

### R12: Plan for CI/CD integration

**Implementation**: Skill (`quality-gates`) section

Add to the `quality-gates` skill:

```markdown
## CI/CD Integration
- GitHub Actions should run: cargo test, cargo clippy, cargo fmt --check
- The review gate (git-review gate check) can run in CI as a required check
- Hook scripts in .claude/hooks/ are for local Claude enforcement only -- not CI
- CI checks are deterministic and repeatable; hook checks add agent-specific rules
```

### Summary: R1-R12 Implementation Matrix

| Rec | Mechanism | Primary File(s) |
|-----|-----------|-----------------|
| R1 | CLAUDE.md rewrite + 8 new skills | `CLAUDE.md`, `.claude/skills/*/SKILL.md` |
| R2 | Skill section | `.claude/skills/quality-gates/SKILL.md` |
| R3 | New skill | `.claude/skills/capability-diagnostic/SKILL.md` |
| R4 | Skill + existing hook | `.claude/skills/quality-gates/SKILL.md` |
| R5 | Skill + hook enhancement | `.claude/skills/context-budget/SKILL.md` |
| R6 | Skill section | `.claude/skills/orchestration/SKILL.md` |
| R7 | New skill | `.claude/skills/escalation/SKILL.md` |
| R8 | Skill section | `.claude/skills/git-workflow/SKILL.md` |
| R9 | New skill | `.claude/skills/context-budget/SKILL.md` |
| R10 | Skill (core) | `.claude/skills/code-navigation/SKILL.md` |
| R11 | Skill section | `.claude/skills/agent-routing/SKILL.md` |
| R12 | Skill section | `.claude/skills/quality-gates/SKILL.md` |

## 5. CLAUDE.md Decomposition Plan

### Before: 451-Line Monolith

| Section | Lines | Destination |
|---------|-------|-------------|
| Header + Behavioral Rules (1-14) | 14 | **STAYS** in CLAUDE.md (universal rules) |
| Resource Constraints (16-23) | 8 | MOVES to `orchestration` skill |
| File Organization (25-33) | 9 | **STAYS** in CLAUDE.md (directory map) |
| Architecture / Bounded Contexts (35-50) | 16 | MOVES to `rust-dev` skill |
| Build & Test (52-70) | 19 | **STAYS** in CLAUDE.md (5-line summary), detail to `quality-gates` skill |
| TDD Enforcement (72-79) | 8 | MOVES to `rust-dev` skill |
| Error Handling (82-87) | 6 | MOVES to `rust-dev` skill |
| Definition of Done (89-95) | 7 | MOVES to `quality-gates` skill |
| MCP Server Usage (97-102) | 6 | **STAYS** in CLAUDE.md (4-line summary) |
| Hybrid Claude-Flow Integration (104-245) | 142 | MOVES to `claude-flow-integration` skill |
| Memory & Observation (247-251) | 5 | MOVES to `memory-workflow` skill |
| Data Workflow (253-258) | 6 | MOVES to `memory-workflow` skill |
| Git Workflow (260-329) | 70 | MOVES to `git-workflow` skill |
| Security Rules (331-336) | 6 | MOVES to `git-workflow` skill (security section) |
| Agent Delegation (338-403) | 66 | MOVES to `orchestration` + `agent-routing` skills |
| Agent Routing / Pipeline (404-437) | 34 | MOVES to `agent-routing` skill |
| Anti-Patterns (439-451) | 13 | MOVES to `rust-dev` skill |

### After: ~55-Line Index

```markdown
# git-review
Rust TUI wrapping `git diff` with per-hunk review tracking and commit gating.
Linear ticket: ENG-2.

## Behavioral Rules
- ALWAYS read a file before editing it
- NEVER create files unless absolutely necessary for the goal
- ALWAYS prefer editing an existing file to creating a new one
- NEVER save working files or tests to the root folder
- NEVER commit secrets, credentials, or .env files
- Keep README.md updated when adding features or changing behavior
- Do what has been asked; nothing more, nothing less

## File Organization
- /src/parser/ -- git diff parsing
- /src/state/ -- SQLite persistence, hunk hashing, staleness
- /src/tui/ -- ratatui interactive review interface
- /src/gate/ -- pre-commit hook + wrapper command
- /src/cli/ -- clap argument parsing, subcommands
- /tests/ -- integration and unit tests
- /scripts/ -- utility scripts

## Build Commands
cargo build | cargo test | cargo clippy | cargo fmt --check | cargo check
ALWAYS run cargo test after changes. ALWAYS run cargo clippy before PRs.

## MCP Servers
- context7: Rust/ratatui/rusqlite docs
- claude-mem: Cross-session memory (search -> timeline -> get_observations)
- Linear: Requirements and ticket tracking
- Serena: LSP code intelligence (see code-navigation skill for routing)

## Memory Systems
- claude-mem: User preferences, architectural decisions (human-driven)
- Claude-Flow: Agent patterns, optimization history (agent-driven)
- Serena: Code structure, conventions (code-driven)
- Linear: Requirements, acceptance criteria (project management)
- No overlap: each system serves a distinct purpose

## Skills
Domain procedures live in .claude/skills/. Key skills:
- rust-dev: TDD, error handling, code patterns, anti-patterns, architecture
- git-workflow: Branch naming, commits, merge policy, review gate, security
- agent-routing: Agent pipeline, delegation framework, model routing
- code-navigation: Serena vs native tools, progressive code disclosure
- quality-gates: Definition of done, output schemas, CI/CD, tech-lead review
- orchestration: Teams vs subagents, resource constraints, hooks awareness
- claude-flow-integration: Hybrid architecture, hook execution order, workers
- memory-workflow: Data workflow, memory separation, observation rules
- context-budget: Token tracking, skill loading limits, escalation thresholds
- capability-diagnostic: Failure analysis, missing capability identification
- escalation: Confidence-based model tier escalation
```

**Line count**: 55 lines. All behavioral rules preserved. All enforcement via hooks unchanged. All procedural knowledge accessible via skills on demand.

### Migration Strategy

**Phase 1** (Session 1): Create the 8 extraction skills with content copied from CLAUDE.md. Do NOT modify CLAUDE.md yet. Run all tests.

**Phase 2** (Session 2): Update agent specs to add `skills` field. Test that agents still function correctly with skills preloaded.

**Phase 3** (Session 3): Rewrite CLAUDE.md to the 55-line version. Run full test suite. Verify hooks still fire correctly (hooks are independent of CLAUDE.md content). Run optimizer agent to validate workflow.

**Rollback plan**: `git revert` the CLAUDE.md rewrite. Skills remain as supplementary content. Zero risk to hook infrastructure.

## 6. Research Gaps and Open Questions

### 6.1 No Empirical Data on Skill Preloading Limits

The research identifies the `skills` field in agent specs and the recommendation to keep preloaded skills under 500 lines total, but there is no published benchmark on how preloaded skill volume affects agent task performance. The "max 8 skills per API request" finding (R17) applies to on-demand matching, not preloading. **Open question**: What is the practical upper bound of preloaded skill content before agent performance degrades?

### 6.2 No Standard for Skill Versioning

R19 mentions "skills as software assets with versioning/testing/governance" but no concrete versioning scheme was found. The SKILL.md spec has only `name` and `description` in frontmatter -- no `version` field. **Open question**: How should skill versions be tracked when skills evolve with the project? Git history may suffice, but explicit versioning would enable dependency management.

### 6.3 Skill Testing Infrastructure Is Absent

R9 recommends capability evals, but there is no existing framework for testing skills in isolation. You can test hooks (shell scripts with exit codes), test code (cargo test), and test MCP connections, but testing whether a skill produces the intended agent behavior requires running the agent. **Open question**: Can skill effectiveness be measured without full agent execution? Could a "skill linter" validate structure and completeness?

### 6.4 Dynamic Skill Generation From Observed Behavior

R15 mentions systems that "dynamically generate skills from observed behavior." This is a tantalizing possibility for the Claude-Flow learning layer -- if the ultralearn worker detects repeated agent patterns, it could propose new skills. **Open question**: Is there a production-ready implementation of this pattern, or is it purely aspirational?

### 6.5 Claude-Flow Hook Interaction With Skills

The current architecture runs Claude-Flow hooks as non-blocking advisory layers. The research does not address whether Claude-Flow hooks could trigger skill loading or modify which skills are available to an agent mid-session. **Open question**: Can hooks influence skill availability dynamically, or is skill loading fixed at session/spawn time?

### 6.6 Cross-Agent Skill Sharing at Runtime

When agents in a team discover something (a pattern, a decision), they communicate via SendMessage. Skills are loaded at spawn time. **Open question**: Can a running agent dynamically load a new skill mid-session, or must skill sets be fixed at initialization? If fixed, team learning during a session can only happen via messages, not via skill injection.

### 6.7 Instruction Degradation Curve Shape

R10 states instruction degradation is uniform, but does not provide the degradation function. Is it linear (each added instruction degrades all equally)? Logarithmic (early additions matter most)? Threshold-based (fine until a cliff)? **Open question**: What is the quantitative relationship between instruction count and compliance rate?

## 7. Sources (Top 15 Most Cited)

1. **Claude Code Skills Docs** (code.claude.com/docs/en/skills) — Credibility 5. Cited by R1, R11, R12, R13, R14, R16, R18, R19, R20.
2. **Claude API Agent Skills Overview** (platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) — Credibility 5. Cited by R1, R11, R13, R15, R17, R18.
3. **Anthropic: Effective Harnesses for Long-Running Agents** — Credibility 5. Cited by R18, R19.
4. **Claude Blog: Skills Explained** (claude.com/blog/skills-explained) — Credibility 5. Cited by R16, R18.
5. **Claude Blog: Extending Claude with Skills and MCP** — Credibility 5. Cited by R16.
6. **Agent Skills Specification** (agentskills.io/specification) — Credibility 5. Cited by R13, R20.
7. **MCP Architecture Overview** (modelcontextprotocol.io/docs/learn/architecture) — Credibility 5. Cited by R7, R16.
8. **Claude Agent Skills Deep Dive** (leehanchung.github.io) — Credibility 4. Cited by R1, R13, R14, R15.
9. **Inside Claude Code Skills** (mikhail.io) — Credibility 4. Cited by R1, R14.
10. **Understanding Claude Code's Full Stack** (alexop.dev) — Credibility 4. Cited by R18, R19.
11. **Agent Skills for Context Engineering** (github.com/muratcankoylan) — Credibility 4. Cited by R15, R18.
12. **VS Code Agent Skills Docs** (code.visualstudio.com/docs/copilot/customization/agent-skills) — Credibility 5. Cited by R20.
13. **Anthropic: Demystifying Evals for AI Agents** — Credibility 5. Cited by R9.
14. **Sierra: τ-Bench Benchmarking AI Agents** — Credibility 3. Cited by R17.
15. **HumanLayer: Writing a Good CLAUDE.md** — Credibility 5. Cited by R10.

---

*Research methodology*: 20 parallel haiku researchers conducted independent web searches on assigned subtopics across 2 waves, producing ~198 raw claims with source attribution and credibility ratings. Claims were deduplicated (~40% overlap between waves), cross-referenced across sources (2+ independent sources with credibility >= 3 for HIGH confidence), and organized by theme. One opus synthesizer produced this final report with architecture blueprint and implementation plan. 4 researchers required respawning due to infrastructure errors (`classifyHandoffIfNeeded` transient bug). All 20 subtopics were ultimately covered.
