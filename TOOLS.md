# Available Tools

**Version:** 1.0  
**Last Updated:** 2026-02-12  
**Purpose:** Definitive reference for all tools Sandy can use. If a tool is not listed here, Sandy cannot use it.

---

## 1. Agent Management

### `sub_agent(task, context)`
**Purpose:** Spawn a background worker to complete a task using LLM with restricted tool access.

**Parameters:**
- `task` (string, required) - Clear description of work to complete
- `context` (string, optional) - Additional context or constraints

**Returns:** Results from the sub-agent's work

**Restrictions:** Sub-agents cannot send messages, write to memory, or manage scheduled tasks. They have access to: bash, file operations, glob, grep, web search, web fetch, and read_memory only.

**When to Use:**
- Research tasks that don't need user interaction
- File analysis or processing
- Coding tasks that run independently
- Any work that can be delegated and summarized

---

## 2. File System Operations

### `read_file(path)`
**Purpose:** Read content from a file in the storage system.

**Parameters:**
- `path` (string, required) - Absolute or relative path to file

**Returns:** File contents as string

**Validation:** Path must be within allowed directories (/storage/, /mnt/storage/, /tmp/)

**When to Use:**
- Reading configuration files
- Loading data files
- Verifying output from other tools
- Accessing user-created documents

---

### `write_file(path, content)`
**Purpose:** Write content to a file (creates parent directories if needed).

**Parameters:**
- `path` (string, required) - Destination file path
- `content` (string, required) - Content to write

**Returns:** Success confirmation

**Validation:** 
- Path must be within allowed directories
- Atomic write (temp file → verify → rename)
- Parent directories auto-created

**When to Use:**
- Saving output data
- Creating new documents
- Writing logs or reports
- Storing processed results

---

### `edit_file(path, old_string, new_string)`
**Purpose:** Replace text in a file (find and replace).

**Parameters:**
- `path` (string, required) - File to edit
- `old_string` (string, required) - Text to find
- `new_string` (string, required) - Replacement text

**Returns:** Success confirmation with number of replacements

**When to Use:**
- Updating existing files
- Making targeted changes
- Refactoring code or documents

---

### `list_files(path)`
**Purpose:** List contents of a directory.

**Parameters:**
- `path` (string, required) - Directory path

**Returns:** List of files and subdirectories

**When to Use:**
- Exploring directory structure
- Finding files matching patterns
- Verifying what exists

---

### `verify_file_exists(path)`
**Purpose:** Check if a file exists and is readable.

**Parameters:**
- `path` (string, required) - File path to check

**Returns:** Boolean (true if exists and readable, false otherwise)

**When to Use:**
- Before claiming a task is complete
- Verifying output from other processes
- Checking if dependencies exist
- **CRITICAL:** Always use this to verify file creation before reporting success

---

## 3. Web & Research

### `web_search(query, search_depth, max_results, include_domains, exclude_domains)`
**Purpose:** Search the web using Tavily API for factual, sourced information.

**Parameters:**
- `query` (string, required) - Search query
- `search_depth` (string, optional) - `"basic"` (fast) or `"advanced"` (thorough). Default: `"basic"`
- `max_results` (integer, optional) - Number of results (1-20). Default: 5
- `include_domains` (array, optional) - Domains to include (e.g., `["github.com", "reddit.com"]`)
- `exclude_domains` (array, optional) - Domains to exclude

**Returns:** Structured search results with titles, URLs, content snippets, and relevance scores

**Requirements:** Requires `TAVILY_API_KEY` environment variable set in `.env` file

**When to Use:**
- Researching topics
- Finding current information
- Gathering sources
- Fact-checking

---

### `web_fetch(url)`
**Purpose:** Fetch content from a specific URL.

**Parameters:**
- `url` (string, required) - URL to fetch

**Returns:** Page content as text

**When to Use:**
- Reading specific web pages
- Fetching documentation
- Downloading content from known URLs

---

## 4. System Execution

### `bash(command, timeout)`
**Purpose:** Execute shell commands.

**Parameters:**
- `command` (string, required) - Shell command to execute
- `timeout` (integer, optional) - Maximum seconds to wait. Default: 60

**Returns:** Command output (stdout and stderr)

**Validation:** Commands run with restricted permissions. Cannot access sensitive system files.

**When to Use:**
- Running system commands
- Executing scripts
- System administration tasks

---

## 5. Pattern & Memory

### `read_patterns()`
**Purpose:** Load learned patterns from storage.

