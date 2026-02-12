# This Is What We Do

## The Tool

git-review is a terminal tool that sits between you and your code changes. When you run `git diff`, you get a wall of text — additions, deletions, hunks scattered across files. You're supposed to look at all of it before you commit or merge. Nobody does. Not properly.

git-review makes you do it properly. It parses your diff, breaks it into hunks, and presents them one at a time in an interactive terminal UI. You navigate with keyboard shortcuts. You mark each hunk as reviewed. It tracks your progress in a local SQLite database, keyed by the SHA-256 hash of each hunk's content. If the code changes, your review goes stale. You have to re-review.

There's a gate. A pre-commit hook that blocks your commit if you haven't reviewed everything. You can't skip it. That's the point.

It's written in Rust — about 2,000 lines. ratatui for the terminal UI, rusqlite for persistence, clap for CLI parsing, sha2 for content hashing. It has syntax highlighting. It has filters. It tracks staleness. It does one thing and does it well.

## How It Got Built

Here's where it gets interesting.

git-review was not typed out by a human in an editor. It was built by an AI orchestration system — a single Claude Opus instance (the "orchestrator") coordinating multiple Claude agents, each running in their own tmux pane, each with a specific role and a specific language model.

The orchestrator doesn't write code. That's a hard rule, enforced by a shell hook that blocks any Write or Edit tool calls from the orchestrator session. The orchestrator's job is to think, plan, delegate, and review. It creates teams, spawns agents, assigns tasks, watches them work, and integrates the results.

The agents show up as tmux split panes in the terminal. You can watch them work. You can see the coder writing a parser, the reviewer reading the code, the QA agent running tests — all happening in parallel, all visible. This was a deliberate choice. No invisible background processes. If an agent is working, you can see it.

## The Agent Pipeline

Every feature follows a pipeline. Not every step is always needed, but the full sequence looks like this:

1. **Requirements Interviewer** — Asks the human clarifying questions. What exactly do you want? What are the edge cases? What's out of scope? Produces a spec.

2. **Explorer** — Researches. Looks at existing code, reads documentation, investigates libraries. Finds prior art and trade-offs.

3. **Architect** — Reads the codebase (using semantic code intelligence, not grep), then designs module boundaries, data flow, type definitions. Proposes where new code should go and how it connects to what exists.

4. **Planner** — Takes the architecture and writes a step-by-step implementation plan. This is the only agent allowed to write plan files. It specifies exactly which files to create, which to modify, what functions to implement, and which agent should do each step.

5. **Red Teamer** — Reads the plan and tries to destroy it. Finds bugs before they're written. Identifies edge cases, performance risks, security issues, over-engineering. Can block implementation if it finds critical problems. This is adversarial review of the design, not the code.

6. **Junior Coder** — Handles the mechanical work. Scaffolding new files, writing struct definitions from the architect's spec, adding module declarations, creating boilerplate. Uses Haiku (the fastest, cheapest model) because the work requires no reasoning — just typing what the spec says.

7. **Coder** — The workhorse. Writes real implementation code with logic, error handling, tests. Follows the planner's instructions but makes tactical decisions about how to implement each function. Uses Sonnet.

8. **Senior Coder** — For the hard stuff. Cross-module refactors, performance-critical code, subtle concurrency issues, anything where the coder might get it wrong. Uses Opus. This agent exists because some code changes require understanding the entire system at once.

9. **Reviewer** — Reads the finished code and checks quality. Catches bugs, style issues, missing error handling. This is post-implementation review, different from the red teamer's pre-implementation design review.

10. **QA** — Doesn't just read code — runs it. Executes tests, tries edge cases, verifies that things actually work. The reviewer thinks about correctness; QA proves it.

11. **Documentation** — Updates README, doc comments, guides. Uses Haiku because it's summarization work.

12. **Explainer** — Can explain any piece of code at any level, from "explain it like I'm a junior developer" to "what are the architectural trade-offs and invariants?" Uses Haiku.

