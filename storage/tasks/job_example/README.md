# Example Job Workflow

This directory shows the structure of a job executed by the agent system.

## Job Flow

### Step 1: Job Creation
When user requests: "Research AI news and write an article"

Sandy creates: `storage/tasks/job_NNN/`

### Step 2: Instructions
File: `instructions.md`
```markdown
# Job Instructions

## Research Phase (Zilla)
Task: Find 5 recent AI news stories
Query: "latest AI artificial intelligence news 2024 2025"
Output: Save to `raw_data.json`

## Writing Phase (Gonza)  
Task: Write article based on research
Input: `raw_data.json`
Output: Save to `article.md`
```

### Step 3: Agent Execution

**Zilla executes:**
- Loads config from `storage/agents/zilla.json`
- Uses tools: web_search, web_fetch, write_file
- Searches web using Tavily API
- Saves structured results to `raw_data.json`

**Verification:**
- Sandy checks: Does `raw_data.json` exist?
- Is file size > 0?
- Can file be read?

**If verification fails:** Stop workflow, report error

**Gonza executes:**
- Loads config from `storage/agents/gonza.json`
- Uses tools: read_file, write_file
- Reads `raw_data.json`
- Writes formatted article to `article.md`

**Verification:**
- Sandy checks: Does `article.md` exist?
- Is file size > 0?
- Can file be read?

**If verification fails:** Stop workflow, report error

### Step 4: Completion

Sandy reports to user:
"Complete. Files:
- Research: storage/tasks/job_NNN/raw_data.json (2.4KB)
- Article: storage/tasks/job_NNN/article.md (1.8KB)"

## File Structure

```
storage/tasks/job_001/
├── instructions.md     # Task description
├── raw_data.json       # Zilla's output (research data)
├── article.md          # Gonza's output (final article)
└── STATUS.txt          # Optional: execution status log
```

## Verification Checklist

Before claiming success:
- [ ] Job folder created
- [ ] instructions.md written
- [ ] Zilla spawned successfully
- [ ] raw_data.json exists and size > 0
- [ ] raw_data.json readable and valid
- [ ] Gonza spawned successfully
- [ ] article.md exists and size > 0
- [ ] article.md readable

If ANY check fails → Report error, do not claim success