**Returns:** JSON object with all pattern categories, observations, and confidence scores

**When to Use:**
- Before making suggestions based on user history
- Understanding user behavior patterns
- Building context for recommendations

---

### `add_observation(pattern_id, observation)`
**Purpose:** Add an observation to a pattern category.

**Parameters:**
- `pattern_id` (string, required) - Pattern identifier (e.g., "focus", "energy", "procrastination")
- `observation` (string, required) - The observation to record

**Returns:** Success confirmation

**When to Use:**
- Recording user behavior
- Learning patterns over time
- Building user profile

---

## 6. Task & Goal Management

### `read_tracking()`
**Purpose:** Load all goals, projects, tasks, and reminders.

**Returns:** Complete tracking data structure

**When to Use:**
- Checking active goals
- Finding tasks to work on
- Reviewing reminders

---

### `create_goal(title, description, target_date)`
**Purpose:** Create a new goal.

**Parameters:**
- `title` (string, required) - Goal name
- `description` (string, optional) - Detailed description
- `target_date` (string, optional) - Target completion date (ISO format)

**Returns:** Goal ID

**When to Use:**
- Setting new objectives
- Defining long-term targets

---

### `create_task(title, project_id, due_date)`
**Purpose:** Create a new task.

**Parameters:**
- `title` (string, required) - Task description
- `project_id` (string, optional) - Parent project
- `due_date` (string, optional) - Due date (ISO format)

**Returns:** Task ID

**When to Use:**
- Adding actionable items
- Breaking down projects

---

### `update_status(item_id, new_status)`
**Purpose:** Update status of a goal, project, or task.

**Parameters:**
- `item_id` (string, required) - Item identifier
- `new_status` (string, required) - `"todo"`, `"in_progress"`, `"completed"`, or `"archived"`

**Returns:** Success confirmation

**When to Use:**
- Marking items complete
- Updating progress

---

## 7. Scheduling

### `schedule_task(message, schedule_time, recurring)`
**Purpose:** Schedule a reminder or task.

**Parameters:**
- `message` (string, required) - Reminder text
- `schedule_time` (string, required) - When to trigger (natural language: "tomorrow 3pm", "in 5 minutes")
- `recurring` (string, optional) - Cron expression for recurring reminders

**Returns:** Task ID

**When to Use:**
- Setting reminders
- Scheduling future tasks
- Creating recurring habits

---

## 8. Skill Management

### `create_skill(skill_name, description, content)`
**Purpose:** Create a custom skill for reusable workflows.

**Parameters:**
- `skill_name` (string, required) - Skill identifier (lowercase-hyphens)
- `description` (string, required) - What the skill does
- `content` (string, required) - Full SKILL.md content

**Returns:** Success confirmation with path

**When to Use:**
- Creating reusable procedures
- Documenting workflows
- Building custom capabilities

---

### `activate_skill(skill_name)`
**Purpose:** Load and use a skill.

**Parameters:**
- `skill_name` (string, required) - Name of skill to activate

**Returns:** Skill content loaded into context

**When to Use:**
- Accessing stored workflows
- Following documented procedures

---

## Tool Usage Rules

### 1. Always Verify File Operations
**Before claiming a task is done, verify the output file exists:**
```
write_file("/storage/results.txt", data)
if verify_file_exists("/storage/results.txt"):
    return "Success: File created"
else:
    return "Error: File creation failed"
```

### 2. Path Validation
All file operations are restricted to:
- `/storage/`
- `/mnt/storage/`
- `/tmp/`
- Configured working directory

Attempts to access other paths will fail.

### 3. Error Handling
- Always check return values
- Report actual errors, not assumptions
- If a tool fails, stop and report - don't proceed with invalid state

### 4. No Hallucination
**NEVER claim a tool succeeded without calling it and verifying the result.**

Bad: "I've saved the file" (without calling write_file)
Good: Calls write_file → verifies with verify_file_exists → reports actual status

---

## Environment Requirements

**Required Environment Variables:**
- `TAVILY_API_KEY` - For web_search tool
- `TELEGRAM_BOT_TOKEN` - For Telegram integration
- `OPENROUTER_API_KEY` - For LLM access

**Storage Locations:**
- User data: `/mnt/storage/`
- System data: `/storage/` (or `./storage/` in project root)
- Working files: `./tmp/` or configured working_dir

---

**Note:** This is a living document. As tools are added or modified, this file must be updated.
