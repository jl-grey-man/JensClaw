# Sandy Development Checklist

> ⚠️ **IMPORTANT:** Update this document at the end of each major task. Mark completed items with [x] and add new items at the bottom.

 

----- START OF EXAMPLE (**DO NOT ALTER**)


## Phase 7: Agent Delegation System [~] IN PROGRESS

### Infrastructure (Complete):
- [x] Create spawn_agent tool (registers agents)
- [x] Create list_agents tool (shows registry)
- [x] Create set_agent_reporting tool (toggle status)
- [x] Add agent status tracking

### Execution (NOT STARTED):
- [ ] Implement actual agent execution engine
  - Use tokio::spawn() for background tasks
  - Give agents real tool access
  - Execute LLM calls with specialized prompts
- [ ] Build sequential workflow support
  - Job folder creation
  - Output verification between steps
  - Agent B waits for Agent A completion
- [ ] Add skill-based tool restrictions
  - Zilla: web tools only
  - Gonza: file tools only
- [ ] Test end-to-end with real agents
  - Agent performs work
  - Writes real files
  - Returns actual results

Status: 30% complete (infrastructure done, execution pending)


____ END OF EXAMPLE




MAIN CHECKLIST.

## Legend
- [x] **Completed** - Feature working in production
- [~] **In Progress** - Currently being implemented
- [ ] **Pending** - Not yet started
- [!] **Blocked** - Waiting on dependency or decision

---

## Phase 1: Foundation ✅ COMPLETED

### Core System Setup
- [x] Clone and setup JensClaw (MicroClaw fork)
- [x] Create OpenClaw file structure (SOUL.md, AGENTS.md, IDENTITY.md)
- [x] Configure OpenRouter + Claude 3.5 Sonnet integration
- [x] Setup Telegram bot connection
- [x] Create data directory structure
- [x] Implement activity logging system
- [x] Build release version

### Web UI - Phase 1
- [x] Create basic web server (Axum)
- [x] Build dashboard HTML/CSS/JS
- [x] Add stats cards (goals, tasks counters)
- [x] Add dropdown lists (Goals, Projects, Tasks, Patterns)
- [x] Implement activity log feed
- [x] Add 5-second auto-refresh
- [x] Make items clickable with modal details

### Pattern System - Phase 1
- [x] Create 18 initial ADHD pattern categories
- [x] Implement pattern JSON schema
- [x] Build `read_patterns` tool
- [x] Build `add_observation` tool
- [x] Build `update_hypothesis` tool
- [x] Build `create_pattern` tool (dynamic creation)
- [x] Add confidence tracking (0-100%)

### Tracking System - Phase 1
- [x] Create unified tracking schema (Goals/Projects/Tasks)
- [x] Build `create_goal` tool
- [x] Build `create_project` tool
- [x] Build `create_task` tool
- [x] Build `update_status` tool
- [x] Build `read_tracking` tool
- [x] Implement hierarchy (Goals → Projects → Tasks)

### Note System
- [x] Add `notes` field to Goal, Project, Task structs
- [x] Build `add_note` tool (auto-timestamped)
- [x] Build `remove_note` tool (by index or clear all)
- [x] Display notes in Web UI with preview
- [x] Show full notes in item detail modal

---

## Phase 2: Reminders & Scheduling ✅ COMPLETED

### Reminder Core
- [x] Implement `schedule_task` tool
- [x] Build scheduler system (60-second polling)
- [x] Create database schema for scheduled tasks
- [x] Implement `list_scheduled_tasks` tool
- [x] Implement `cancel_task` tool
- [x] Link reminders to tracking.json for Web UI display

### Flexible Time Parsing - Iteration 1
- [x] Parse "in X minutes/hours/days"
- [x] Parse "tomorrow at HH:MM"
- [x] Parse "today at HH:MM"
- [x] Convert all timestamps to UTC
- [x] Fix timezone mismatch bugs

