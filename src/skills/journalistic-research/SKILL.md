# Journalistic Research Skill

## Overview
This skill performs structured web research and saves results as JSON. It uses the DuckDuckGo search API to find information and formats it as structured data.

## Usage

### Command Line
```bash
python3 run_research.py "<query>" <output_path> [search_depth]
```

### Arguments
- `query`: The search query (enclose in quotes if contains spaces)
- `output_path`: Path where results JSON will be saved
- `search_depth`: (Optional) "basic" or "advanced" - controls thoroughness of search. Default: "basic"

### Examples
```bash
# Basic research
python3 run_research.py "AI news 2026" storage/tasks/job_001/raw_data.json

# Advanced deep research
python3 run_research.py "climate change solutions" storage/tasks/job_002/research.json advanced

# Search with quotes in query
python3 run_research.py '"machine learning" healthcare' storage/tasks/job_003/ml_health.json
```

## Output Format
The script produces structured JSON:
```json
{
  "query": "original search query",
  "timestamp": "2026-02-12T10:30:00Z",
  "search_depth": "basic",
  "results": [
    {
      "title": "Article Title",
      "url": "https://example.com/article",
      "snippet": "Brief description...",
      "source": "example.com"
    }
  ],
  "sources": ["example.com", "another-source.com"],
  "summary": "Brief summary of findings"
}
```

## Error Handling
- If search fails, writes ERROR message to output file
- Returns non-zero exit code on failure
- Sandy reads the error and reports to user

## Constraints
- This skill only performs research
- Cannot write articles or creative content
- Must cite all sources with URLs
- Saves raw data only (does not analyze)

## Cost Optimization
- Uses existing web_search tool (no additional API costs)
- Basic search: ~1 web search per query
- Advanced search: ~3 web searches per query (deeper results)
- Saves results locally to avoid repeated searches