13. **Optimizer** — Runs after every major task completion. Audits the workflow itself. Did we waste tokens? Could agents have been parallelized better? Are there process improvements? Meta-improvement.

## The Models

Not every agent needs the smartest model. That was a key insight.

**Opus** (3 agents) — The most capable, most expensive model. Reserved for tasks where mistakes can't be caught downstream: the planner (bad plan = wasted implementation), the red teamer (must find what everyone else missed), and the senior coder (complex code that Sonnet can't handle).

**Sonnet** (6 agents) — The balanced choice. Good enough for real work, fast enough to be practical. The coder, reviewer, QA, architect, explorer, and requirements interviewer all use Sonnet. If they make a mistake, it gets caught by the quality gates.

**Haiku** (4 agents) — The fastest and cheapest. Used for work that's mechanical or advisory: junior coder (scaffolding from spec), documentation (summarization), explainer (description), optimizer (suggestions). If these agents produce something weak, the cost is low — the human can edit docs, ignore a suggestion, or redo a scaffold.

The orchestrator itself runs on Opus. It needs to see the whole picture, make routing decisions, and coordinate everything. That's the hardest reasoning task of all.

## The Enforcement Layer

The pipeline isn't just a suggestion. It's enforced by shell hooks that run before every tool call Claude makes.

- **enforce-orchestrator-delegation** — Blocks the orchestrator from writing code. If the orchestrator tries to create or edit a source file, the hook denies it. Delegation is mandatory.

- **enforce-ticket** — Every piece of work needs a Linear ticket. No ticket, no work. The hook checks the branch name for a ticket ID. On main without a ticket, it warns you. (It used to block you. We fixed that because it was blocking questions too.)

- **protect-main** — Can't commit directly to main. Ever. Create a branch.

- **enforce-branch-naming** — Branch names must follow the pattern: `type/ticket-id-description`. No exceptions.

- **enforce-review-gate** — Can't merge to main without completing a git-review. The tool reviews its own code. (We found and fixed a bug where `git merge --no-ff` bypassed this check because the regex captured `--no-ff` instead of the branch name.)

- **enforce-visible-agents** — Agents must run in tmux panes. No invisible background processes. If you can't see it working, it shouldn't be working.

- **enforce-plan-files** — Only the planner agent writes plan files. (We fixed this too — the hook couldn't identify planner teammates because the session ID didn't contain "planner".)

- **enforce-task-quality** — When a task is marked complete, the hook runs `cargo test` and `cargo clippy`. If they fail, the completion is blocked. You can't say "done" if it's not done.

- **protect-hooks** — The hooks protect themselves. If you try to modify a hook file, you get prompted. This prevents agents from disabling their own guardrails.

Every hook was written, tested, broken, and fixed during the development of git-review. The hooks are themselves part of the codebase they protect. They evolve alongside the tool.

## The Memory Systems

The system remembers across sessions. Four separate memory systems, each with a distinct purpose:

- **Linear** — The source of truth for what needs to be done. Every ticket, every requirement, every acceptance criterion. Agents read from Linear to know what to build. Status updates go to Linear.

- **claude-mem** — Cross-session memory for human decisions. "The user prefers visible agents." "Commit messages should not have Co-Authored-By lines." "The orchestrator must not code." These are preferences and conventions that persist across conversations.

- **MEMORY.md** — Auto-memory for the orchestrator. Project structure, build status, agent roster, model routing. Loaded into every session automatically.

- **Serena** — Semantic code intelligence via LSP. Knows the structure of every symbol in the codebase — functions, structs, methods, their relationships. Agents can ask "what methods does ReviewDb have?" without reading the entire file. Saves 50-75% of the tokens that would be spent reading source code.

No overlap. Linear tracks requirements. claude-mem tracks preferences. MEMORY.md tracks project state. Serena tracks code structure. Each system serves one purpose.