### Flexible Time Parsing - Iteration 2 (Enhanced)
- [x] Support abbreviated units: "3m", "5min", "1h", "2hr"
- [x] Parse day names: "Monday", "Tuesday", etc. (next occurrence)
- [x] Add time keywords: "morning" (9am), "afternoon" (2pm), "evening" (6pm), "tonight" (8pm), "noon", "midnight"
- [x] Handle variations: "in 3 min" anywhere in sentence (not just at start)
- [x] Smart defaults: Day names default to 9am if no time specified

### Activity Log Integration
- [x] Log reminder creation to activity feed
- [x] Show reminder creation in Web UI live feed
- [x] Fix activity log path issues (runtime/ vs data/)

### Error Handling & UX
- [x] Add strict validation - reject unclear time references
- [x] Update tool description to guide LLM behavior
- [x] Make Sandy ask for clarification on vague times ("later", "soon")
- [x] Add helpful error messages with supported formats

---

## Phase 3: Bug Fixes ✅ COMPLETED

### Critical Bugs
- [x] **Fix:** Activity log not updating in Web UI
  - Root cause: ActivityLogger cached entries in memory
  - Solution: Re-read file fresh on every request

  
- [x] **Fix:** Web UI path mismatch for activity log
  - Root cause: Different directories (data/ vs runtime/)
  - Solution: Always use runtime_data_dir consistently

### Minor Fixes
- [x] Add debug logging to scheduler
- [x] Fix borrow checker issues in RemoveNoteTool
- [x] Make tracking functions public (read_tracking, write_tracking)
- [x] Add Datelike import for date parsing

- [x] **Fix:** Reminders firing immediately (wrong year: 2024)
  - Root cause: System prompt didn't include current date/time and instructed LLM to compute ISO timestamps itself. LLM used its training cutoff year (2024) instead of the real date.
  - Solution: (1) Inject `current_time` (UTC) into system prompt so LLM knows the real date. (2) Changed scheduling instructions to tell LLM to pass natural language time expressions (e.g., "in 5 minutes") directly as `schedule_value`, letting server-side `parse_natural_to_iso()` handle the conversion with `chrono::Local::now()`.


---

## Phase 4: Self-Review System ✅ COMPLETED

### Daily Self-Analysis
- [x] **Task 4.1:** Build automatic daily analysis (3 AM)
  - Analyze conversation quality and helpfulness
  - Review goal and project support effectiveness
  - Check pattern recognition accuracy
  - Evaluate tool usage appropriateness
  
- [x] **Task 4.2:** Create Review Mode workflow
  - Present findings as suggestions to user
  - Require explicit approval ("yes") for changes
  - Never make autonomous modifications
  - Log all proposals for transparency
  
- [x] **Task 4.3:** Implement configuration change suggestions
  - Propose updates to SOUL.md (personality)
  - Propose updates to AGENTS.md (capabilities)
  - Track approved vs rejected changes
  
- [x] **Task 4.4:** Safety guardrails
  - No autonomous changes without approval
  - No code modifications (Rust source protected)
  - No memory deletion without consent
  - Full transparency before any action

**Status:** ✅ Implemented via `soul/data/skills/sandy-evolver/SKILL.md`
**Result:** Self-review system operational, all changes require user approval

---

## Phase 5: Automatic Note Addition 🔄 SUPERCEDED

Replaced by Self-Review System (Phase 4). Pattern and context learning now happens through daily self-review with user approval rather than fully automatic detection. This gives user control over what Sandy learns.

---

## Phase 5: Document Management System ✅ COMPLETED

### File Operations
- [x] **Task 5.1:** Create `soul/data/skills/documents/SKILL.md`
  - Instructions for creating Markdown, Text, HTML files
  - Guidance for CSS, JavaScript, JSON, Python files
  - File organization best practices
  
