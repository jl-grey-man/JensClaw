# CLAUDE.md - Sandy Development Guide

# Rules

## Never break what works
- Run the FULL test suite before AND after every change — not just the tests you think are related
- If any test fails after your change, revert and fix before proceeding
- Never modify code you weren't asked to touch
- Never refactor, "improve", or clean up adjacent code while working on something else
- Check that the application still starts/builds after every change

## Atomic changes only
- One logical change per commit
- Every commit must leave the project in a working state (tests green, app builds)
- If a change touches multiple files, they all go in one commit — but the change itself stays minimal
- No "while I'm here" fixes — make a separate commit or note it for later

## Documentation is not optional
- If you change behavior, update the docs in the SAME commit (not "later")
- This file (CLAUDE.md) is the source of truth — if code contradicts it, the code is wrong
- New pattern or convention? Document it here BEFORE using it
- Removing a feature? Remove its documentation in the same commit
- Added a new dependency? Update CLAUDE.md
- Added or changed commands/scripts? Update CLAUDE.md
- Changed directory structure or moved files? Update CLAUDE.md
- New environment variables required? Update CLAUDE.md
- Discovered a fragile area? Add it to the Fragile section

## Don't guess, ask
- Ambiguous requirements → ask before implementing
- Unsure if a change will break something → say so
- Unfamiliar with a pattern in the codebase → read it first, don't assume
- Never assume "this is probably fine"

## Test discipline
- New code = new tests (same commit)
- Changed behavior = updated tests (same commit)
- Tests must verify behavior, not just cover lines
- If the project has no tests yet, add them for any new code you write

## Failure is an option
- If you don't know how to fix something, SAY SO — don't fake a solution
- "I don't know" is always better than a wrong fix that hides the real problem
- Never paper over errors with try/catch, silent fallbacks, or suppressed warnings
- If a fix feels like guesswork, stop and explain what you've tried and where you're stuck

## Diagnose before you fix
- Never start changing code until you understand WHY it's broken
- Read the error, trace the code path, check the logs — then fix
- Don't throw code at the problem hoping something sticks
- If your first fix didn't work, stop and re-analyze — don't try a second guess
- "I changed 5 things and now it works" means you don't know what fixed it — revert and do it properly

## No shortcuts, no quickfixes
- Every fix must be a proper fix — no hacks, workarounds, or "temporary" solutions
- If doing it properly takes longer, that's fine — stability beats speed
- Never disable a check, skip validation, or comment out code to make something work
- If the proper fix is too complex for the current scope, say so instead of hacking around it

## Respect the Fragile section
- Before changing ANY code, check the Fragile section below for warnings about that area
- If your change touches a fragile area, take extra care and test more thoroughly
- If you discover a new fragile area (tight coupling, brittle integration, non-obvious dependency), add it to the Fragile section

## Task tracking belongs in Checklist.md
- Never put todos, progress tracking, or phase status in CLAUDE.md
- Use Checklist.md for all task tracking — it's the living progress doc
- Check off items as you complete them
- When starting a new phase or big task, update the checklist first

---

## What Is Sandy?

Sandy is an ADHD coach and personal assistant Telegram bot built in Rust. She helps neurodivergent users manage tasks, track goals, learn behavioral patterns, and stay accountable. Sandy is an **orchestrator** — she delegates research/writing to specialized agents (Zilla, Gonza) and handles coaching, pattern analysis, and task management directly.

Runs on a Raspberry Pi 5 at `/home/jens/sandy`. Build artifacts, logs, and cargo registry live on SSD via symlinks (see Infrastructure below).

## Tech Stack

- **Language:** Rust 2021 edition, v0.0.31
- **Async runtime:** tokio (full features)
- **Telegram:** teloxide 0.17
- **HTTP:** reqwest 0.12, axum 0.7 (web dashboard)
- **Database:** rusqlite 0.32 (bundled SQLite)
- **Search:** tavily 2.1
- **Audio:** async-openai 0.24 (Whisper transcription)
- **Serialization:** serde, serde_json, serde_yaml
- **Other key deps:** chrono, cron, regex, uuid, dirs, include_dir, tracing