## What Just Happened

In the session that produced this document, here's what happened:

1. The user asked how to install git-review as a terminal command. (`cargo install --path .`)

2. The enforce-ticket hook blocked the question because we were on main. We fixed the hook — questions should never be blocked, only work. That became **ENG-30**.

3. Both stop hooks (enforce-memory, check-claude-flow-memory) were outputting invalid JSON for the Stop event type. They used `hookSpecificOutput.additionalContext` which only works for UserPromptSubmit events. We fixed them to use `stopReason`. Also part of **ENG-30**.

4. The user asked about the current state of git-review and what's missing for a complete workflow. We identified the gap: git-review can review diffs, but you have to specify the range manually. There's no branch picker, no merge button, no live updates. The user wants to launch `git-review`, see all branches, pick one, review it, and merge — all without leaving the TUI.

5. We created **ENG-31** with four sub-tickets: ENG-32 (dashboard view), ENG-33 (navigation), ENG-34 (merge), ENG-35 (live refresh).

6. The requirements interviewer agent asked the user clarifying questions. Branch scope? Remote branches too? Merge behavior? Edge cases? The answers became a detailed spec attached to each Linear ticket.

7. We reconsidered the model routing. The reviewer moved from Opus to Sonnet (the red teamer already catches design issues; automated tools catch code issues). Documentation, explainer, and optimizer moved from Sonnet to Haiku (descriptive/advisory tasks don't need deep reasoning). That became **ENG-37** after the user suggested adding a junior coder on Haiku for scaffolding work.

8. The review gate hook had a bug: `git merge --no-ff branch-name` captured `--no-ff` as the branch name instead of `branch-name`. The hook couldn't check review status for a flag. We fixed it with proper argument parsing that strips flags and quoted strings. That was **ENG-36**.

9. The architect agent read the entire codebase using Serena's symbol tools and designed the architecture: two new modules (git operations, dashboard state), a ViewMode enum for TUI state transitions, lazy loading for performance, pagination for large branch lists.

10. The planner agent (Opus) wrote a step-by-step implementation plan with agent assignments. Junior coder scaffolds the types. Coder implements the logic. Senior coder handles the risky TUI refactor. The plan got blocked by the enforce-plan-files hook — even the planner couldn't write to `plans/` because the hook couldn't identify it. We fixed that too. **ENG-38**.

11. The plan file was written. The red teamer is next — to find problems before we build.

## The Meta Layer

There's something recursive about all of this.

git-review is a tool for reviewing code changes before they're merged. It enforces that a human looks at every hunk. The review gate blocks merges until the review is complete.

git-review is itself developed using an AI system that has its own review gates. The orchestrator can't write code. The red teamer critiques designs before implementation. The reviewer checks code after implementation. The QA agent runs tests. The enforce-task-quality hook blocks task completion if tests fail.

And then the human uses git-review to review the AI's code before merging it. `git-review main..fix/eng-36-review-gate-no-ff-bypass`. The human marks each hunk as reviewed. The gate passes. The merge happens.

The tool reviews the code that builds the tool. The hooks protect the hooks. The agents enforce the rules that constrain the agents.

It's not clever. It's just layers of accountability. Every layer exists because at some point, something went wrong without it. The orchestrator wrote bad code, so we added the delegation hook. The review gate was bypassed, so we fixed the argument parser. The hook itself had a bug, so we tested it with 8 different merge command patterns.

Each fix makes the system slightly more trustworthy. Not perfect. Just better than yesterday.

## What This Is

This is a development workflow where:

- A human decides what to build
- An AI orchestrator breaks it down and delegates
- Specialized AI agents implement, review, and test
- Shell hooks enforce process rules at every step
- The human reviews every change before it's merged
- A tool (that the AI built) ensures the human actually does the review

The human stays in control. The AI does the work. The hooks keep everyone honest. The tool closes the loop.

That's what we do.
