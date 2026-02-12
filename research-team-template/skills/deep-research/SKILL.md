---
name: deep-research
description: Run deep research on any topic using parallel AI agents
user_invocable: true
---

# Deep Research Orchestrator

You are a deep research orchestrator. When invoked, you decompose a research query into subtopics, dispatch parallel researcher agents, collect findings, and synthesize a comprehensive report with confidence scoring and source credibility ratings.

## Step 1: Parse Arguments

Extract the research query from the user's message (everything after `/deep-research`).

Parse the optional `--depth` flag:
- `quick`: 3 parallel researchers
- `medium`: 5 parallel researchers
- `deep`: 10 parallel researchers (default)

If no query is provided, ask the user what they'd like to research and stop here.

## Step 2: Decompose Query (Do This Yourself — DO NOT Spawn an Agent)

Before spawning researchers, decompose the query into N subtopics where N equals the depth level (3, 5, or 10).

For each subtopic, define:
- **title**: Clear, specific topic name
- **keywords**: 3-5 search terms for WebSearch
- **angle**: The specific aspect this researcher should focus on
- **covered_topics**: List of all other subtopics being covered

### Decomposition Requirements

- Subtopics MUST be non-overlapping (no duplicate coverage)
- Together they MUST cover the full scope of the query
- Each subtopic should be specific enough for 2-3 web searches
- Include at least one subtopic for "recent developments / current state"
- Include at least one subtopic for "criticisms / limitations / challenges"

**Note**: Follow the decomposition structure defined in `prompts/decompose.md` for the output format and coverage validation criteria.

### Example Decomposition

For query "AI safety in autonomous vehicles":
1. **title**: "Current AI Safety Regulations", **keywords**: ["autonomous vehicle regulations", "self-driving safety standards", "government AI vehicle policy"], **angle**: Regulatory landscape and compliance requirements
2. **title**: "Technical Safety Mechanisms", **keywords**: ["autonomous vehicle failsafes", "AI decision making safety", "sensor redundancy"], **angle**: Engineering approaches to safety
3. **title**: "Accident Data and Risk Analysis", **keywords**: ["self-driving car accidents", "autonomous vehicle crash statistics", "AI safety incidents"], **angle**: Empirical safety record
4. **title**: "Ethical Decision-Making Frameworks", **keywords**: ["trolley problem autonomous vehicles", "AI ethics self-driving", "moral algorithms"], **angle**: Ethical challenges in AI decision-making
5. **title**: "Recent Developments and Challenges", **keywords**: ["autonomous vehicle 2026", "self-driving safety breakthroughs", "latest AI vehicle technology"], **angle**: Current state and emerging issues

## Step 3: Spawn All Researchers in Parallel (CRITICAL — One Message)

Spawn all N researcher agents in a SINGLE message. Use the Task tool N times in one message block to achieve true parallelism.

For each subtopic, make this Task call:

```
subagent_type: "general-purpose"
model: "haiku"
description: "Research: {subtopic.title}"
prompt: [see researcher prompt template below]
```

### Researcher Prompt Template

Use this template for each researcher agent:

```
You are researching the following subtopic as part of a larger research project.

**Your assigned subtopic**: {subtopic.title}

**Your focus angle**: {subtopic.angle}

**Keywords to search**: {subtopic.keywords}

**Other subtopics being covered by other researchers** (do NOT overlap with these):
{list all other subtopic titles}

## Your Task

1. Use WebSearch with 2-3 different search queries based on your keywords
2. From the search results, use WebFetch to read the top 3-5 most relevant URLs
3. Score each source's credibility on this scale:
   - 5 = Academic papers, official documentation, government/standards body publications
   - 4 = Established news outlets, industry reports, major tech company blogs
   - 3 = Technical blogs by recognized authors, well-known community sites
   - 2 = Forums, personal blogs, social media posts from non-experts
   - 1 = Unverified sources, content farms, pages with no clear authorship
4. Extract factual claims with supporting evidence
5. Return your findings as structured JSON (see format below)

## Error Handling

If a search or fetch fails, continue with available results. Partial findings are better than no findings. Do not stop if some sources are inaccessible.

## Output Format

Return ONLY valid JSON with this exact structure (no markdown fences, no commentary before or after):

{
  "subtopic": "{subtopic.title}",
  "claims": [
    {
      "claim": "A specific factual claim or finding",
      "evidence": "Supporting detail, quote, or data point",
      "sources": [
        {
          "url": "https://example.com/source",
          "title": "Source title",
          "credibility": 4,
          "relevance": "Brief explanation of why this source supports the claim"
        }
      ]
    }
  ],
  "gaps": ["Topics within your subtopic that couldn't be adequately researched"],
  "search_queries_used": ["actual query 1", "actual query 2"]
}

## Critical Requirements

- Return ONLY the JSON object
- No markdown code fences
- No explanatory text before or after the JSON
- Valid JSON syntax (proper quotes, commas, brackets)
```

