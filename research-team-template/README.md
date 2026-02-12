# Deep Research Team Template

A portable template for running deep research queries using parallel AI agents. Decomposes questions into subtopics, dispatches parallel researchers, cross-validates findings, and synthesizes results into comprehensive cited reports.

## Architecture

```
User: /deep-research "What are best practices for distributed consensus in 2026?"
         ↓
┌─────────────────────────────────────────────────────────────────┐
│ Orchestrator (SKILL.md)                                         │
│                                                                  │
│ 1. Decompose Query → N Subtopics                                │
│    - Quick: 3 subtopics                                         │
│    - Medium: 5 subtopics                                        │
│    - Deep: 10 subtopics                                         │
│                                                                  │
│ 2. Spawn N Haiku Researchers (parallel, single message)         │
│    ├── Researcher 1: Subtopic A                                 │
│    ├── Researcher 2: Subtopic B                                 │
│    └── Researcher N: Subtopic N                                 │
│                                                                  │
│ 3. Each Researcher:                                             │
│    - WebSearch for keywords                                     │
│    - WebFetch top sources                                       │
│    - Score credibility (1-5)                                    │
│    - Extract claims + citations                                 │
│    - Return structured JSON                                     │
│                                                                  │
│ 4. Orchestrator Processes Results:                              │
│    - Deduplicate sources (same URL → merge)                     │
│    - Deduplicate facts (>80% similar → merge citations)         │
│    - Cross-validate (2+ sources → high confidence)              │
│                                                                  │
│ 5. Spawn 1 Sonnet Synthesizer                                   │
│    - Aggregate findings by theme                                │
│    - Write executive summary                                    │
│    - Produce detailed analysis with citations                   │
│    - Group sources by credibility tier                          │
│    - Identify research gaps                                     │
│                                                                  │
│ 6. Return Markdown Report                                       │
└─────────────────────────────────────────────────────────────────┘
```

## Installation

Copy the template into your project's `.claude/` directory:

```bash
# From the research-team-template directory
cp -r skills/deep-research <your-project>/.claude/skills/
cp agents/deep-*.md <your-project>/.claude/agents/

# Verify installation
ls <your-project>/.claude/skills/deep-research/SKILL.md
ls <your-project>/.claude/agents/deep-researcher.md
ls <your-project>/.claude/agents/deep-synthesizer.md
```

No external dependencies required beyond Claude Code's built-in tools.

## Usage

Invoke the skill with your research query:

```bash
# Default depth (deep = 10 agents)
/deep-research "What are the best practices for distributed consensus in 2026?"

# Quick mode (3 agents, ~3 minutes)
/deep-research --depth quick "Overview of Rust async runtimes"

# Medium mode (5 agents, ~5 minutes)
/deep-research --depth medium "Compare React vs Svelte for enterprise apps"

# Explicit deep mode (10 agents, ~10 minutes)
/deep-research --depth deep "Comprehensive analysis of quantum computing progress"
```

## Depth Levels

| Level  | Agents | Est. Tokens | Est. Time | Best For                              |
|--------|--------|-------------|-----------|---------------------------------------|
| quick  | 3      | ~100K       | 2-3 min   | Quick overviews, preliminary research |
| medium | 5      | ~150K       | 4-5 min   | Balanced depth and breadth            |
| deep   | 10     | ~260K       | 7-10 min  | Comprehensive analysis, thorough research |

**Timing Notes:**
- Times assume average internet connection and no API rate limiting
- Actual time varies based on query complexity and source availability
- Token counts are estimates; actual usage depends on source length and finding complexity

## Output Format

The research report is a structured markdown document with the following sections:

### 1. Executive Summary

3-5 paragraph overview synthesizing the most important insights across all subtopics. Provides context, key trends, and actionable takeaways.

### 2. Key Findings

Bullet list of 8-12 top findings with confidence indicators:

```markdown
- 🟢 **High Confidence**: Raft and Paxos remain dominant consensus algorithms in production systems [1][2][3]
- 🟡 **Medium Confidence**: New variants like EPaxos show promise for geo-distributed deployments [4]
- 🔴 **Low Confidence**: Quantum-resistant consensus protocols are still experimental [5]
```