- [x] **Task 5.2:** File creation workflow
  - Create notes: "Create a note about my project ideas"
  - Create code files: "Write a Python script"
  - Create web pages: "Build me a simple website"
  - Store in `/mnt/storage/` (accessible from Mac)
  
- [x] **Task 5.3:** Directory organization
  - Recommended structure: notes/, projects/, scripts/, lists/
  - Guidance for naming conventions
  - Tips for shared Mac access
  
- [x] **Task 5.4:** Update AGENTS.md
  - Document document management capability
  - Add to system capabilities list
  - Include in HELP command

**Status:** ✅ Fully operational
**Location:** Files in `/mnt/storage/` accessible from Mac
**Skills:** Document management workflow documented in AGENTS.md

---

## Phase 6: Skill Builder System ✅ COMPLETED

### Skill Creation Infrastructure
- [x] **Task 6.1:** Create `soul/data/skills/sandy-skill-builder/SKILL.md`
  - Skill structure documentation
  - SKILL.md format guide
  - Examples: morning routine, research assistant, file organizer
  
- [x] **Task 6.2:** Build `create_skill` tool
  - Tool definition and implementation
  - Input: skill_name, description, content
  - Output: SKILL.md file in `soul/data/skills/custom/`
  - Validate skill_name format (lowercase-hyphens)
  
- [x] **Task 6.3:** Skill storage and organization
  - Store in `soul/data/skills/custom/{skill-name}/SKILL.md`
  - Separate from builtin skills
  - Easy to list, edit, delete
  
- [x] **Task 6.4:** Test skill activation
  - "Use my [skill-name] skill" loads and activates
  - Sandy follows skill instructions
  - Multiple skills can coexist
  
- [x] **Task 6.5:** Documentation updates
  - Update AGENTS.md with Skill Builder section
  - Add to HELP command
  - Include examples of useful skills

**Status:** ✅ Fully operational
**First skills created:** research-assistant (built-in example)
**Tool registered:** `create_skill` in tools/mod.rs

---

## Phase 7: Agent System ⚠️ INFRASTRUCTURE ONLY - NO EXECUTION

### ⚠️ CRITICAL ISSUE: Placeholder Code

**Status:** Infrastructure exists but **DOES NOT EXECUTE ACTUAL WORK**

### What Exists (Infrastructure Only):
- [x] **Registry system:** Tools created (`spawn_agent`, `list_agents`, etc.)
- [x] **Tracking:** Global registry using lazy_static
- [x] **State monitoring:** Agent status, timestamps, reporting flags
- [x] **Documentation:** AGENTS.md section, HELP command

### What's Missing (Execution):
- [ ] **Real task execution:** spawn_agent only creates registry entries
- [ ] **Agent types:** No actual Research/Code/File agents that execute
- [ ] **Verification:** No checking if agents produced output
- [ ] **Sequential workflows:** No Zilla → verify → Gonza capability
- [ ] **Tool restrictions:** Agents can access all tools (not restricted)

### The Problem:
**File:** `src/tools/agent_management.rs` lines 132-133
```rust
// Note: In a full implementation, this would actually spawn a background task
// For now, we register the agent and simulate the async nature
```

**What this means:**
- spawn_agent creates a registry entry but NEVER executes the task
- Sandy may claim "agent is working" but nothing happens
- Violates AI-RULES.md "NO STUB" and "Definition of DONE"
- User time wasted testing non-functional feature

### The Fix:
See IMPLEMENTATION_PLAN.md **Phase 4: Real Agent Execution** for complete rebuild plan.

**Required:**
- [ ] Rebuild spawn_agent using sub_agent as execution engine
- [ ] Create Python skill scripts (The Hands)
- [ ] Implement job folder system (storage/tasks/job_XXX/)
- [ ] Add verification after each step (file exists + size > 0)
- [ ] Implement sequential workflows (Agent A → Verify → Agent B)

**Estimated:** 6-8 hours (Phases 1-4)

### What Works NOW:
The `sub_agent` tool DOES work - it spawns real LLM subprocesses. Use this for background tasks until the full agent system is rebuilt.

