# Sandy Development TODO

## ✅ Bug Fixes (Completed)
- [x] Fix activity log path - Web UI now reads fresh from file every request
- [x] Fix reminder scheduling - Timestamps now properly converted to UTC

## 🔄 Features to Add

### High Priority
- [ ] **2. Automatic pattern learning with 30-min session analysis**
  - **Trigger:** After 30 mins of conversation inactivity
  - **Action:** Analyze all messages from that session
  - **Extract:** Behaviors, struggles, preferences, patterns
  - **Auto-record:** Observations to appropriate pattern categories
  - **Auto-create:** New patterns when behavior doesn't fit existing ones
  - **Add notes:** Context to relevant goals/tasks
  - **Implementation:** Add session analysis logic that triggers after quiet period

- [ ] **3. Automatic note addition**
  - **Trigger:** When Sandy learns useful context about goals/tasks
  - **Action:** Auto-call `add_note` tool
  - **Examples:**
    - User mentions "I work better on this in mornings" → Add to task notes
    - User struggles with a specific aspect → Document for future reference
  - **Implementation:** Add conversation analysis to detect note-worthy info

### Medium Priority
- [ ] Make Sandy proactive
  - Send check-ins and reminders unprompted
  - Pattern-based suggestions
- [ ] Web UI enhancements
  - Edit items directly in web UI
  - Dark mode
  - Reminders view (see risk analysis below)
- [ ] Multi-user support

## Risk Analysis: Adding Reminders to tracking.json

### Current State:
- **Reminders stored in:** SQLite database (scheduled_tasks table)
- **Web UI reads from:** JSON files (tracking.json, activity_log.json)
- **Problem:** Web UI can't display reminders without DB access

### Option A: Add Reminders to tracking.json

**✅ Benefits:**
- Web UI can display reminders immediately
- Simple implementation - just save/load JSON
- No database connection needed for web UI
- Consistent with goals/projects/tasks architecture

**⚠️ Risks:**
1. **Data Duplication:** Same data in DB + JSON = sync issues
   - Risk: Low (reminders don't change often)
   - Mitigation: Write to both on create/delete

2. **Scheduler Dependency:** Scheduler uses DB, not JSON
   - Risk: Medium (reminders might not fire if out of sync)
   - Mitigation: Always keep DB as source of truth, JSON is view-only

3. **Performance:** JSON file grows with many reminders
   - Risk: Very Low (reminders are small, usually <100 per user)
   - Mitigation: Purge old completed reminders

4. **Concurrency:** Multiple writes could corrupt JSON
   - Risk: Low (single-user system currently)
   - Mitigation: File locking or atomic writes

### Option B: Give Web UI Database Access

**✅ Benefits:**
- Single source of truth
- No sync issues
- Real-time data

**⚠️ Risks:**
1. **Complexity:** Need to share DB connection with web server
   - More code changes
   - Thread safety concerns

2. **Coupling:** Web UI tightly coupled to DB schema
   - Harder to change schema later

### Recommendation: Option A (Add to tracking.json)

**Rationale:**
- For single-user ADHD coach, sync issues are minimal
- JSON is human-readable and debuggable
- Faster to implement
- Matches existing architecture

**Implementation:**
1. When creating reminder via `schedule_task`, also append to tracking.json reminders array
2. When deleting/canceling reminder, remove from both DB and JSON
3. Web UI reads reminders from JSON (read-only view)
4. Scheduler continues to use DB (source of truth)

### Reminder Timezone Issue
**Status:** ✅ Fixed in parse_datetime tool
- All timestamps now converted to UTC before storage
- Ensures consistent comparison in scheduler

### Activity Log Path
**Status:** ✅ Fixed
- ActivityLogger now re-reads file fresh on every request
- Web UI shows live updates every 5 seconds
