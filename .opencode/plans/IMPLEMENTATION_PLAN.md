# SANDY SYSTEM REBUILD: IMPLEMENTATION PLAN

## Executive Summary

**Objective:** Rebuild Sandy from a prototype with placeholder code into a production-ready "Hard Rails" system following the Hybrid Brain/Hands architecture.

**Current State:** Infrastructure-heavy, execution-light. Fake agent system, working sub_agent tool, solid file operations.

**Target State:** Deterministic execution via Python scripts, strict separation of Brain (LLM reasoning) and Hands (code execution), file system as single source of truth.

---

## Phase 0: Analysis & Audit ✅ COMPLETED

**Status:** FINISHED - Truthful inventory created, false claims removed from documentation

### Step 0.1: Document Current System State ✅ DONE
**Files reviewed:**
- ✅ `src/tools/agent_management.rs` - Confirmed fake spawn_agent at lines 132-133
- ✅ `src/tools/sub_agent.rs` - Verified working sub-agent system
- ✅ `soul/AGENTS.md` - Documented false claims about agent system
- ✅ `PROJECT.md` - Documented false capabilities
- ✅ `CHECKLIST.md` - Documented Phase 7 marked as "COMPLETED" falsely

**Deliverable:** `.opencode/plans/PHASE0_AUDIT.md` - Truthful inventory of what's real vs placeholder

### Step 0.2: Remove False Claims from Documentation ✅ DONE
**Actions completed:**
- ✅ **soul/AGENTS.md:** Replaced "Agent Delegation System" section with warning that it's "NOT YET IMPLEMENTED"
- ✅ **PROJECT.md:** Removed false claims about spawn_agent capabilities
- ✅ **PROJECT.md:** Added "What's Placeholder / In Development" section
- ✅ **PROJECT.md:** Updated "Completed Recently" to show agent system is in development
- ✅ **CHECKLIST.md:** Changed Phase 7 from "✅ COMPLETED" to "⚠️ INFRASTRUCTURE ONLY - NO EXECUTION"
- ✅ **CHECKLIST.md:** Added detailed explanation of the problem and rebuild plan
- ✅ **src/tools/agent_management.rs:** Added file header warning about placeholder status
- ✅ **src/tools/agent_management.rs:** Replaced placeholder comment with explicit WARNING

### Summary of Phase 0:
**CRITICAL ISSUE IDENTIFIED:** spawn_agent tool creates registry entries but NEVER executes actual work. All documentation claiming otherwise has been corrected.

**Working Components (9/10):**
- ✅ File operations
- ✅ Web search/fetch
- ✅ SubAgentTool (real execution)
- ✅ Pattern learning
- ✅ Tracking system
- ✅ Scheduler
- ✅ Activity logging
- ✅ Skill builder
- ✅ Self-review

**Placeholder/Broken (1/10):**
- ❌ Agent Management System - Infrastructure only, no execution

**Next:** Proceed to Phase 1 (Foundation - Storage & Core Files)

---

## Phase 1: Foundation - Storage & Core Files (2-3 hours)

### Step 1.1: Create Storage Directory Structure
```
/storage/                    # NEW: The Truth (persistent storage)
├── agents/                  # Agent JSON configurations
│   ├── zilla.json          # Example: Research agent config
│   └── gonza.json          # Example: Writer agent config
├── tasks/                   # Job workspaces
│   └── job_001/            # Each job gets unique folder
│       ├── instructions.md  # Task description
│       ├── raw_data.md     # Agent output
│       └── final_output.md # Final result
└── memory/                  # Long-term data
    ├── projects/           # Project data
    ├── todos/             # Task lists
    └── logs/              # Execution logs
```

**Action:** 
- Create directory structure
- Add to .gitignore if needed (or commit empty dirs with .gitkeep)
- Update PROJECT.md to document storage layout

### Step 1.2: Create TOOLS.md (The Constitution)
**Location:** `/Users/jenslennartsson/Documents/-ai_projects-/SandyNew/TOOLS.md`

**Purpose:** Single source of truth for available tools. Sandy reads this to know what she can actually do.