**Status:** ❌ **NOT EXECUTABLE** - Rebuild required per IMPLEMENTATION_PLAN.md

---

## Phase 8: Proactive Sandy ⏳ PENDING

### Proactive Check-ins
- [ ] **Task 6.1:** Daily/weekly check-in reminders
  - "How's your energy today?"
  - Pattern-based: "You usually crash around 2pm - want a break?"
  
- [ ] **Task 6.2:** Task deadline reminders
  - "Your Website project is due tomorrow - want to work on it?"
  - Based on due dates and learned patterns
  
- [ ] **Task 6.3:** Pattern-based suggestions
  - "You have high energy this morning - perfect time for that difficult task"
  - Use learned energy/focus patterns
  
- [ ] **Task 6.4:** Unprompted accountability
  - "You said you'd exercise 3x this week - how's that going?"
  - Gentle, non-judgmental check-ins

**Priority:** Medium
**Estimated Time:** 4-5 days
**Dependencies:** Phase 4 (needs learned patterns)

---

## Phase 7: Web UI Enhancements ⏳ PENDING

### Editing Capabilities
- [ ] **Task 7.1:** Edit items directly in Web UI
  - Modify task titles, due dates
  - Change project/goal associations
  - Update notes
  
- [ ] **Task 7.2:** Create items from Web UI
  - Add new task without Telegram
  - Quick-add with simple form
  
- [ ] **Task 7.3:** Dark mode toggle
  - CSS dark theme
  - Persist preference

### Calendar Integration
- [ ] **Task 7.4:** Calendar view for deadlines
  - Monthly/weekly calendar
  - Show tasks/projects on calendar
  - Visual overview of workload
  
- [ ] **Task 7.5:** Reminder calendar
  - See all upcoming reminders
  - Drag to reschedule?

**Priority:** Medium
**Estimated Time:** 3-4 days
**Dependencies:** None

---

## Phase 8: Multi-User Support ⏳ PENDING

### Architecture Changes
- [ ] **Task 8.1:** User isolation system
  - Separate data directories per user
  - User ID in all file paths
  
- [ ] **Task 8.2:** Separate pattern learning per user
  - patterns_{user_id}.json
  - Isolated pattern confidence/observations
  
- [ ] **Task 8.3:** Web UI user selection
  - Login/auth (simple token-based?)
  - Switch between users
  
- [ ] **Task 8.4:** Privacy considerations
  - Users can't see each other's data
  - Optional: Admin view

**Priority:** Low (single-user focus for now)
**Estimated Time:** 5-7 days
**Dependencies:** None

---

## Quick Fixes / Nice-to-Have

### Small Improvements
- [ ] Add loading states to Web UI buttons
- [ ] Export data to CSV/JSON
- [ ] Search/filter in Web UI lists
- [ ] Keyboard shortcuts for common actions
- [ ] Mobile app (PWA wrapper?)

### Documentation
- [x] Create PROJECT.md (comprehensive overview)
- [x] Create CHECKLIST.md (this file)
- [x] Archive old documentation to /old
- [ ] Add inline code documentation (rustdocs)
- [ ] Create user guide for Telegram commands

---

## Hard Rails Rebuild (New Architecture)

Implementation of the Brain/Hands architecture per architecture.md

### Phase 1: Foundation ✅ COMPLETED

**Goal:** Create storage structure and core files for Hard Rails architecture

**Completed:**
- [x] **Storage Directory Structure**
  - Created `storage/agents/` - Agent JSON configurations
  - Created `storage/tasks/` - Job workspaces
  - Created `storage/memory/` - Long-term data
  - Added .gitkeep files to preserve structure

- [x] **Core Documentation**
  - Created TOOLS.md - Definitive tool reference with 8 categories
  - Documented all parameters, validation rules, environment requirements
  - Created prompts/guard_rails.txt - Immutable safety DNA
  - Safety protocols: restricted scope, file-only output, no chit-chat, tool-only

