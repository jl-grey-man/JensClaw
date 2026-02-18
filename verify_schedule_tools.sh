#!/bin/bash
# Verify schedule tools are properly configured

echo "🔍 Sandy Schedule Tools Verification"
echo "===================================="
echo ""

# Check 1: Tools registered in mod.rs
echo "1️⃣  Checking tool registration in mod.rs..."
if grep -q "schedule::ScheduleTaskTool::new" src/tools/mod.rs && \
   grep -q "schedule::ListTasksTool::new" src/tools/mod.rs && \
   grep -q "schedule::PauseTaskTool::new" src/tools/mod.rs && \
   grep -q "schedule::ResumeTaskTool::new" src/tools/mod.rs && \
   grep -q "schedule::CancelTaskTool::new" src/tools/mod.rs && \
   grep -q "schedule::GetTaskHistoryTool::new" src/tools/mod.rs; then
    echo "   ✅ All 6 schedule tools registered in mod.rs"
else
    echo "   ❌ Not all schedule tools registered"
    exit 1
fi
echo ""

# Check 2: Tools documented in AGENTS.md with correct names
echo "2️⃣  Checking AGENTS.md documentation..."
if grep -q "schedule_task" soul/AGENTS.md && \
   grep -q "list_scheduled_tasks" soul/AGENTS.md && \
   grep -q "pause_scheduled_task" soul/AGENTS.md && \
   grep -q "resume_scheduled_task" soul/AGENTS.md && \
   grep -q "cancel_scheduled_task" soul/AGENTS.md && \
   grep -q "get_task_history" soul/AGENTS.md; then
    echo "   ✅ All schedule tools documented in AGENTS.md with correct names"
else
    echo "   ❌ Schedule tools not properly documented in AGENTS.md"
    exit 1
fi
echo ""

# Check 3: Binary is up to date
echo "3️⃣  Checking binary build status..."
MOD_RS_TIME=$(stat -c %Y src/tools/mod.rs)
AGENTS_MD_TIME=$(stat -c %Y soul/AGENTS.md)
BINARY_TIME=$(stat -c %Y target/release/sandy 2>/dev/null || echo "0")

if [ "$BINARY_TIME" -eq "0" ]; then
    echo "   ⚠️  No release binary found. Run: cargo build --release"
elif [ "$BINARY_TIME" -ge "$MOD_RS_TIME" ]; then
    echo "   ✅ Binary is up to date (built after mod.rs changes)"
else
    echo "   ⚠️  Binary may be outdated. Consider rebuilding: cargo build --release"
fi

if [ "$AGENTS_MD_TIME" -gt "$BINARY_TIME" ] && [ "$BINARY_TIME" -ne "0" ]; then
    echo "   ℹ️  AGENTS.md updated after binary build (restart Sandy to load new docs)"
fi
echo ""

# Check 4: Sandy service status
echo "4️⃣  Checking Sandy service..."
if systemctl is-active --quiet sandy 2>/dev/null; then
    echo "   ✅ Sandy service is running"
    START_TIME=$(systemctl show sandy --property=ActiveEnterTimestamp --value)
    echo "   ℹ️  Started: $START_TIME"

    # Check if it started after AGENTS.md was modified
    AGENTS_MD_DATE=$(date -r soul/AGENTS.md '+%Y-%m-%d %H:%M:%S')
    echo "   ℹ️  AGENTS.md last modified: $AGENTS_MD_DATE"
else
    echo "   ⚠️  Sandy service is not running"
    echo "   Run: sudo systemctl start sandy"
fi
echo ""

# Check 5: Database exists
echo "5️⃣  Checking database..."
if [ -f "soul/data/runtime/microclaw.db" ]; then
    echo "   ✅ Database exists"
    TASK_COUNT=$(sqlite3 soul/data/runtime/microclaw.db "SELECT COUNT(*) FROM scheduled_tasks WHERE status='active';" 2>/dev/null)
    echo "   ℹ️  Active scheduled tasks: $TASK_COUNT"
else
    echo "   ⚠️  Database not found (will be created on first run)"
fi
echo ""

# Check 6: Logs
echo "6️⃣  Checking recent logs..."
if [ -f "logs/sandy.log" ]; then
    echo "   ✅ Log file exists"
    echo "   ℹ️  Last 5 log entries:"
    tail -5 logs/sandy.log | sed 's/^/      /'
else
    echo "   ⚠️  No log file found"
fi
echo ""

echo "===================================="
echo "✅ Verification complete!"
echo ""
echo "📋 Next steps:"
echo "   1. Ensure Sandy is running: sudo systemctl status sandy"
echo "   2. Send test reminder via Telegram: 'remind me to drink water in 2 minutes'"
echo "   3. Check logs: tail -f logs/sandy.log"
echo "   4. Verify database: sqlite3 soul/data/runtime/microclaw.db 'SELECT * FROM scheduled_tasks;'"
echo ""