### 3. Detailed Analysis

Thematic sections (not organized by subtopic) with full explanations and inline citations:

```markdown
## Consensus Algorithm Evolution

The landscape of distributed consensus has matured significantly since 2020. Raft continues
to dominate cloud-native deployments due to its understandability and proven reliability
in systems like etcd and Consul [Source Name](URL). Meanwhile, Byzantine Fault Tolerant
protocols have gained traction in blockchain applications, with practical implementations
like HotStuff demonstrating linear communication complexity [Another Source](URL).
```

### 4. Sources

Grouped by credibility tier (highest to lowest):

```markdown
## Sources

### Tier 5: Academic & Official Documentation
1. [Raft Consensus Algorithm](https://raft.github.io/) - Official Raft specification
2. [USENIX Paper: EPaxos Analysis](https://example.com) - Peer-reviewed performance study

### Tier 4: Industry & Established News
3. [AWS: Building Distributed Systems](https://aws.amazon.com/...) - Cloud provider guidance
4. [Google SRE Book: Consensus Chapter](https://sre.google/...) - Production insights

### Tier 3: Technical Blogs
5. [Martin Kleppmann: Consensus Explained](https://example.com) - Well-known distributed systems expert
```

### 5. Confidence Statistics

Summary of finding quality:

```markdown
## Confidence Statistics

- High Confidence (🟢): 45 findings (62%)
- Medium Confidence (🟡): 22 findings (30%)
- Low Confidence (🔴): 6 findings (8%)

Total Sources: 48 (18 Tier 5, 15 Tier 4, 10 Tier 3, 4 Tier 2, 1 Tier 1)
```

### 6. Research Gaps

Topics that lacked sufficient sources or had conflicting information:

```markdown
## Research Gaps & Limitations

- **Limited data on quantum consensus**: Only 2 sources found, both experimental
- **Conflicting performance claims**: EPaxos vs Multi-Paxos benchmarks varied widely across studies
- **Emerging protocols**: Several newer algorithms (CockroachDB's consensus, TiDB's variant) lack independent analysis
```

## Confidence Indicators

The system assigns confidence levels to each finding based on source quality and cross-validation:

- **High confidence** (🟢): Supported by 2+ independent sources with credibility ≥ 3
  - Example: "Raft is widely used in production" (confirmed by AWS docs, Google SRE book, academic papers)

- **Medium confidence** (🟡): Supported by 1 source with credibility ≥ 3, OR 2+ sources with credibility < 3
  - Example: "EPaxos reduces latency by 30%" (single academic paper, not yet widely replicated)

- **Low confidence** (🔴): Single source with credibility ≤ 2
  - Example: "New consensus algorithm X solves Y" (claimed in blog post, no peer review or adoption data)

**Source Independence:** Sources are considered independent if they meet ALL of the following:
- Different domains (not just different URLs on the same site)
- Different authors OR different organizations
- Original reporting (not republished/copied content)

Note: Same publication date does NOT prevent sources from being independent.

## Troubleshooting

### "Agent timeout" or "No response from researcher"

**Cause:** Network issues, rate limiting, or very slow source loading.

**Solution:**
- Reduce depth level (`--depth quick` or `--depth medium`)
- Check internet connectivity
- Retry after a few minutes if search API rate limits apply

### "No results for subtopic X"

**Cause:** Topic may be too niche, technical terminology mismatched, or genuinely lacking online sources.

**Solution:**
- Try broader query terms
- Check if topic is too new/specialized for public documentation
- Review the "Research Gaps" section of partial results for insights

### "Malformed JSON from researcher"

**Cause:** Researcher agent returned invalid structured output due to parsing errors or unexpected source content.

**Solution:**
- Partial results from other researchers will still be included in synthesis
- The synthesizer notes the missing subtopic in "Research Gaps"
- If this happens frequently, check for special characters in query that might confuse JSON formatting

### "Low confidence on most findings"

**Cause:** Query topic may be emerging, controversial, or poorly documented online.

**Solution:**
- Consider increasing depth level to find more sources
- Review individual source credibility scores to assess quality
- Use findings as starting point for further manual research

