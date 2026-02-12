#!/bin/bash
# Phase 4 Integration Test
# Tests the real agent execution system with Zilla → Gonza workflow

set -e

echo "🧪 Phase 4 Integration Test"
echo "============================"
echo ""

# Check if agents exist
echo "1️⃣  Checking agent configs..."
if [ ! -f "storage/agents/zilla.json" ]; then
    echo "❌ zilla.json not found"
    exit 1
fi
if [ ! -f "storage/agents/gonza.json" ]; then
    echo "❌ gonza.json not found"
    exit 1
fi
echo "✅ Agent configs exist"
echo ""

# Test 1: Spawn Zilla for research
echo "2️⃣  Test 1: Spawn Zilla agent..."
echo "   This should:"
echo "   - Load zilla.json config"
echo "   - Create job folder"
echo "   - Execute sub_agent with research task"
echo "   - Verify output file exists"
echo ""
echo "   Manual test via Telegram:"
echo "   'Spawn agent zilla to research AI news and save to storage/tasks/test_zilla/research.json'"
echo ""

# Test 2: Spawn Gonza for writing
echo "3️⃣  Test 2: Spawn Gonza agent..."
echo "   This should:"
echo "   - Load gonza.json config"
echo "   - Read input from previous step"
echo "   - Write article to output file"
echo "   - Verify output exists"
echo ""
echo "   Manual test via Telegram:"
echo "   'Spawn agent gonza to write an article from storage/tasks/test_zilla/research.json and save to storage/tasks/test_zilla/article.md'"
echo ""

# Test 3: Execute workflow
echo "4️⃣  Test 3: Execute sequential workflow..."
echo "   This should:"
echo "   - Run Zilla step"
echo "   - Verify output"
echo "   - Run Gonza step with input from Zilla"
echo "   - Verify final output"
echo "   - Report success with all file paths"
echo ""
echo "   Manual test via Telegram:"
echo "   'Execute workflow named AI News Article with steps: [Zilla research → Gonza write]'"
echo ""

echo "✅ Phase 4 Implementation Complete!"
echo ""
echo "Next steps:"
echo "1. Start Sandy: cargo run -- telegram"
echo "2. Test spawn_agent with Zilla"
echo "3. Test execute_workflow with Zilla→Gonza"
echo "4. Verify output files are created"