**Content:**
```markdown
# Available Tools

## 1. Agent Management
- `sub_agent(task, context)` - Spawn background worker to complete task
- `create_agent_config(id, role, tools[])` - Create agent JSON config

## 2. File System
- `read_file(path)` - Read content from storage
- `write_file(path, content)` - Write content to storage
- `list_files(path)` - List directory contents
- `verify_file_exists(path)` - Check if file exists (verification)

## 3. Web & Research
- `web_search(query)` - Search web via DuckDuckGo
- `web_fetch(url)` - Fetch content from URL

## 4. Execution
- `bash(command)` - Execute shell command
- `python_script(path, args[])` - Run Python script with arguments
```

### Step 1.3: Create prompts/guard_rails.txt
**Location:** `/Users/jenslennartsson/Documents/-ai_projects-/SandyNew/prompts/guard_rails.txt`

**Purpose:** DNA injected into every spawned agent to prevent drift.

**Content:**
```
*** CRITICAL SAFETY PROTOCOLS ***
1. YOU ARE RESTRICTED: You are a specialized sub-process.
2. FILE SYSTEM ONLY: You perform your task and save the result to the provided file path.
3. NO CHIT-CHAT: Do not converse. Output only the requested data or the error message.
4. SOURCE OF TRUTH: If you cannot read the input file, stop and report error.
5. TOOL ONLY: You cannot browse web or access files directly. Use provided tools only.
6. OUTPUT REQUIRED: You MUST save your work to the specified output file before completing.
```

---

## Phase 2: Core Infrastructure - File Operations (3-4 hours)

### Step 2.1: Create src/tools/file_ops.rs
**Purpose:** Safe file operations with verification

**Functions needed:**
- `read_file(path) -> Result<String, Error>` - Read with error handling
- `write_file(path, content) -> Result<(), Error>` - Write with parent dir creation
- `verify_file_exists(path) -> bool` - Check existence
- `list_directory(path) -> Result<Vec<String>, Error>` - List contents
- `create_job_folder(job_id) -> Result<PathBuf, Error>` - Create task workspace

**Key features:**
- Path validation (prevent directory traversal)
- Automatic parent directory creation
- Error messages written to output files (per architecture.md pattern)
- Atomic writes (write to temp, rename on success)

### Step 2.2: Integrate file_ops into ToolRegistry
**Location:** `src/tools/mod.rs`

**Action:**
- Add `pub mod file_ops;`
- Register tools in registry
- Ensure path guards protect sensitive directories

### Step 2.3: Test File Operations End-to-End
**Test cases:**
1. Write file → Verify exists → Read back → Confirm content
2. Create nested directory structure → Write files at multiple levels
3. Attempt path traversal attack → Verify blocked
4. Write large file → Verify atomic (no partial files on crash)

---

## Phase 3: The Hands - Skill Scripts (4-5 hours)

### Step 3.1: Create Journalistic Research Skill

**Directory:** `src/skills/journalistic-research/`

**Files:**
1. **SKILL.md** - Metadata and usage instructions
2. **scripts/run_research.py** - Python script for web research

**run_research.py requirements:**
- Accepts: query (str), output_path (str)
- Uses DuckDuckGo API (existing web_search tool calls)
- Saves structured results to output_path
- Handles errors gracefully (writes ERROR: message to file)
- Returns: "SUCCESS: Data saved to {path}"

**Content outline:**
```python
#!/usr/bin/env python3
import sys
import json
import subprocess

def main():
    if len(sys.argv) < 3:
        print("Usage: python run_research.py <query> <output_path>")
        sys.exit(1)
    
    query = sys.argv[1]
    output_path = sys.argv[2]
    
    try:
        # Call Sandy's web_search via CLI or API
        results = perform_search(query)
        
        # Structure the output
        output = {
            "query": query,
            "results": results,
            "timestamp": get_timestamp()
        }
        
        # Write to file
        with open(output_path, 'w') as f:
            json.dump(output, f, indent=2)
        
        print(f"SUCCESS: Data saved to {output_path}")
        
    except Exception as e:
        with open(output_path, 'w') as f:
            f.write(f"ERROR: {str(e)}")
        print(f"ERROR: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    main()
```

