#!/bin/bash
# Test script to verify Sandy's agent delegation behavior

echo "=== Sandy Agent Delegation Test ==="
echo ""
echo "This script will test if Sandy properly delegates to agents"
echo "instead of doing work herself."
echo ""

# Check if config exists
if [ ! -f "config/sandy.config.yaml" ]; then
    echo "❌ Error: config/sandy.config.yaml not found"
    echo "Please create config file before testing"
    exit 1
fi

# Check if agent configs exist
if [ ! -f "storage/agents/zilla.json" ]; then
    echo "❌ Error: storage/agents/zilla.json not found"
    exit 1
fi

if [ ! -f "storage/agents/gonza.json" ]; then
    echo "❌ Error: storage/agents/gonza.json not found"
    exit 1
fi

echo "✅ Config files found"
echo ""

# Check if SOUL.md was updated
if grep -q "Work Orchestration" soul/SOUL.md; then
    echo "✅ SOUL.md contains Work Orchestration section"
else
    echo "❌ SOUL.md missing Work Orchestration section"
    exit 1
fi

# Check if AGENTS.md was updated
if grep -q "spawn_agent" soul/AGENTS.md; then
    echo "✅ AGENTS.md contains agent tools"
else
    echo "❌ AGENTS.md missing agent tools"
    exit 1
fi

echo ""
echo "=== Configuration Check Complete ==="
echo ""
echo "To test delegation behavior, you need to:"
echo "1. Start Sandy: ./target/release/sandy start"
echo "2. In Telegram, send: 'Research quantum computing news and write a summary article'"
echo "3. Check logs for agent spawning:"
echo "   - Look for 'Spawning agent' messages"
echo "   - Look for 'execute_workflow' tool calls"
echo "   - Verify files created in storage/tasks/"
echo ""
echo "Expected behavior:"
echo "  ✓ Sandy uses execute_workflow"
echo "  ✓ Zilla spawns for research"
echo "  ✓ Gonza spawns for writing"
echo "  ✓ Output files created"
echo ""
echo "Incorrect behavior (if documentation didn't work):"
echo "  ✗ Sandy uses web_search directly"
echo "  ✗ Sandy writes content directly"
echo "  ✗ No agent spawning occurs"
echo ""
echo "Run this command in another terminal to watch logs:"
echo "  tail -f logs/sandy.log"
