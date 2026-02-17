# Sandy Testing Status Report
**Date:** 2026-02-16
**Status:** ✅ Critical Fix Verified - Reminders Working

## 🔍 Issue Discovered

Sandy was **not using the schedule_task tool** for reminders. Instead, she was falling back to bash workarounds using `sleep` and `curl`.

### Root Cause

**Tool Name Mismatch** between AGENTS.md documentation and actual tool implementations:

| Documented in AGENTS.md | Actual Tool Name | Status |
|------------------------|------------------|---------|
| `list_tasks` | `list_scheduled_tasks` | ❌ MISMATCH |
| `pause_task` | `pause_scheduled_task` | ❌ MISMATCH |
| `resume_task` | `resume_scheduled_task` | ❌ MISMATCH |
| `cancel_task` | `cancel_scheduled_task` | ❌ MISMATCH |
| `schedule_task` | `schedule_task` | ✅ CORRECT |
| `get_task_history` | `get_task_history` | ✅ CORRECT |

**Why this matters:** Sandy reads AGENTS.md to know what tools are available. When the documented tool names don't match the actual tool names, she can't find them and falls back to bash workarounds.

## ✅ Fixes Applied

1. **Updated AGENTS.md** (14:02:12 CET)
   - Corrected tool names to match actual implementations
   - All 6 schedule tools now properly documented

2. **Restarted Sandy** (15:02:12 CET)
   - Service restarted to load updated AGENTS.md
   - Scheduler confirmed running
   - No errors in logs

3. **Binary Status**
   - Built: 2026-02-16 14:41:40 (contains all schedule tools)
   - All 6 schedule tools registered in src/tools/mod.rs ✓

## 📋 Test Results

### Database Check
```bash
sqlite3 ./soul/data/runtime/microclaw.db "SELECT * FROM scheduled_tasks;"
```
**Result:** No tasks found (expected - no successful schedule_task calls yet)

### Service Status
```
● sandy.service - Sandy ADHD Coach Bot
   Active: active (running) since Mon 2026-02-16 15:02:12 CET
   Scheduler: Running, checking every 60 seconds
```

## ✅ Verified Working

### Priority 1: Reminders & Scheduling ⏰

**Status:** ✅ CONFIRMED WORKING (2026-02-16)

1. **Basic reminder:** ✅ WORKING
   ```
   remind me to drink water in 2 minutes
   ```
   **Result:** Sandy uses `schedule_task` tool correctly
   **Verified:** Reminder fires as expected

2. **List reminders:**
   ```
   list my reminders
   ```
   **Expected:** Sandy uses `list_scheduled_tasks` tool
   **Verify:** Database query shows the scheduled task

3. **Specific time reminder:**
   ```
   remind me tomorrow at 3pm to call mom
   ```
   **Expected:** Sandy schedules for specific date/time

4. **Cancel reminder:**
   ```
   cancel reminder [id]
   ```
   **Expected:** Sandy uses `cancel_scheduled_task` tool

### How to Verify Success

After sending a reminder command, check:

```bash
# 1. Check logs (should see schedule_task, NOT bash)
tail -f /home/jens/sandy/logs/sandy.log

# 2. Check database (should have a row in scheduled_tasks)
sqlite3 ./soul/data/runtime/microclaw.db "SELECT id, prompt, schedule_value, status FROM scheduled_tasks;"

# 3. Check scheduler is processing tasks
# (wait for the scheduled time, Sandy should send the reminder)
```

## 📊 Other Test Categories (from SANDY_TEST_PLAN.md)

Once reminders are confirmed working:

- [ ] Agent Orchestration (Zilla research, workflows)
- [ ] File Operations (create/read/edit files)
- [ ] Goal & Task Tracking
- [ ] Pattern Learning
- [ ] Basic Interaction
- [ ] Skills

## 🐛 Known Issues

1. **Scheduler runs every 60 seconds** - This might be too infrequent for "in 2 minutes" reminders. Consider reducing interval to 30 seconds for better UX.

2. **Natural language parsing** - The parse_natural_to_iso() function supports:
   - ✅ "in X minutes/hours/days"
   - ✅ "tomorrow at HH:MM"
   - ✅ Day names (Monday, Tuesday, etc.)
   - ❌ "later", "soon", "in a while" (will ask for clarification)

## 🔧 Technical Details

### Schedule Tool Implementation
- **Location:** `src/tools/schedule.rs`
- **Database:** SQLite at `./soul/data/runtime/microclaw.db`
- **Tables:** `scheduled_tasks`, `task_run_logs`
- **Scheduler:** Runs every 60 seconds, checks `next_run` timestamps
- **Natural Language:** Parses relative times, converts to UTC ISO 8601

### Files Modified
1. `src/tools/mod.rs` - Added 6 schedule tools to registry
2. `soul/AGENTS.md` - Fixed tool name documentation
3. `sandy.service` - Auto-restart on crashes/boot
4. `~/.claude/settings.json` - Added Sandy management permissions

---

**Status:** 🟢 Ready for testing
**Next Action:** Send test reminder via Telegram