### Step 3.2: Create Journalistic Writing Skill

**Directory:** `src/skills/journalistic-writing/`

**Files:**
1. **SKILL.md** - Metadata
2. **scripts/write_article.py** - Transform raw data to article

**write_article.py requirements:**
- Accepts: input_path (str), output_path (str)
- Reads raw data from input_path
- Calls LLM (via sub_agent or direct API) to format/transform
- Writes formatted article to output_path
- Has NO web access (enforced by not importing web tools)

### Step 3.3: Create Agent Factory

**File:** `src/tools/agent_factory.rs`

**Purpose:** Create agent configurations safely

**Function:** `create_agent_config(agent_id, role, allowed_tools) -> Result<PathBuf, Error>`

**Validation:**
- Filter tools against whitelist: ["web_search", "web_fetch", "read_file", "write_file", "bash"]
- Reject invalid tools (anti-hallucination)
- Write JSON to `storage/agents/{agent_id}.json`

**Example output:**
```json
{
  "id": "zilla",
  "role": "Journalistic Researcher",
  "tools": ["web_search", "web_fetch", "write_file"],
  "created_at": "2026-02-12T10:00:00Z",
  "note": "This agent CANNOT write articles. It can only find facts."
}
```

### Step 3.4: Test Skill Scripts

**Test workflow:**
1. Create test job folder: `storage/tasks/job_test/`
2. Run: `python3 src/skills/journalistic-research/scripts/run_research.py "AI news" storage/tasks/job_test/raw.json`
3. Verify: raw.json exists with search results
4. Run: `python3 src/skills/journalistic-writing/scripts/write_article.py storage/tasks/job_test/raw.json storage/tasks/job_test/article.md`
5. Verify: article.md exists with formatted content

---

## Phase 4: Agent Execution System (6-8 hours)

### Step 4.1: Replace spawn_agent with Real Implementation

**Current problem:** `spawn_agent` in agent_management.rs only creates registry entries

**Solution:** Rebuild using `sub_agent` as the execution engine

**New architecture:**

```rust
pub struct SpawnAgentTool {
    config: Config,
}

#[async_trait]
impl Tool for SpawnAgentTool {
    async fn execute(&self, input: Value) -> ToolResult {
        // 1. Load agent config from storage/agents/{agent_id}.json
        // 2. Create job folder: storage/tasks/{job_id}/
        // 3. Write instructions.md with task details
        // 4. Build system prompt: agent role + guard_rails.txt
        // 5. Call sub_agent with restricted tool set
        // 6. Monitor execution (blocking or async with polling)
        // 7. Verify output file exists
        // 8. Return result or error
    }
}
```

**Key differences from old spawn_agent:**
- Actually executes work (via sub_agent)
- Creates job folder structure
- Verifies output before returning success
- Uses agent config to restrict tools (anti-hallucination)

### Step 4.2: Implement Sequential Workflow Support

**Requirement:** Zilla → (verify) → Gonza workflow

**New tool:** `execute_workflow(steps: Vec<WorkflowStep>)`

**WorkflowStep structure:**
```rust
struct WorkflowStep {
    agent_id: String,
    task: String,
    input_file: Option<PathBuf>,      // Read this before execution
    output_file: PathBuf,              // Must create this
    verify_output: bool,               // Check file exists and not empty
}
```

**Execution logic:**
1. For each step:
   - Read input_file if specified
   - Spawn agent with task + context
   - Wait for completion (blocking or polling)
   - Verify output_file exists
   - Verify output_file not empty
   - If verification fails, return error (stop workflow)
2. If all steps complete, return success with output paths

### Step 4.3: Add Agent Reporting/Toggle System

**New tool:** `set_agent_reporting(agent_id, enabled)`

**Behavior:**
- When enabled: Agent sends progress messages via Telegram bot
- When disabled: Agent works silently, only final result reported
- Store preference in agent config JSON

**Implementation:**
- Pass Telegram bot reference to SpawnAgentTool
- If reporting enabled, agent calls `send_message` tool periodically
- Or: Sandy polls agent status and relays updates

