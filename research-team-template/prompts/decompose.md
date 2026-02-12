# Query Decomposition Template

## Task
Decompose a research query into ${num_subtopics} non-overlapping subtopics for parallel investigation.

## Query
"${query}"

## Requirements

1. **Non-overlapping**: Each subtopic covers a distinct aspect. No two subtopics should return the same search results.

2. **Comprehensive**: Together, all subtopics must cover the full scope of the query. After completing all subtopics, there should be no major aspect left unresearched.

3. **Searchable**: Each subtopic must be specific enough to yield relevant results from 2-3 web searches.

4. **Balanced**: Subtopics should be roughly equal in scope — avoid one massive subtopic and several trivial ones.

5. **Required angles**: At minimum, include:
   - One subtopic focused on **current state / recent developments** (2025-2026)
   - One subtopic focused on **challenges / limitations / criticisms**
   - One subtopic focused on **practical applications / real-world usage**

## Output Format

For each subtopic, specify:

```json
{
  "subtopics": [
    {
      "id": 1,
      "title": "Clear, descriptive subtopic name",
      "keywords": ["keyword1", "keyword2", "keyword3"],
      "angle": "What specific aspect this researcher should focus on",
      "rationale": "Why this subtopic is important to the overall query"
    }
  ]
}
```

## Coverage Validation
After generating subtopics, verify:
- [ ] No two subtopics would produce the same search results
- [ ] A reader of all subtopics combined would have a comprehensive understanding
- [ ] Each subtopic has 3-5 actionable keywords
- [ ] Recent developments and limitations are explicitly covered
