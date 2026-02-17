# Sandy System Assessment - Feb 16, 2026, 23:05 CET

## What's Actually Implemented ✅

### 1. Core Agent System (Working)
- ✅ `spawn_agent` - Real execution via sub_agent
- ✅ `execute_workflow` - Sequential multi-agent chains
- ✅ Agent configs in `storage/agents/` (zilla.json, gonza.json)
- ✅ Tool filtering per agent (whitelisted tools only)
- ✅ Format validation (JSON/Markdown verification)
- ✅ Progress updates (`send_message`, `send_file`)

### 2. OpenClaw-Inspired Guardrails (Partial)
- ✅ **Tool Filtering** (`src/tools/tool_filter.rs`)
  - Hard-coded forbidden tools for Sandy (web_search, web_fetch, browser)
  - Enforced via `is_main_bot` flag in ToolRegistry
  - Sub-agents NOT affected ✅ (tested and verified)

- ✅ **Verification Requirements** (`src/tools/memory_log.rs`)
  - Solutions require `verification` parameter
  - Vague content detection (rejects "fixed", "should work", etc.)
  - Minimum content length (30 chars)
  - Verification embedded in memory files

- ✅ **Memory System**
  - `search_memory` - Text search through past solutions
  - `log_memory` - Record to categories (solutions, errors, patterns, insights)
  - Timestamp-based organization

### 3. Skill System (Implemented, but limited)
- ✅ `create_skill` - Creates markdown-based skills
- ✅ `activate_skill` - Loads and activates skills
- ⚠️ **BUT:** Skills are markdown instructions, NOT executable code
- ⚠️ **NOT like OpenClaw:** Cannot generate new tools dynamically

### 4. What's NOT Implemented ❌

#### Missing OpenClaw Features:
1. **Doctor Command** (self-healing)
   - ❌ Auto-detect tool name mismatches
   - ❌ Verify database integrity
   - ❌ Check config validity
   - ❌ Fix broken references automatically

2. **Pre-Action Hooks**
   - ❌ Search memory before problem-solving
   - ❌ Validate inputs before tool execution
   - ❌ Auto-check prerequisites

3. **Dynamic Code Generation**
   - ❌ Cannot write new Rust tools
   - ❌ Cannot compile and add tools at runtime
   - ❌ Cannot modify her own code

4. **Idempotent Operations**
   - ❌ No idempotencyKey for preventing duplicates
   - ❌ No safe retry mechanisms (beyond basic exponential backoff)

5. **JSON Schema Validation**
   - ⚠️ Format validation exists but is basic
   - ❌ No comprehensive schema validation for agent outputs

## Comparison to OpenClaw

| Feature | OpenClaw | Sandy | Status |
|---------|----------|-------|--------|
| **Tool Filtering** | ✅ Filtered registries | ✅ Hard filter | ✅ DONE |
| **Output Verification** | ✅ verify_output() | ✅ verification param | ✅ DONE |
| **Memory Search** | ✅ Vector search | ✅ Text search | ✅ DONE |
| **Circuit Breakers** | ✅ Cooldowns | ✅ Retry wrapper | ✅ DONE |
| **Doctor Command** | ✅ Auto-repair | ❌ Not implemented | ❌ MISSING |
| **Hooks System** | ✅ Event-driven | ❌ Not implemented | ❌ MISSING |
| **Skill Creation** | ✅ Dynamic code | ⚠️ Markdown only | ⚠️ PARTIAL |
| **Config Migration** | ✅ Auto-migrate | ❌ Not implemented | ❌ MISSING |
| **Idempotent Ops** | ✅ idempotencyKey | ❌ Not implemented | ❌ MISSING |
| **JSON Schema Validation** | ✅ Full validation | ⚠️ Basic | ⚠️ PARTIAL |

**OpenClaw Parity: ~50%** (5/10 core features)

## Architecture Analysis

### Current Design Strengths:
1. **Rust-based** - Memory safe, fast, production-ready
2. **Agent delegation** - Clear separation of concerns (Sandy orchestrates)
3. **File system as truth** - All data persisted, verifiable
4. **Hard guardrails** - System-level enforcement, not prompt-based

### Current Design Limitations:
1. **Static tool set** - Cannot add tools at runtime
2. **No self-healing** - Relies on manual intervention
3. **Limited introspection** - Cannot diagnose own problems
4. **No automatic validation** - Manual verification required

## What Should Be Implemented Next (Priority Order)

### PRIORITY 1: Doctor Command (Foundation for Self-Healing)
**Why first:** Foundation for all self-modification features
**What it does:**
- Validates tool names in AGENTS.md match registered tools
- Checks agent configs reference valid tools
- Verifies database integrity
- Auto-fixes common mismatches

**Files to create:**
- `src/tools/doctor.rs` - Main doctor command
- Add validation functions to existing modules

**Estimated effort:** 4-6 hours

### PRIORITY 2: Pre-Action Hooks (Memory-Driven Problem Solving)
**Why second:** Prevents Sandy from re-solving known problems
**What it does:**
- Before any problem-solving task, search memory first
- Apply previous solutions if available
- Only troubleshoot if no solution exists

**Implementation:**
- Add hook system in `src/tools/mod.rs`
- Intercept tool calls to trigger memory search
- Auto-inject memory results into context

**Estimated effort:** 3-4 hours

### PRIORITY 3: Dynamic Tool Generation (True Self-Modification)
**Why third:** Requires doctor command working first
**What it does:**
- Sandy writes new Rust tool modules
- Validates code compiles
- Tests new tool
- Adds to registry dynamically (or guides user to restart)

**Implementation:**
- Extend `create_skill` to generate Rust code
- Add code validation (cargo check)
- Add rollback on failure
- Require user approval for code changes

**Estimated effort:** 8-12 hours (complex, requires restart mechanism)

### DEFERRED: Lower Priority Features
- Config migration (not needed yet, config is stable)
- Idempotent operations (nice-to-have, not critical)
- Full JSON schema validation (basic validation working)

## Recommended Implementation Path

**TODAY (Session 1):**
1. ✅ Verify tool filter fix is working (done - tests pass)
2. ⏳ Implement doctor command (basic version)
   - Tool name validation
   - Agent config validation
   - Output mismatch detection

**NEXT SESSION:**
3. Enhance doctor command
   - Auto-fix capabilities
   - Database integrity checks
4. Test doctor command end-to-end

**FUTURE SESSIONS:**
5. Implement pre-action hooks
6. Implement dynamic tool generation (long-term goal)

## Critical Questions to Answer

1. **Code Generation Scope:**
   - Should Sandy create full Rust tools? (Requires compilation, restart)
   - Or Python scripts that integrate via bash tool? (Simpler, no restart)
   - **Recommendation:** Start with Python scripts, upgrade to Rust later

2. **Doctor Command Trigger:**
   - Manual command only? (`/doctor` in Telegram)
   - Automatic on startup?
   - Automatic before every tool call?
   - **Recommendation:** Manual + automatic on startup

3. **Memory Hook Behavior:**
   - Always search memory before problem-solving?
   - User opt-in/opt-out?
   - **Recommendation:** Always search, show results, let user decide

## Files to Read Next (Before Implementation)

1. `src/tools/mod.rs` - Understand tool registry architecture
2. `src/tools/agent_management.rs` - See how agents are validated
3. `soul/AGENTS.md` - Understand tool documentation format
4. `storage/agents/*.json` - See actual agent configs

---

**Assessment Complete:** System is solid, 50% OpenClaw parity achieved. Next logical step is **doctor command** for self-healing foundation.
