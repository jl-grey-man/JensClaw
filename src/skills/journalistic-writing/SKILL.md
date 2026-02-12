# Journalistic Writing Skill

## Overview
This skill transforms structured research data into readable, journalistic articles. It reads research JSON and produces well-formatted markdown articles.

## Usage

### Command Line
```bash
python3 write_article.py <input_path> <output_path> [options]
```

### Arguments
- `input_path`: Path to research JSON file (from run_research.py)
- `output_path`: Path where article markdown will be saved
- `--style`: (Optional) Writing style: "neutral", "analytical", "engaging". Default: "neutral"
- `--length`: (Optional) Article length: "short", "medium", "long". Default: "medium"

### Examples
```bash
# Standard article
python3 write_article.py storage/tasks/job_001/raw_data.json storage/tasks/job_001/article.md

# Analytical style, short length
python3 write_article.py storage/tasks/job_002/research.json storage/tasks/job_002/article.md --style analytical --length short

# Engaging style for general audience
python3 write_article.py storage/tasks/job_003/data.json storage/tasks/job_003/article.md --style engaging
```

## Input Format
Expects JSON from run_research.py:
```json
{
  "query": "research topic",
  "results": [
    {
      "title": "Source Title",
      "url": "https://example.com/article",
      "snippet": "Content...",
      "source": "example.com"
    }
  ],
  "sources": ["example.com"],
  "summary": "Brief summary"
}
```

## Output Format
Produces markdown article:
```markdown
# Article Title

## Summary
Brief overview of key points.

## Main Content
Well-structured paragraphs with citations.

## Sources
- [Source Title](URL) - example.com
- [Another Title](URL) - another-site.com
```

## Error Handling
- If input file is missing/invalid, writes ERROR message to output
- If research data contains errors, reports them
- Returns non-zero exit code on failure
- Sandy reads error and reports to user

## Constraints
- NO web access - must use ONLY provided input file
- NO additional research - only facts from input
- MUST cite all sources with URLs
- MUST stick to journalistic style (clear, factual, structured)
- MUST preserve all important details from source

## Cost Optimization
- Zero API costs - uses local processing only
- No LLM calls in this script (transformation is template-based)
- Keeps article generation lightweight and fast
- If advanced transformation needed, Sandy uses sub_agent tool separately