## Project Structure

### Source Files (`src/`)

| File | Purpose |
|---|---|
| `main.rs` | Entry point, CLI args, starts Telegram/Discord/web |
| `lib.rs` | Re-exports core modules |
| `config.rs` | YAML config loading, defaults, validation |
| `telegram.rs` | Telegram message handler, conversation loop |
| `discord.rs` | Discord bot integration |
| `llm.rs` | LLM API calls (Claude, OpenAI routing) |
| `llm_retry.rs` | Retry logic for LLM calls |
| `backoff.rs` | Exponential backoff for API rate limits |
| `claude.rs` | Claude-specific API client |
| `memory.rs` | Memory manager: builds memory context (full or summary mode), loads rules, counts entries |
| `memory_decay.rs` | Time-based decay weighting for memory entries |
| `db.rs` | SQLite database schema and queries |
| `scheduler.rs` | Cron/once task scheduler (runs in background) |
| `hooks.rs` | Event hook system (pre/post tool execution) |
| `skills.rs` | Skill loading and management |
| `builtin_skills.rs` | Compile-time embedded skills via `include_dir!` |
| `activity.rs` | Activity logging |
| `atomic_io.rs` | Atomic file write helpers (temp+rename) |
| `error.rs` | Error types |
| `error_classifier.rs` | Classifies errors for self-healing |
| `confidence.rs` | Decay-weighted confidence scoring for patterns |
| `context_guard.rs` | Context window size management, token budget logging |
| `exec_log.rs` | Execution logging |
| `gateway.rs` | API gateway / web dashboard (axum) |
| `heartbeat.rs` | Health check / heartbeat system |
| `logging.rs` | Tracing/logging setup |
| `mcp.rs` | Model Context Protocol integration |
| `pattern_actions.rs` | Actions triggered by detected patterns |
| `proactive.rs` | Proactive check-in system |
| `setup.rs` | First-run setup wizard |
| `config_wizard.rs` | Interactive config wizard |
| `transcribe.rs` | Audio transcription via Whisper |
| `whatsapp.rs` | WhatsApp integration (experimental) |

### Tools (`src/tools/`) — 46 registered tools

The tool system implements a `Tool` trait. Tools are registered in `mod.rs` into two registries: full (for Sandy) and filtered (for sub-agents).

| File | Tools |
|---|---|
| `mod.rs` | Tool trait, registry, auth context (`__sandy_auth` key) |
| `sub_agent.rs` | `sub_agent` — run sub-agent with filtered tool access |
| `agent_factory.rs` | `spawn_agent`, `execute_workflow` delegation |
| `agent_management.rs` | `list_agents`, `agent_status`, `create_agent_config` |
| `send_message.rs` | `send_message` — send Telegram messages |
| `send_file.rs` | `send_file` — send files via Telegram |
| `bash.rs` | `bash` — shell command execution |
| `read_file.rs` | `read_file` — read files (with path validation + extra allowed roots) |
| `write_file.rs` | `write_file` — write files (atomic, path-validated) |
| `edit_file.rs` | `edit_file` — edit existing files |
| `glob.rs` | `glob` — file pattern matching |
| `grep.rs` | `grep` — content search |
| `browser.rs` | `browser` — headless browser |
| `web_search.rs` | `web_search` — web search via Tavily |
| `web_fetch.rs` | `web_fetch` — fetch web content |
| `memory.rs` | `read_memory`, `write_memory` — structured memory access |
| `memory_log.rs` | `log_memory` — append to memory categories (insights/solutions/patterns/errors) |
| `memory_search.rs` | `search_memory` — search memory entries |
| `patterns.rs` | `read_patterns`, `add_observation`, `update_hypothesis`, `create_pattern` |
| `tracking.rs` | `read_tracking`, `create_goal`, `create_project`, `create_task`, `update_status`, `add_note`, `remove_note`, `get_task_history` |
| `schedule.rs` | `schedule_task`, `list_scheduled_tasks`, `cancel_scheduled_task`, `pause_scheduled_task`, `resume_scheduled_task` |
| `todo.rs` | `todo_read`, `todo_write` |
| `export_chat.rs` | `export_chat` — export conversation history |
| `activate_skill.rs` | `activate_skill` — load a skill module |
| `create_skill.rs` | `create_skill` — create new skill definitions |
| `doctor.rs` | `doctor` — system health diagnostics |
| `parse_datetime.rs` | `parse_datetime` — natural language date/time parsing |
| `transcribe.rs` | `transcribe_audio` — Whisper transcription |
| `file_ops.rs` | Path validation (`validate_path`, `validate_path_with_extras`), atomic writes |
| `tool_filter.rs` | Per-agent tool access filtering |
| `path_guard.rs` | Path security guards |
| `mcp.rs` | MCP tool integration |