- [x] **Agent Configurations**
  - Created storage/agents/zilla.json - Research agent
  - Created storage/agents/gonza.json - Writer agent
  - Defined tool restrictions and constraints for each

- [x] **Example Workflow**
  - Created storage/tasks/job_example/README.md
  - Documents Zilla → Verify → Gonza sequential flow
  - Shows verification checkpoints

- [x] **Documentation Updates**
  - Updated PROJECT.md with new storage/ structure
  - Added TOOLS.md and prompts/ to project tree

**Status:** Foundation ready for Phase 2
**Next:** Phase 2 - Hardened File Operations

### Phase 2: Hardened File Operations [~] IN PROGRESS

**Goal:** Bulletproof file operations with strict path validation

**Completed:**
- [x] **Create src/tools/file_ops.rs**
  - read_file() with path validation
  - write_file() with atomic writes (temp → verify → rename)
  - verify_file_exists() - Check before claiming success
  - list_directory() - Safe directory listing
  - create_job_folder() - Workspace creation
  - safe_join() - Safe path joining
  
- [x] **Hard-coded Path Guards**
  - Validate all paths against allowed roots: /storage, /mnt/storage, /tmp
  - Prevent directory traversal (../../../etc/passwd blocked)
  - Canonicalizes paths to resolve symlinks
  
- [x] **Atomic Write Implementation**
  - Write to temp file first (.tmp extension)
  - Verify content matches expected
  - Atomic rename on success
  - Cleans up temp file if verification fails
  - Prevents partial file corruption
  
- [x] **Integration with Existing Tools**
  - Updated read_file tool to use file_ops validation
  - Updated write_file tool to use file_ops atomic writes
  - Dual validation: path_guard (existing) + file_ops (Hard Rails)
  - Added mandatory verification after write

**In Progress / Remaining:**
- [~] Comprehensive integration testing
  - Test path traversal attacks (basic tests passing)
  - Test atomic writes under various conditions
  - Test concurrent write scenarios
  - Benchmark performance vs non-atomic writes

**Tests:**
- Path validation tests (allowed, blocked, traversal attacks)
- Safe path joining tests
- All security tests passing

**Status:** Core implementation complete, integrated into existing tools
**Next:** Phase 3 - Skill Scripts

### Phase 3: The Hands - Skill Scripts ⏳ PENDING

**Goal:** Create Python scripts that actually execute (not LLM hallucinations)

**Tasks:**
- [ ] Setup Python Virtual Environment
  - Create /storage/.venv/
  - Install dependencies: tavily, requests
  - All scripts run in venv isolation
  
- [ ] Create Journalistic Research Skill
  - src/skills/journalistic-research/SKILL.md
  - scripts/run_research.py
  - Takes: query, output_path
  - Uses Tavily API (real HTTP)
  - Saves structured JSON
  - Returns: "SUCCESS" or "ERROR"
  
- [ ] Create Journalistic Writing Skill
  - src/skills/journalistic-writing/SKILL.md
  - scripts/write_article.py
  - Takes: input_path, output_path
  - NO web access (reads file only)
  - Transforms data to article format
  - Saves formatted output
  
- [ ] Test End-to-End
  - Run research script → Verify output file exists
  - Run writing script → Verify article created
  - Both scripts must produce real files

**Dependencies:** Phase 2
**Next:** Phase 4 - Real Agent Execution

### Phase 4: Real Agent Execution ⏳ PENDING

**Goal:** Fix the broken agent system - make it actually execute work

**Critical Issue:** Current spawn_agent only creates registry entries, never executes

**Tasks:**
- [ ] Rebuild spawn_agent using sub_agent as engine
  - Load agent config from storage/agents/{id}.json
  - Create job folder: storage/tasks/{job_id}/
  - Call sub_agent with restricted tool set
  - Monitor execution (blocking, not async)
  - Verify output file exists before returning success
  
