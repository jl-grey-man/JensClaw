#!/bin/bash
# Quick verification that send_message tool exists

echo "=== Verifying send_message Tool ==="
echo ""

if grep -q "send_message" soul/AGENTS.md; then
    echo "✅ send_message documented in AGENTS.md"
else
    echo "❌ send_message missing from AGENTS.md"
    exit 1
fi

if grep -q "send_message" soul/SOUL.md; then
    echo "✅ send_message documented in SOUL.md"
else
    echo "❌ send_message missing from SOUL.md"
    exit 1
fi

if grep -q "SendMessageTool::new" src/tools/mod.rs; then
    echo "✅ send_message tool registered in mod.rs"
else
    echo "❌ send_message tool not registered"
    exit 1
fi

echo ""
echo "✅ All checks passed! send_message tool is ready."
echo ""
echo "=== Ready to Test ==="
echo "1. Sandy is running on Telegram"
echo "2. Send: 'Research AI agents 2026 and write an article'"
echo "3. Expected: Immediate message 'Got it! Setting up workflow...'"
echo "4. Then: Workflow executes (Zilla → Gonza)"
echo ""
