# Sandy Development Checklist

> ⚠️ **IMPORTANT:** Update this document at the end of each major task. Mark completed items with [x] and add new items at the bottom.

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

## Phase 4: Automatic Pattern Learning 🔄 IN PROGRESS

### Session Analysis System
- [ ] **Task 4.1:** Track conversation inactivity timer (30 minutes)
  - Store last_message_timestamp per chat
  - Detect when 30 minutes pass without messages
  
- [ ] **Task 4.2:** Archive conversation session for analysis
  - Save conversation history to file
  - Trigger analysis after 30-min inactivity
  
- [ ] **Task 4.3:** Extract pattern-relevant information
  - Analyze for keywords: "always", "never", "struggle with", "when I"
  - Identify behaviors, preferences, energy patterns
  - Detect ADHD-related context
  
- [ ] **Task 4.4:** Auto-record observations to existing patterns
  - Match extracted info to appropriate pattern category
  - Call `add_observation` tool automatically
  - Update confidence based on new evidence
  
- [ ] **Task 4.5:** Auto-create new patterns when needed
  - Detect when behavior doesn't fit existing 18 categories
  - Generate pattern ID, name, description
  - Call `create_pattern` tool automatically
  
- [ ] **Task 4.6:** Log automatic pattern learning to activity feed
  - Show "Pattern observation auto-added" in Web UI
  - Track what was learned

**Priority:** High
**Estimated Time:** 3-4 days
**Dependencies:** None (all tools exist)

---

## Phase 5: Automatic Note Addition 🔄 IN PROGRESS

### Context Detection System
- [ ] **Task 5.1:** Identify note-worthy information in conversation
  - Keywords: "I prefer", "I work better", "when I", "usually"
  - Context about specific goals/tasks
  - User preferences and workarounds
  
- [ ] **Task 5.2:** Match context to relevant items
  - Determine which goal/project/task the context applies to
  - Use conversation context and recent mentions
  
- [ ] **Task 5.3:** Auto-add notes with timestamps
  - Call `add_note` tool automatically
  - Format: "[timestamp] Context: ..."
  
- [ ] **Task 5.4:** Confirmation or silent mode?
  - Option A: Ask user "Should I note that you prefer mornings?"
  - Option B: Add silently, show in Web UI
  - **Decision needed:** Which approach?

**Priority:** High
**Estimated Time:** 2-3 days
**Dependencies:** Phase 4 (shared analysis logic)

---

## Phase 6: Proactive Sandy ⏳ PENDING

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

## Current Sprint: Week of Feb 11, 2026

### This Week's Goals:
1. **Complete Phase 4** - Automatic pattern learning (30-min sessions)
2. **Start Phase 5** - Automatic note addition
3. **Documentation cleanup** ✅ DONE

### Active Tasks:
- [~] Implement 30-minute inactivity detection
- [~] Build conversation session archiver
- [ ] Create pattern extraction logic
- [ ] Test auto-observation recording

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