- [ ] Implement Sequential Workflow Support
  - New tool: execute_workflow(steps)
  - Zilla runs → VERIFY → Gonza runs → VERIFY
  - If verification fails: stop workflow, report error
  - No "I'll monitor" - actual blocking wait
  
- [ ] Verification After Every Step
  - Check: File exists?
  - Check: File size > 0?
  - Check: File readable?
  - Only proceed if all checks pass
  
- [ ] Integration Testing
  - Test: "Research AI news" → Zilla executes → File verified
  - Test: Zilla → Gonza workflow
  - Test: Verification failure handling
  - Test: Error propagation

**Dependencies:** Phase 3
**Status:** CRITICAL - This fixes the fake agent system from Phase 0 audit

---

## Current Sprint: Week of Feb 11, 2026

### This Week's Goals:
1. **✅ COMPLETED:** Self-Review System (Phase 4)
2. **✅ COMPLETED:** Document Management (Phase 5)
3. **✅ COMPLETED:** Skill Builder System (Phase 6)
4. **✅ COMPLETED:** Agent Delegation System (Phase 7)
5. **Update Documentation** ✅ DONE - PROJECT.md and CHECKLIST.md updated

### Active Tasks:
- [x] Deploy to Raspberry Pi and test all features
- [x] Create first custom skill (research-assistant)
- [x] Test agent delegation with toggle reporting
- [x] Verify document management in /mnt/storage

### Completed Today:
- ✅ Self-review system with Review Mode (user approval required)
- ✅ Document management skill for /mnt/storage
- ✅ Skill builder with create_skill tool
- ✅ Agent delegation system with toggleable reporting
- ✅ HELP command with all features
- ✅ Config file support for sandy.config.yaml

### Next Sprint Goals:
- Start Phase 9: Proactive Sandy (unprompted check-ins)
- Test agent system with real research/coding tasks
- Create additional custom skills

### Blocked/Issues:
- [!] None currently

---

## Completed Milestones

### ✅ Milestone 1: Foundation (Feb 1-5, 2026)
- Basic Telegram bot working
- Web UI dashboard functional
- Pattern and tracking systems operational

### ✅ Milestone 2: Reminders (Feb 6-10, 2026)
- Reminder scheduling working
- Flexible time parsing implemented
- Bug fixes for timing issues

### ✅ Milestone 3: Documentation Cleanup (Feb 11, 2026)
- Created PROJECT.md
- Created CHECKLIST.md
- Archived old documentation

---

## Notes & Decisions

### Design Decisions:
- **File-based storage** preferred over DB complexity
- **Single-user focus** for now, multi-user later
- **Natural language** interface over rigid commands
- **Proactive features** only after learning phase is solid

### Technical Debt:
- Reminders in both DB and JSON (acceptable sync risk)
- Activity log in runtime/ directory (could be consolidated)
- No authentication on Web UI (local-only for now)

### Open Questions:
1. Should auto-note addition ask for confirmation or be silent?
2. How aggressive should proactive reminders be?
3. Calendar integration - which calendars to support?

---

## Next Review: February 18, 2026

**Review cadence:** Weekly on Tuesdays

**Last updated:** February 11, 2026
**Updated by:** Developer task completion

---

## How to Use This Checklist

### When Starting a Task:
1. Mark as [~] In Progress
2. Add your name if multiple developers
3. Note any blockers or questions

### When Completing a Task:
1. Mark as [x] Completed
2. Add completion date
3. Update PROJECT.md if needed
4. Move to "Completed Milestones" if major

### When Adding New Tasks:
1. Add at bottom of appropriate phase
2. Estimate time and priority
3. Note dependencies
4. Update this header: "Last updated: [date]"

### Weekly Review:
1. Review all [~] In Progress items
2. Check for [!] Blocked items
3. Re-prioritize pending tasks
4. Celebrate completed milestones! 🎉
