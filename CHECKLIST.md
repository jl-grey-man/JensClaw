# Sandy Development Checklist

> ⚠️ **IMPORTANT:** Update this document at the end of each major task. Mark completed items with [x] and add new items at the bottom.

## Legend
- [x] **Completed** - Feature working in production
- [~] **In Progress** - Currently being implemented
- [ ] **Pending** - Not yet started
- [!] **Blocked** - Waiting on dependency or decision

---

## What's Done ✅

### Foundation & Core (Phases 0–2)
- [x] System audit — identified fake agent system, corrected all false documentation
- [x] Storage directory structure (`storage/agents/`, `storage/tasks/`, `storage/memory/`)
- [x] TOOLS.md created (definitive tool reference)
- [x] `prompts/guard_rails.txt` created (agent safety DNA)
- [x] `src/tools/file_ops.rs` — atomic writes, path validation, traversal protection
- [x] Integrated file_ops into existing read_file/write_file tools

### Sandy Core Features
- [x] Telegram bot with Sandy personality (SOUL.md)
- [x] Web UI dashboard with real-time activity feed (http://localhost:3000)
- [x] Pattern learning system (18 ADHD categories, `patterns.json`)
- [x] Unified tracking (Goals → Projects → Tasks, `tracking.json`)
- [x] Notes system (add_note, remove_note, auto-timestamped)
- [x] Reminder scheduling with natural language time parsing
- [x] Activity logging system
- [x] Document management (`/mnt/storage/`)
- [x] Self-review system (daily analysis, user approval required)
- [x] Skill builder (`create_skill` tool, custom workflows)
- [x] Sub-agent execution (real LLM subprocesses)
- [x] HELP command

### Bug Fixes & Production Stability
- [x] Fixed: OpenRouter "Provider returned error" — forced Anthropic provider routing
- [x] Fixed: Reminders firing immediately (wrong year 2024) — injected current time into system prompt
- [x] Fixed: Activity log not updating in Web UI
- [x] Fixed: Watchdog logs in wrong directory
- [x] Fixed: Config file naming (supports `sandy.config.yaml`)
- [x] Fixed: sandy-updater.sh failing every 5 min — added cargo PATH export, fixed update detection to use `rev-list` instead of hash comparison (was false-positive when local ahead of remote) (March 2, 2026)

### API Cost Optimization
- [x] Anthropic prompt caching via `cache_control` on static system prompt
- [x] System prompt split with `---CACHE_BREAK---` marker (static vs dynamic)
- [x] AGENTS.md trimmed from 15.6KB → 2.3KB (unregistered tool docs archived to `AGENTS_FUTURE.md`)
- [x] Removed duplicate tool registrations (web_search, activate_skill)
- [x] Cached token tracking and logging

### Phase 3: The Hands — Skill Scripts ✅ COMPLETED
- [x] Setup Python virtual environment (`/storage/.venv/`)
- [x] Create Journalistic Research skill
  - `src/skills/journalistic-research/SKILL.md`
  - `scripts/run_research.py` — takes query + output_path, performs web search (DuckDuckGo), saves structured JSON
  - **Cost Efficiency:** Uses existing web_search tool, no additional API costs; saves results locally to avoid repeated searches
- [x] Create Journalistic Writing skill
  - `src/skills/journalistic-writing/SKILL.md`
  - `scripts/write_article.py` — reads input file, transforms to article using local processing only, NO web access
  - **Cost Efficiency:** Zero API costs — template-based transformation, no LLM calls required
- [x] Create Agent Factory (`src/tools/agent_factory.rs`)
  - Tool whitelisting per agent config (anti-hallucination)
  - Writes JSON to `storage/agents/{agent_id}.json`
  - Predefined templates: zilla, gonza, file-organizer, code-assistant
- [x] Test end-to-end: research script → verify output → writing script → verify article ✅

### Phase 4: Real Agent Execution ✅ COMPLETED
- [x] Rebuild `spawn_agent` using `sub_agent` as execution engine
  - Loads agent config from `storage/agents/{agent_id}.json`
  - Creates job folder: `storage/tasks/{job_id}/`
  - Calls sub_agent with guard rails and task prompt
  - Verifies output file exists before returning success
  - Updates agent registry with status
- [x] Implement `execute_workflow` tool for sequential workflows
  - Multi-step agent chains (Zilla → VERIFY → Gonza → VERIFY)
  - Stops on verification failure
  - File existence, size, and content validation
- [x] Agent configs created: `zilla.json`, `gonza.json`
- [x] All code compiles with `cargo check` ✅

### Agent System Enhancements (February 16, 2026) ✅ COMPLETED
- [x] **Tool Filtering** — Agents only get whitelisted tools from config
  - `SubAgentTool::with_registry()` accepts custom tool registry
  - `create_filtered_registry()` builds registry from allowed_tools list
  - Zilla gets 5 tools (web_search, web_fetch, write_file, read_file, bash)
  - Gonza gets 2 tools (read_file, write_file) — NO web access
  - 9 comprehensive tests — all passing ✅
- [x] **Format Validation** — JSON/Markdown output verification
  - `verify_output()` validates file format matches expected type
  - JSON validation: Ensures valid JSON structure
  - Markdown validation: Checks for markdown markers and sufficient content
  - Workflow stops if format validation fails
  - 10 format validation tests — all passing ✅
- [x] **Progress Updates** — Real-time status during long operations
  - `send_message` tool for mid-workflow status updates
  - Immediate acknowledgment before starting work
  - Progress updates every 30-60 seconds for long tasks
  - Documented in SOUL.md and AGENTS.md
- [x] **File Delivery** — Automatic Telegram file sending
  - `send_file` tool sends output files as attachments
  - Optional caption with file details (size, word count, sources)
  - Sends files after workflow completion
  - Works with any file type (MD, JSON, PDF, etc.)
- [x] **Effort Levels** — User controls research depth
  - Quick: 2-3 sources, 2-3 minutes
  - Medium: 5-7 sources, 5-7 minutes (default)
  - Full: 10+ sources, 10+ minutes
  - Sandy detects keywords in user requests
- [x] **Working Directory** — Output files in `/mnt/storage/`
  - Agent configs stay in project: `/home/jens/sandy/storage/agents/`
  - Output files: `/mnt/storage/tasks/` (Samba-shared)
  - Accessible from other computers via network share

---

## What's Next 🔨

### Phase 4: Real Agent Execution ✅ COMPLETED

**Goal:** Fix the broken agent system — make `spawn_agent` actually execute work using the Phase 3 skill scripts.

**Status:** ✅ COMPLETED - All core functionality implemented and compiling

- [x] Rebuild `spawn_agent` using `sub_agent` as execution engine
  - ✅ Load agent config from `storage/agents/{agent_id}.json`
  - ✅ Create job folder: `storage/tasks/{job_id}/`
  - ✅ Call sub_agent with guard rails and task prompt
  - ✅ Verify output file exists before returning success
  - ✅ Update agent registry with status
- [x] Implement sequential workflow support
  - ✅ New tool: `execute_workflow(steps)`
  - ✅ Zilla → VERIFY → Gonza → VERIFY
  - ✅ Stop on verification failure
- [x] Add verification after every step (file exists? size > 0? readable?)
  - ✅ File existence check
  - ✅ File size check
  - ✅ Content validation (no ERROR prefix)
- [x] Integration testing ✅ COMPLETED
  - Tested via Telegram with real workflows
  - Tool filtering verified (Zilla: 5 tools, Gonza: 2 tools)
  - Format validation verified (JSON/Markdown checking)
  - Progress updates working
  - File delivery working
  - Error propagation and failure handling tested

**Dependencies:** Phase 3 ✅ (skill scripts complete)

**Implementation Details:**
- `src/tools/agent_management.rs` - Rewritten with real execution via sub_agent (614 lines)
- `src/tools/execute_workflow.rs` - New sequential workflow tool (457 lines)
- `src/tools/agent_factory.rs` - Agent config creation with tool whitelisting (449 lines)
- Agent configs created: `zilla.json`, `gonza.json`
- All tools registered in `src/tools/mod.rs`
- Code compiles with cargo check ✅ (only minor warnings remain)

---

### Phase 5: Self-Healing System ✅ COMPLETED (February 17, 2026)

**Goal:** Implement OpenClaw-style self-healing and anti-hallucination features
**Status:** ✅ COMPLETE — 12 features implemented, 406 tests passing

#### Self-Healing Features ✅
- [x] **Hook system** — PreHook/PostHook pipeline on all tool calls (`src/hooks.rs`, 6 tests)
- [x] **Loop detection** — Blocks repeated identical calls, 3x in window of 8 (`src/hooks/loop_detect.rs`, 5 tests)
- [x] **Execution logging** — JSONL tool call logging with 10MB rotation (`src/exec_log.rs`, 5 tests)
- [x] **Context window guard** — Auto-compact at 80%, hard limit at 95% (`src/context_guard.rs`, 10 tests)
- [x] **Model failover with cooldown** — Exponential backoff per model, auto-fallback (`src/llm_retry.rs`, 8 tests)
- [x] **Doctor auto-fix** — Creates missing dirs, repairs corrupt files (`src/tools/doctor.rs`, 6 tests)
- [x] **Temporal memory decay** — Exponential decay with category half-lives (`src/memory_decay.rs`, 10 tests)
- [x] **BM25 memory search** — TF-IDF ranking + temporal decay weighting (`src/tools/memory_search.rs`, 10 tests)
- [x] **Solution confidence tracking** — Wilson score interval, flags low-confidence solutions (`src/confidence.rs`, 13 tests)
- [x] **Memory injection hook** — Auto-injects relevant context into agent calls (`src/hooks/memory_inject.rs`, 6 tests)
- [x] **Heartbeat check-ins** — 6-hour health checks to control chat (`src/heartbeat.rs`, 3 tests)
- [x] **Pattern-to-action pipeline** — Auto-suggests skills/tasks from frequent patterns (`src/pattern_actions.rs`, 9 tests)

#### Previously Completed ✅
- [x] **Tool filtering system** — Sandy blocked from web tools, must delegate to agents
- [x] **Verification requirements** — Solutions must provide proof, vague content rejected
- [x] **Doctor command** — System diagnostics (tool refs, agent configs, DB integrity)

---

#### Deferred (Lower Priority) 📋
- [ ] Dynamic code generation (Sandy creates new tools at runtime)
- [ ] Verify path traversal protection applied everywhere
- [ ] Error handling standards (errors written to output files, not just logged)
- [ ] Config migration (auto-update old configs)
- [ ] Idempotent operations (prevent duplicate actions)
- [ ] Full JSON schema validation

---

### Phase 6: Cleanup ⏳ PENDING

- [ ] Remove fake agent code from `agent_management.rs`
- [ ] Update HELP command (mark agent system as beta until proven)
- [ ] Archive old documentation to `/old/`

---

### Phase 7: Proactive Sandy ⏳ PENDING

- [ ] Daily/weekly check-ins ("How's your energy today?")
- [ ] Task deadline reminders based on due dates
- [ ] Pattern-based suggestions ("You have high energy mornings — tackle hard tasks now")
- [ ] Unprompted accountability ("You said you'd exercise 3x this week — how's that going?")

---

### LLM Resilience — Auto-Discovery Improvements ⏳ PENDING

- [ ] Cache discovered model for the session (avoid re-querying `/models` on every message)
- [ ] Persist discovered model to config so it survives restarts
- [ ] Notify user via Telegram when a fallback/discovered model is being used
- [ ] Distinguish 402 (out of credits) from 404 (model gone) — auto-discovery can't fix billing
- [ ] Add OpenRouter credit balance check on startup (warn if low)

---

### Backlog

**Web UI Enhancements:**
- [ ] Edit items directly in Web UI
- [ ] Create items from Web UI (quick-add form)
- [ ] Dark mode toggle
- [ ] Calendar view for deadlines and reminders

**Multi-User Support:**
- [ ] User isolation (separate data dirs per user)
- [ ] Separate pattern learning per user
- [ ] Web UI auth

**Nice-to-Have:**
- [ ] Export data to CSV/JSON
- [ ] Search/filter in Web UI
- [ ] Inline code documentation (rustdocs)
- [ ] User guide for Telegram commands

---

## Key Files

| File | Purpose |
|------|---------|
| `PROJECT.md` | Project overview |
| `CHECKLIST.md` | This file — development tracker |
| `CHECKLIST_OLD.md` | Previous detailed checklist (archived) |
| `.opencode/plans/IMPLEMENTATION_PLAN.md` | Full implementation plan with technical details |
| `soul/SOUL.md` | Sandy's personality |
| `soul/AGENTS.md` | Registered tool documentation (trimmed for cost) |
| `soul/AGENTS_FUTURE.md` | Archived docs for planned/unregistered tools |
| `config/sandy.config.yaml` | Bot configuration |
| `src/tools/mod.rs` | Tool registry |
| `src/llm.rs` | LLM provider (OpenRouter, prompt caching, provider routing) |
| `src/hooks.rs` | Hook system (PreHook/PostHook pipeline) |
| `src/exec_log.rs` | Execution logging (JSONL) |
| `src/context_guard.rs` | Context window management |
| `src/llm_retry.rs` | Model failover with cooldown |
| `src/confidence.rs` | Solution confidence tracking (Wilson score) |
| `src/memory_decay.rs` | Temporal memory decay |
| `src/heartbeat.rs` | Periodic health check-ins |
| `src/pattern_actions.rs` | Pattern-to-action pipeline |

---

**Last updated:** February 17, 2026
**Major update:** Self-healing system complete — 12 features, 406 tests passing
**Review cadence:** Weekly on Tuesdays
**Next review:** February 24, 2026

---

## 🚀 START HERE NEXT SESSION

**Status:** Self-healing system complete. Service running.
**Next:** Phase 7 — Proactive Sandy (unprompted check-ins, pattern-based suggestions)
