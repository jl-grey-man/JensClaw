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

## Known Test Failures (18 tests, 6 categories)

### A. Auth key rename (`__microclaw_auth` → `__sandy_auth`) — 8 tests

Tests still insert `__microclaw_auth` into context. `auth_context_from_input()` returns `None`, authorization is silently skipped, "permission denied" never fires.

- `test_export_chat_permission_denied` (`src/tools/export_chat.rs`)
- `test_read_memory_chat_permission_denied` (`src/tools/memory.rs`)
- `test_write_memory_global_denied_for_non_control_chat` (`src/tools/memory.rs`)
- `test_send_message_permission_denied_before_network` (`src/tools/send_message.rs`)
- `test_pause_task_permission_denied_cross_chat` (`src/tools/schedule.rs`)
- `test_schedule_task_permission_denied_cross_chat` (`src/tools/schedule.rs`)
- `test_todo_read_permission_denied` (`src/tools/todo.rs`)
- `test_todo_write_permission_denied` (`src/tools/todo.rs`)

**Fix:** Find-replace `__microclaw_auth` → `__sandy_auth` in test code.

### B. Config default changes not reflected in tests — 2 tests

`default_data_dir()` changed from `"./sandy.data"` to XDG `~/.local/share/sandy`. `default_working_dir()` changed from `"./tmp"` to `/tmp/sandy_work`.

- `test_config_yaml_defaults` (`src/config.rs`)
- `test_post_deserialize_empty_working_dir_uses_default` (`src/config.rs`)

**Fix:** Update assertions to match current defaults.

### C. Missing embedded skills — 2 tests

`src/skills/` only has `journalistic-research/` and `journalistic-writing/`. Tests expect `weather/` and `pdf/` to be embedded at compile time.

- `test_ensure_builtin_skills_includes_new_macos_and_weather_skills` (`src/builtin_skills.rs`)
- `test_ensure_builtin_skills_writes_missing_files` (`src/builtin_skills.rs`)

**Fix:** Copy needed skills into `src/skills/` or update tests to check only existing embedded skills.

### D. `/storage` directory doesn't exist on Pi — 2 tests

`ALLOWED_ROOTS` includes `/storage` but no such directory exists. `canonicalize()` fails.

- `test_validate_path_allowed` (`src/tools/file_ops.rs`)
- `test_safe_join` (`src/tools/file_ops.rs`)

**Fix:** Create `/storage` on the Pi, or change tests to use `/tmp` or `/mnt/storage`.

### E. Path validation blocks before file-not-found — 1 test

`test_read_file_not_found` uses `/nonexistent/file.txt` which fails path validation before the read.

- `test_read_file_not_found` (`src/tools/read_file.rs`)

**Fix:** Use a path inside an allowed root, or update the expected error.

### F. Write to non-existent temp paths — 2 tests

`canonicalize()` fails on paths whose parent dirs don't exist yet.

- `test_write_file_creates_parent_dirs` (`src/tools/write_file.rs`)
- `test_write_file_resolves_relative_to_working_dir` (`src/tools/write_file.rs`)

**Fix:** Loosen `validate_path` to handle not-yet-created parent dirs, or create dirs before validation.

### G. Scheduled task timestamp parsing — 1 test

Expected error `"Invalid ISO 8601"` but natural language datetime parsing changed the error path.

- `test_schedule_task_invalid_once_timestamp` (`src/tools/schedule.rs`)

**Fix:** Update expected error message or test input.

## Tech Debt & Fragilities

- `microclaw.db` and `sandy.db` both exist in runtime dir (legacy naming)
- `/storage` assumed to exist but doesn't on all deployments
- `include_dir!` embeds only 2 skills at compile time; rest are runtime-only
- Memory category files (`insights.md`, etc.) are plain markdown, not structured
- Conversation logs accumulate without rotation
- `patterns.json` schema has been reworked multiple times; old entries may have stale fields
- WhatsApp and Discord integrations are experimental/incomplete