### Step 4.4: Integration Testing

**Test case: Research & Write Workflow**

**Input:** "Research AI news and write a summary"

**Expected execution:**
1. Sandy creates job folder: `storage/tasks/job_001/`
2. Sandy spawns Zilla: `sub_agent(task="Search for AI news", context=guard_rails)`
3. Zilla executes: Calls web_search, saves results to `job_001/raw_data.json`
4. Sandy verifies: `raw_data.json` exists and size > 0
5. Sandy spawns Gonza: `sub_agent(task="Write article from raw_data.json", input_file=job_001/raw_data.json)`
6. Gonza executes: Reads raw_data, formats article, saves to `job_001/article.md`
7. Sandy verifies: `article.md` exists
8. Sandy reports to user: "Done. Article saved at storage/tasks/job_001/article.md"

**Failure scenarios to test:**
- Zilla fails (web search error) → Workflow stops, error reported
- Zilla succeeds but output empty → Verification fails, error reported
- Gonza succeeds but output empty → Verification fails, error reported
- File permissions error → Error written to output file, reported to user

---

## Phase 5: Update Documentation (2-3 hours)

### Step 5.1: Rewrite AGENTS.md

**Current issue:** Claims agent system works (false)

**New structure:**
```markdown
# Sandy - System Capabilities

## Working Features (Verified)
1. File Management - read/write/verify via tools
2. Web Search - DuckDuckGo integration
3. Sub-Agent Execution - background task delegation
4. Sequential Workflows - multi-step agent chains
5. Pattern Learning - manual observation recording
6. Task Tracking - Goals/Projects/Tasks/Reminders

## Hybrid Model Implementation
- Brain: LLM orchestration via Sandy
- Hands: Python scripts in src/skills/*/scripts/
- Truth: File system in storage/

## Agent System (Working)
- Agent configs stored in storage/agents/
- Spawn via sub_agent tool with restricted capabilities
- Verification required before reporting success
- Sequential workflows supported (Agent A → Agent B)
```

### Step 5.2: Update PROJECT.md

**Changes:**
- Remove "Agent delegation" from "What's Working" until Phase 4 complete
- Add "Current Implementation Status" section with honest percentages
- Mark agent system as "[IN PROGRESS - Phase 4]" until tested

### Step 5.3: Update CHECKLIST.md

**New Phase 8:** Agent System Rebuild (0% → 100%)
- [ ] Step 8.1: Remove fake spawn_agent (COMPLETED)
- [ ] Step 8.2: Create storage/ directory structure
- [ ] Step 8.3: Create TOOLS.md and guard_rails.txt
- [ ] Step 8.4: Build file_ops.rs with verification
- [ ] Step 8.5: Create research skill with Python script
- [ ] Step 8.6: Create writing skill with Python script
- [ ] Step 8.7: Rebuild spawn_agent with real execution
- [ ] Step 8.8: Implement sequential workflow support
- [ ] Step 8.9: Add agent reporting toggle
- [ ] Step 8.10: End-to-end testing (Zilla → Gonza)

---

## Phase 6: Safety & Guardrails (2-3 hours)

### Step 6.1: Path Traversal Protection

**Current:** path_guard.rs exists but verify it's applied everywhere

**Required:**
- All file operations validate paths
- Prevent `../../../etc/passwd` attacks
- Only allow operations within allowed directories:
  - `/mnt/storage/` (user files)
  - `/storage/` (new system)
  - `/tmp/` or configured working_dir

### Step 6.2: Tool Whitelisting

**Implementation:**
- Agent configs specify allowed_tools: ["web_search", "write_file"]
- SpawnAgentTool filters tool registry to only include allowed tools
- Attempt to use non-allowed tool → Error: "Tool X not available to this agent"

### Step 6.3: Error Handling Standards

**Pattern from architecture.md:**
- Errors written to output file (not just logged)
- Format: `ERROR: {message}`
- Exit code non-zero
- Sandy reads error from file and reports to user

**Example:**
```python
try:
    result = do_work()
    write_output(result)
    print("SUCCESS: ...")
except Exception as e:
    write_output(f"ERROR: {str(e)}")
    print(f"ERROR: {str(e)}")
    sys.exit(1)
```