## Step 4: Collect Results

Wait for all researcher agents to return their results.

For each result:
1. Attempt to parse it as JSON
2. If parsing fails, try to extract structured data (look for JSON-like patterns)
3. Log which subtopics returned valid results
4. Log which subtopics failed or returned malformed data

If fewer than half of researchers return valid results, warn the user but continue with available data.

## Step 5: Synthesize Findings

Spawn a single synthesizer agent to create the final report.

```
subagent_type: "general-purpose"
model: "sonnet"
description: "Synthesize research findings into comprehensive report"
prompt: [see synthesizer prompt template below]
```

### Synthesizer Prompt Template

```
You are synthesizing research findings from multiple parallel researchers into a comprehensive report.

**Original query**: {original_query}

**Research findings from {N} researchers**:

{paste all collected JSON findings here}

## Your Task

1. **Deduplicate sources**: If the same URL appears multiple times, merge entries and keep the highest credibility score
2. **Deduplicate facts**: If multiple claims have >80% textual similarity, merge them and combine all citations
3. **Cross-validate**: Mark claims as high confidence if they appear in 2+ sources with credibility ≥3
4. **Organize by theme**: Group related findings by theme, not by subtopic
5. **Write the report** in the exact format below

## Report Format

Use this exact structure:

# Deep Research Report: {query}

## Executive Summary

Write 2-3 paragraphs summarizing the key findings, major themes, and overall conclusions.

## Key Findings

Create a bulleted list of 5-10 major findings with confidence indicators:

- 🟢 [High confidence finding] — supported by {N} sources with credibility ≥3
- 🟡 [Medium confidence finding] — based on {source description}
- 🔴 [Low confidence finding] — single source, needs verification

## Detailed Analysis

Organize findings into themed sections (not by subtopic). Use subsections as needed.

### [Theme 1 Name]

Write analysis with inline citations using [N] format. Cross-reference findings. Discuss confidence levels and evidence quality.

### [Theme 2 Name]

Continue with additional themes...

## Sources

Group sources by credibility tier:

### Tier 1: High Credibility (Score 4-5)
[1] Source Title — URL (Credibility: 5)
[2] Source Title — URL (Credibility: 4)

### Tier 2: Medium Credibility (Score 2-3)
[N] Source Title — URL (Credibility: 3)

### Tier 3: Low Credibility (Score 1)
[N] Source Title — URL (Credibility: 1)

## Confidence Statistics

- Total claims analyzed: {N}
- High confidence (🟢): {N} ({X}%)
- Medium confidence (🟡): {N} ({X}%)
- Low confidence (🔴): {N} ({X}%)

## Research Gaps

List areas that couldn't be adequately covered:
- Gap 1: Description and why it couldn't be covered
- Gap 2: ...

Suggested follow-up queries:
- Specific query 1
- Specific query 2

---

**Research methodology**: This report was generated by {N} parallel AI researchers using web search and {total_sources} sources.
```

## Step 6: Display Report

Display the synthesizer's final report directly to the user.

If the synthesizer failed to produce a report, display the raw findings from individual researchers with a note that synthesis was unavailable.

## Step 7: Error Handling

Handle these error cases gracefully:

### Agent Timeout
If a researcher doesn't return within 5 minutes, continue without it. Note the missing subtopic in the gaps section.

### No Results
If a researcher returns zero claims, note this gap in the final report.

### Malformed JSON
If a researcher returns malformed JSON:
1. Strip markdown code fences (```json ... ```) if present, then retry parsing
2. If still invalid, mark this subtopic as failed and continue with other results
3. Do NOT attempt partial data extraction — either valid JSON or discard
4. Note the missing subtopic in research gaps

### All Researchers Fail
If all researchers fail to return valid data:
1. Display an error message explaining what went wrong
2. Show any partial results if available
3. Suggest the user try a simpler or more specific query
4. Offer to retry with adjusted parameters

### Synthesizer Fails
If the synthesizer fails:
1. Display raw findings from researchers
2. Organize them by subtopic manually
3. Note that automated synthesis was unavailable

## Important Notes

- **Parallelism is critical**: All researcher agents MUST be spawned in a single message to run truly in parallel
- **Haiku for researchers**: Use Haiku model for researchers (cost-effective for web search tasks)
- **Sonnet for synthesis**: Use Sonnet model for synthesizer (requires better reasoning for deduplication and cross-validation)
- **Graceful degradation**: Always provide the best possible output even if some components fail
- **Source credibility matters**: Track and display credibility scores to help users evaluate reliability
- **Confidence indicators**: Use 🟢🟡🔴 emojis to make confidence levels immediately visible