### Data Layout

```
soul/
  SOUL.md              # Core identity/personality
  AGENTS.md            # System capabilities, tool docs, guardrails
  IDENTITY.md          # Name, emoji, vibe
  data/
    skills/            # Runtime skills (weather, pdf, journalistic-*, etc.)
    runtime/
      microclaw.db     # Main SQLite database
      tracking.json    # Goals, projects, tasks
      patterns.json    # Behavioral patterns (decay-weighted)
      activity_log.json
      exec_log.jsonl
      memory/          # Category files (authoritative runtime memory): insights.md, solutions.md, patterns.md, errors.md, rules.md
      groups/          # Per-group conversation logs
storage/
  agents/              # Agent config JSON files
  memory/              # Persistent memory store
  tasks/               # Task data
  TRACKING.json        # Legacy tracking
config/
  sandy.config.yaml    # Main config (gitignored, use .example)
src/skills/
  journalistic-research/
  journalistic-writing/  # Compile-time embedded skills only
```

### Security Model

- **Path validation:** All file reads/writes go through `file_ops::validate_path` which enforces allowed roots: `/storage`, `/mnt/storage`, `/tmp`, configured `working_dir`, and runtime `data_dir`
- **Atomic writes:** Files written via temp→verify→rename pattern (`atomic_io.rs`)
- **Tool filtering:** Sub-agents get a restricted tool registry (no `send_message`, no `bash`, etc.)
- **Auth context:** Tool calls carry `__sandy_auth` (NOT `__microclaw_auth`) for permission checks
- **Guardrails:** Sandy cannot use `web_search`/`web_fetch`/`browser` directly — must delegate to agents

## How to Work on Sandy

### Build & Test

```bash
cargo build --release     # ~2.5 min on Pi 5, much faster on x86
cargo test                # Run tests (see known failures below)
cargo test -- --nocapture # With stdout
```

### Config

Main config: `config/sandy.config.yaml` (gitignored). Copy from `.example`.

Key config values:
- `data_dir`: defaults to XDG path (`~/.local/share/sandy`), NOT `./sandy.data`. Production config overrides to `./soul/data`
- `working_dir`: defaults to `/tmp/sandy_work`, NOT `./tmp`. Production config overrides to `./storage`
- `telegram_bot_token`, `api_key` (OpenRouter), `openai_api_key` (Whisper)

LLM model config:
- `model`: primary model (e.g. `anthropic/claude-sonnet-4.5`)
- `fallback_models`: list of models to try if primary fails (e.g. `anthropic/claude-3.7-sonnet`, `anthropic/claude-haiku-4.5`)
- **Auto-discovery failsafe**: if ALL configured models fail with 404/400, Sandy queries OpenRouter's `/models` endpoint and automatically picks the best available Claude model. This prevents total outage when model IDs are deprecated.

### Deploy

```bash
sudo systemctl restart sandy
journalctl -u sandy -f        # Follow logs
```