---

## Phase 7: Cleanup & Removal (1-2 hours)

### Step 7.1: Remove Fake Agent Code

**Files to modify:**
1. **src/tools/agent_management.rs**
   - Keep: `SpawnAgentTool` structure (reuse for real implementation)
   - Remove: Comment "In a full implementation..."
   - Rewrite: `execute()` to actually spawn work
   - Keep: `AgentInfo`, `AgentStatus` for tracking (valid infrastructure)

2. **src/tools/mod.rs**
   - Keep: `agent_management` module
   - Remove: Any tool registrations that are placeholder-only

3. **KEEP but mark:**
   - `list_agents` - Useful for monitoring (returns real status)
   - `agent_status` - Useful for checking progress
   - `set_agent_reporting` - Toggle actually works once spawn_agent does

### Step 7.2: Update Help Command

**Location:** `src/telegram.rs` get_help_text()

**Change:**
- Remove: "🤖 **Agent Delegation**..." section (until Phase 4 proven)
- Add: "🤖 **Agent System** [In Development] - Sequential multi-agent workflows coming soon"
- Or: Keep description but add "(Beta - Testing in progress)"

### Step 7.3: Archive Old Documentation

**Move to /old/:**
- Any docs describing fake agent system as "working"
- Old PROJECT.md versions with false claims
- Keep: Current versions (will be updated in Phase 5)

---

## Implementation Order & Dependencies

### Recommended Sequence:

**Week 1: Foundation (Phases 0-2)**
- Day 1: Audit current system, document truth
- Day 2: Create storage/ structure, TOOLS.md, guard_rails.txt
- Day 3: Build file_ops.rs, test file operations

**Week 2: The Hands (Phase 3)**
- Day 4: Create research skill + Python script
- Day 5: Create writing skill + Python script
- Day 6: Build agent_factory.rs, test end-to-end

**Week 3: Execution (Phase 4)**
- Day 7-8: Rebuild spawn_agent with real execution
- Day 9: Implement sequential workflows
- Day 10: Add reporting toggle, integration testing

**Week 4: Polish (Phases 5-7)**
- Day 11-12: Update all documentation
- Day 13: Safety review, path traversal tests
- Day 14: Cleanup, final verification

### Critical Path:
1. File operations must work before skills can write output
2. Skills must work before spawn_agent can use them
3. spawn_agent must work before sequential workflows can execute
4. Everything must be tested individually before integration

---

## Success Criteria

**Definition of DONE for this rebuild:**

✅ User can say: "Research AI news and write a summary"
✅ Sandy creates job folder with unique ID
✅ Sandy spawns research agent that actually searches web
✅ Research agent writes real results to file
✅ Sandy verifies file exists and is not empty
✅ Sandy spawns writer agent with input file
✅ Writer agent reads input, formats output, writes article
✅ Sandy verifies final output exists
✅ Sandy reports: "Done. Article at storage/tasks/job_XXX/article.md"
✅ If any step fails, Sandy reports exact error and stops (no fake success)

**Verification command for user:**
```bash
# After implementation, test with:
ls -la storage/tasks/job_*/
cat storage/tasks/job_*/raw_data.json
cat storage/tasks/job_*/article.md
```

All files should exist with real content, not placeholders.

---

## Questions for User

Before starting implementation, clarify:

1. **Python dependency:** Do you have Python 3.x installed on the Pi? (Required for skill scripts)
2. **Tavily API:** Do you have a Tavily API key, or should we use existing DuckDuckGo web_search? (Tavily mentioned in architecture.md but not required)
3. **Storage location:** Use `/mnt/storage/` (existing, Mac-accessible) or create new `./storage/` in project root? (Recommend `/mnt/storage/` for Mac access)
4. **Parallel vs Sequential:** Do you want agents to run in parallel (faster, complex) or strictly sequential (simpler, easier to debug)? (Recommend sequential for reliability)
5. **Reporting:** Real-time progress messages in Telegram, or just final result? (Recommend final result only to start, add real-time later)

**Ready to proceed with Phase 0 (Audit)?**