### "Duplicate findings with different wording"

**Cause:** Deduplication threshold (80% similarity) may not catch all semantic duplicates.

**Expected behavior:** Minor variations in wording are preserved to maintain nuance. Check citation lists to see if facts are truly from multiple sources or restated from the same source.

## File Structure

```
research-team-template/
├── README.md                          # This file
├── requirements.md                    # Full requirements specification
├── skills/
│   └── deep-research/
│       └── SKILL.md                   # Skill entry point (orchestrator logic)
├── agents/
│   ├── deep-researcher.md             # Haiku researcher agent spec
│   └── deep-synthesizer.md            # Sonnet synthesizer agent spec
└── prompts/
    ├── decompose.md                   # Query decomposition prompt template
    ├── research.md                    # Researcher instruction prompt
    └── synthesize.md                  # Synthesizer instruction prompt
```

## Technical Details

### Agent Roles

**Orchestrator (Skill):**
- Parses user query and depth argument
- Decomposes query into N subtopics using `prompts/decompose.md`
- Spawns N researcher subagents in parallel (single Task message)
- Deduplicates and cross-validates results
- Spawns synthesizer subagent with aggregated findings
- Returns final markdown report to user

**Researcher Agents (deep-researcher, haiku):**
- Receives single subtopic assignment
- Uses WebSearch to find relevant sources
- Uses WebFetch to read top 5-10 sources
- Scores each source credibility (1-5)
- Extracts claims and data points
- Returns structured JSON: `{"subtopic": "...", "claims": [...], "sources": [...]}`

**Synthesizer Agent (deep-synthesizer, sonnet):**
- Receives all deduplicated, scored, validated findings
- Identifies thematic patterns across subtopics
- Writes executive summary and detailed analysis
- Formats citations and source list
- Calculates confidence statistics
- Identifies research gaps

### Credibility Scoring Rules

Researchers apply these rules consistently:

| Score | Domain Authority | Author Credentials | Publication Type |
|-------|------------------|-------------------|------------------|
| 5 | .edu, .gov, official org docs | PhD, recognized researcher | Peer-reviewed, official spec |
| 4 | Established tech companies | Industry expert, known engineer | Company engineering blog, major news |
| 3 | Popular dev platforms | Active community contributor | Technical blog, conference talk |
| 2 | Q&A sites, forums | Community member | Stack Overflow answer, Reddit thread |
| 1 | Unknown domain | No clear credentials | Anonymous blog, no attribution |

When in doubt, researchers err on the side of lower scores.

### Deduplication Algorithm

**Source-level:**
```python
# Pseudocode
sources_by_url = {}
for source in all_sources:
    if source.url in sources_by_url:
        # Merge: keep highest credibility, union of claims
        existing = sources_by_url[source.url]
        existing.credibility = max(existing.credibility, source.credibility)
        existing.claims.extend(source.claims)
    else:
        sources_by_url[source.url] = source
```

**Fact-level:**
```python
# Pseudocode
def similarity(claim1, claim2):
    # Compute embedding cosine similarity or Levenshtein ratio
    return cosine_similarity(embed(claim1), embed(claim2))

deduplicated_claims = []
for claim in all_claims:
    matched = False
    for existing in deduplicated_claims:
        if similarity(claim.text, existing.text) > 0.8:
            # Merge: union of citations, keep most detailed text
            existing.citations.extend(claim.citations)
            if len(claim.text) > len(existing.text):
                existing.text = claim.text
            matched = True
            break
    if not matched:
        deduplicated_claims.append(claim)
```

## Contributing

This template is part of the `git-review` repository. Improvements and bug fixes welcome:

1. Test changes on real research queries (verify citation accuracy, confidence scoring)
2. Run the template on 3+ diverse queries before submitting changes
3. Update this README and `requirements.md` if behavior changes
4. Follow the project's CLAUDE.md instructions for commits and PRs

## License

This template is distributed with the `git-review` project. See repository LICENSE for details.

## Credits

Inspired by Gemini Deep Research and ChatGPT Deep Research. Implemented using Claude Code's Task tool and parallel subagent capabilities.