Auto-updater: systemd timer (`sandy-updater.timer`) triggers `scripts/sandy-updater.sh` every 5 minutes (oneshot — checks once and exits). The script exports `$HOME/.cargo/bin` to PATH (systemd doesn't load shell profiles) and uses `git rev-list HEAD..origin/main --count` for update detection (correctly handles local-ahead scenarios). The old `scripts/auto-update.sh` is legacy (infinite loop, unused).

### Infrastructure — SD Card + SSD

The Pi has a small SD card (29G) and an SSD at `/mnt/storage` (916G). Heavy directories are symlinked to SSD:

| Symlink | Target | Purpose |
|---|---|---|
| `/home/jens/sandy/target` | `/mnt/storage/sandy/target` | Build artifacts (~7G) |
| `/home/jens/sandy/logs` | `/mnt/storage/sandy/logs` | Service logs |
| `/home/jens/.cargo/registry` | `/mnt/storage/cargo-registry` | Crate cache |

**CRITICAL:** The systemd service (`sandy.service`) has `Restart=always` and `ExecStart` resolves through the target symlink. If the symlink breaks or the SSD is unmounted, Sandy enters a restart loop. Always ensure the symlink target exists before restarting.

**Never delete `target/` while Sandy is running** — stop the service first, then make changes, then restart.

### Key Conventions

- **Atomic file I/O:** Always use `atomic_io::write_atomic()` for data files, never raw `fs::write()`
- **File locking:** All read-modify-write on JSON/md data files must hold the appropriate static Mutex: `TRACKING_LOCK` (tracking.rs), `PATTERNS_LOCK` (patterns.rs), `MEMORY_LOG_LOCK` (memory_log.rs). Acquire lock before read, hold through write.
- **Auth key:** Tests must use `"__sandy_auth"` (renamed from `__microclaw_auth`)
- **Path security:** All file tools validate paths against allowed roots before I/O
- **Tool filter:** Sub-agents use `ToolRegistry::new_sub_agent()` with restricted tool lists
- **Memory verification:** Solutions logged to memory must include `verification` field with proof
- **No direct web access:** Sandy delegates research to Zilla agent, writing to Gonza agent
- **DB mutex recovery:** Use `.lock().unwrap_or_else(|e| e.into_inner())` on db.rs, never bare `.unwrap()`
- **Error logging:** Never `let _ =` on DB writes (`save_session`, `store_message`). Always log errors with `tracing::error!`
- **Soul file formatting:** No asterisks in soul/SOUL.md, soul/AGENTS.md, soul/IDENTITY.md. Use ALL CAPS headers, plain text, [Brackets] for emphasis, dashes for lists.

## Known Test Failures

**None.** All 560 tests pass (500 lib + 60 integration, 2 doc-tests ignored).

## Security Model — Memory System

- **Prompt injection prevention:** All memory content (AGENTS.md, insights, solutions, patterns, errors, rules) is XML-escaped via `sanitize_xml()` before injection into system prompts (`src/memory.rs`)
- **Fail-closed auth:** `authorize_chat_access()` and all write tools (memory, patterns, tracking, memory_log) require `__sandy_auth` context. Missing auth = denied.
- **Auth on write tools:** `create_pattern`, `add_observation`, `update_hypothesis`, `create_goal`, `create_project`, `create_task`, `update_status`, `add_note`, `remove_note`, `log_memory` all require auth
- **Confidence lock protection:** Only control chats can set `confidence_locked = true` on patterns
- **Hook injection escaping:** Memory context injected into sub-agent tasks via `MemoryInjectHook` is XML-escaped
- **Input validation:** Memory writes reject content >5000 chars, pattern fields >1000 chars, notes >2000 chars. Content containing XML closing tags matching prompt structure (e.g., `</recent_solutions>`) is rejected.

## Token Optimization

Sandy uses several strategies to minimize LLM token consumption:

- **Session windowing:** Only the last `context_window_messages` (default 12) messages are sent to the LLM. Older messages are summarized as a preamble (first sentence extraction, 2000 char cap). Full session stays in SQLite for saving.
- **Memory summary mode:** `memory_injection_mode: "summary"` (default) injects counts + latest preview per category + rules instead of bulk memory. Rules always injected. Use `"full"` to restore bulk injection.
- **Token budget logging:** `tracing::info` logs system/tool/message token estimates on first LLM iteration per user message.
- **Compaction manages storage, windowing manages sending:** `compact_messages()` triggers when stored messages > `max_session_messages` (40). Windowing is applied independently at send time.

Key config values:
- `context_window_messages: 12` — messages sent to LLM per call (doubled for resumed jobs)
- `max_session_messages: 40` — stored messages before compaction
- `compact_keep_recent: 15` — kept after compaction
- `max_history_messages: 20` — DB history fallback
- `max_tool_iterations: 50` — max tool loop iterations per query
- `memory_injection_mode: "summary"` — "summary" or "full"

## Long Job Checkpoint & Resume

When Sandy hits `max_tool_iterations`, she saves a checkpoint instead of giving up:
1. The job summary is stored in `chat_settings` (`paused_job` key)
2. Sandy responds: *"⏸ I've hit my step limit — progress is saved. Reply **continue** and I'll pick up right where I left off."*
3. The full session (all tool calls and results) is saved to SQLite as usual

When the user replies with "continue" (or "keep going", "resume", etc.):
1. Sandy detects `paused_job` in `chat_settings` + a continuation trigger
2. The `paused_job` key is cleared
3. The "continue" message is replaced with a resume instruction reminding Sandy of the original task
4. The context window is doubled (`context_window_messages * 2`) so Sandy can see more prior work
5. Sandy resumes with a fresh iteration budget

**Resume triggers:** "continue", "keep going", "go on", "resume", "please continue", "carry on", "pick up where you left off", and phrases starting with "continue " or "resume ".

## Concurrency & Resilience Model

- **File locking:** `tracking.json` (6 write tools), `patterns.json` (3 write tools), and memory `.md` files (log_memory) are protected by static `Mutex<()>` locks (`TRACKING_LOCK`, `PATTERNS_LOCK`, `MEMORY_LOG_LOCK`). Lock is acquired before read, held through write. Read-only callers (e.g., `build_memory_summary`) don't need the lock — they read atomic snapshots from the last rename.
- **DB mutex poison recovery:** All `db.rs` methods use `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoned mutex instead of cascading panics.
- **Error visibility:** All DB write operations in `telegram.rs` (`save_session`, `store_message`) log errors via `tracing::error!` instead of silently discarding with `let _ =`.
- **LLM tool arg safety:** Malformed JSON from LLM SSE streams is caught explicitly, logged, and replaced with a `{"__parse_error": "..."}` object instead of silently becoming `null`.
- **Sub-agent message cap:** After 40 messages, old tool result content is truncated to 100 chars. Last 30 messages kept intact.
- **Pattern evidence cap:** Capped at 100 observations per pattern (oldest removed on overflow).
- **WAL checkpoint:** `PRAGMA wal_checkpoint(TRUNCATE)` runs on DB startup to prevent unbounded WAL growth.
- **Timezone validation:** Invalid timezone in config logs `tracing::warn!` before falling back to UTC (in `scheduler.rs` and `proactive.rs`).
- **Proactive schedule idempotency:** `ensure_for_chat` deletes existing proactive tasks before recreating, preventing duplicates from race conditions or rapid restarts.
- **Scheduler conversation awareness:** `build_context_aware_prompt()` checks last message timestamp. If conversation active (<5 min), tells LLM not to greet fresh. If recent (<30 min), tells LLM to keep it casual.

## Tech Debt & Fragilities

- `microclaw.db` and `sandy.db` both exist in runtime dir (legacy naming)
- `/storage` assumed to exist but doesn't on all deployments
- `include_dir!` embeds only 2 skills at compile time; rest are runtime-only
- Memory category files (`insights.md`, etc.) are plain markdown, not structured
- Conversation logs: `scripts/cleanup_conversations.sh` runs nightly via cron (`0 2 * * *`), archives `.md` files >7 days to `archive/`, deletes archives >30 days, checkpoints WAL. Does NOT trim `exec_log.jsonl`/`activity_log.json` or VACUUM while Sandy runs (corruption/lock risk)
- `patterns.json` schema has been reworked multiple times; old entries may have stale fields
- WhatsApp and Discord integrations are experimental/incomplete
- `scripts/auto-update.sh` is legacy (infinite loop daemon); replaced by `scripts/sandy-updater.sh` (single-run for systemd timer)
- `sandy-updater.service` references `sandy-updater.sh` — this file must exist or the timer silently fails

## Known Fragilities (from red-team analysis)

### FIXED — Concurrent file access
- `tracking.json`: `TRACKING_LOCK` static Mutex in `tracking.rs` wraps all 6 read-modify-write tools
- `patterns.json`: `PATTERNS_LOCK` static Mutex in `patterns.rs` wraps all 3 read-modify-write tools
- Memory `.md` files: `MEMORY_LOG_LOCK` static Mutex in `memory_log.rs` wraps read-append-write
- Session save race: still exists (scheduler vs user messages) — session is per-chat SQLite row, hard to lock without major refactor

### FIXED — Silent error swallowing
- `telegram.rs`: All 5 `let _ =` on `save_session`/`store_message` replaced with `tracing::error!` logging
- `llm.rs`: `unwrap_or_default()` on tool JSON replaced with explicit error handling + `__parse_error` field
- `db.rs`: All `.lock().unwrap()` replaced with `.lock().unwrap_or_else(|e| e.into_inner())` for poison recovery

### FIXED — Resource growth
- Sub-agent messages: compacted after 40 messages (old tool results truncated to 100 chars, last 30 kept intact)
- WAL file: `PRAGMA wal_checkpoint(TRUNCATE)` runs on startup
- Pattern evidence: capped at 100 per pattern (oldest removed on overflow)

### REMAINING
- Memory entry splitting uses `"\n## "` delimiter — breaks if entry body contains markdown headers
- Duplicate proactive schedules: FIXED — `ensure_for_chat` now deletes existing proactive tasks before recreating (idempotent)
- Timezone fallback: FIXED — now logs `tracing::warn!` before falling back to UTC
- **Model auto-discovery (partial):** `llm.rs` queries OpenRouter `/models` when all configured models fail, but only as a last resort. Could be improved: (1) cache the discovered model for the session instead of re-querying each time, (2) persist discovered model to config so it survives restarts, (3) notify the user via Telegram when a fallback model is being used, (4) handle 402 (out of credits) distinctly from 404 (model gone) — currently auto-discovery triggers on both but can't fix a billing issue

## Guardrails System

Sandy's system prompt now includes **tool whitelists and validation rules** loaded from `storage/guardrails.json` at runtime.

### How It Works

1. On startup, `telegram.rs` loads `storage/guardrails.json`
2. Injects its content into the system prompt BEFORE the LLM sees any user messages
3. Claude sees explicit allowed/forbidden tool lists per agent role
4. Tool calls that violate rules are caught early

### Guardrail File Structure

```json
{
  "version": "1.0",
  "updated": "2025-03-05",
  "rules": [
    {
      "role": "Sandy (orchestrator)",
      "allowed_tools": [...],
      "forbidden_tools": [...],
      "enforcement": "hard",
      "rationale": "..."
    }
  ],
  "global_constraints": [...]
}
```

### Guardrail Types

- **Hard enforcement:** System blocks violating tool calls with error
- **Soft enforcement:** Claude sees the rule but no runtime block (used for design principles, not technical restrictions)

### Editing Guardrails

To add/change rules:

1. Edit `/mnt/storage/guardrails.json` (or `storage/guardrails.json` in repo)
2. Update `"updated"` timestamp
3. Restart Sandy OR just let it reload on next user message (prompt is rebuilt per-message)
4. Commit changes if modifying repo copy

**Never edit rules directly in telegram.rs** — the JSON file is the single source of truth.

### Current Guardrails

- Sandy CANNOT use `web_search`, `web_fetch`, `browser` (must delegate to Zilla)
- Zilla (research agent) can ONLY use web tools + file ops
- Gonza (writer) can ONLY use file ops (no web access)
- Solution logs MUST include verification proof
- Memory logs MUST be specific (>30 chars, no vague statements)

