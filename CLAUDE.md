# CLAUDE.md - Sandy Development Guide

## What Is Sandy?

Sandy is an ADHD coach and personal assistant Telegram bot built in Rust. She helps neurodivergent users manage tasks, track goals, learn behavioral patterns, and stay accountable. Sandy is an **orchestrator** — she delegates research/writing to specialized agents (Zilla, Gonza) and handles coaching, pattern analysis, and task management directly.

Runs on a Raspberry Pi 5 at `/home/jens/sandy`.

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
| `memory.rs` | Memory manager: loads AGENTS.md, insights, solutions, patterns, errors into context |
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
| `context_guard.rs` | Context window size management |
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
- `data_dir`: defaults to XDG path (`~/.local/share/sandy`), NOT `./sandy.data`
- `working_dir`: defaults to `/tmp/sandy_work`, NOT `./tmp`
- `telegram_token`, `claude_api_key`, `openai_api_key`

### Deploy

```bash
sudo systemctl restart sandy
journalctl -u sandy -f        # Follow logs
```

Auto-updater: systemd timer (`scripts/sandy-updater.timer`) pulls, builds, and restarts every 5 minutes.

### Key Conventions

- **Atomic file I/O:** Always use `atomic_io::write_atomic()` for data files, never raw `fs::write()`
- **Auth key:** Tests must use `"__sandy_auth"` (renamed from `__microclaw_auth`)
- **Path security:** All file tools validate paths against allowed roots before I/O
- **Tool filter:** Sub-agents use `ToolRegistry::new_sub_agent()` with restricted tool lists
- **Memory verification:** Solutions logged to memory must include `verification` field with proof
- **No direct web access:** Sandy delegates research to Zilla agent, writing to Gonza agent

## Known Test Failures

**None.** All 535 tests pass (475 lib + 60 integration/doc).

## Security Model — Memory System

- **Prompt injection prevention:** All memory content (AGENTS.md, insights, solutions, patterns, errors, rules) is XML-escaped via `sanitize_xml()` before injection into system prompts (`src/memory.rs`)
- **Fail-closed auth:** `authorize_chat_access()` and all write tools (memory, patterns, tracking, memory_log) require `__sandy_auth` context. Missing auth = denied.
- **Auth on write tools:** `create_pattern`, `add_observation`, `update_hypothesis`, `create_goal`, `create_project`, `create_task`, `update_status`, `add_note`, `remove_note`, `log_memory` all require auth
- **Confidence lock protection:** Only control chats can set `confidence_locked = true` on patterns
- **Hook injection escaping:** Memory context injected into sub-agent tasks via `MemoryInjectHook` is XML-escaped
- **Input validation:** Memory writes reject content >5000 chars, pattern fields >1000 chars, notes >2000 chars. Content containing XML closing tags matching prompt structure (e.g., `</recent_solutions>`) is rejected.

## Recent Changes

### Scheduler Conversation Awareness
- `scheduler.rs`: `build_context_aware_prompt()` checks last message timestamp before firing tasks. If conversation active (<5 min), prepends context note telling LLM not to greet. If recent (<30 min), tells LLM to keep it casual.
- `db.rs`: Added `get_last_message_timestamp(chat_id)` method.

### Soul File Formatting (No Asterisks)
- `soul/SOUL.md`, `soul/AGENTS.md`, `soul/IDENTITY.md`: Removed all `**bold**` and `_italic_` markdown formatting. Replaced with ALL CAPS headers, plain text, [Brackets] for emphasis, dashes for lists. Matches formatting rules in `rules.md`.

### Memory Security (previous session)
- `memory.rs`: All memory content XML-escaped via `sanitize_xml()` before prompt injection
- `tools/mod.rs`: `authorize_chat_access()` is fail-closed (missing auth = denied)
- `tools/patterns.rs`, `tracking.rs`, `memory_log.rs`: Auth required on all write tools
- `tools/patterns.rs`: Confidence lock requires control chat
- `hooks/memory_inject.rs`: Memory context XML-escaped before sub-agent injection
- Input validation: max length limits + forbidden XML tag rejection on memory/pattern/tracking writes

## Tech Debt & Fragilities

- `microclaw.db` and `sandy.db` both exist in runtime dir (legacy naming)
- `/storage` assumed to exist but doesn't on all deployments
- `include_dir!` embeds only 2 skills at compile time; rest are runtime-only
- Memory category files (`insights.md`, etc.) are plain markdown, not structured
- Conversation logs accumulate without rotation
- `patterns.json` schema has been reworked multiple times; old entries may have stale fields
- WhatsApp and Discord integrations are experimental/incomplete

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
