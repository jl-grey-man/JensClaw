# Sandy - ADHD Coach

> ⚠️ **IMPORTANT:** Read the document **architecture.md** before implementing any new features. It defines the Hard Rails architecture and implementation standards.

> ⚠️ **IMPORTANT:** These documents must be updated at the end of each major task or feature implementation. This means: if you are implementing a big change, you need to stop whenever one sub-feature is implemented and update these documents before continuing. 

> ⚠️ **IMPORTANT:** Read the document AI-RULES.MD and follow the instructions. 

## Project Overview

**Sandy** is an AI-powered ADHD coach and accountability partner built as a Telegram bot. She helps neurodivergent users manage their lives, understand their patterns, and build systems that work for their brains.

**Key Differentiator:** Unlike generic task managers, Sandy learns your ADHD patterns over time and provides personalized, ADHD-aware support. She doesn't judge, doesn't use toxic positivity, and understands executive dysfunction.

## Architecture

```
User Message → Telegram Bot → Sandy (Rust/JensClaw fork)
                                    ↓
                         LLM (OpenRouter + Claude Sonnet 4.5)
                                    ↓
                         Tools (tracking, patterns, reminders)
                                    ↓
                         File-based Storage (JSON)
                                    ↓
                         Web Dashboard + Activity Log
```

### Tech Stack
- **Language:** Rust (JensClaw fork of JensGrey's MicroClaw)
- **Database:** SQLite for scheduled tasks, JSON files for user data
- **LLM:** OpenRouter with Claude Sonnet 4.5
- **Interfaces:** Telegram (primary), Web UI (dashboard)
- **Storage:** File-based (OpenClaw style) - SOUL.md, AGENTS.md, patterns.json, tracking.json

## Project Structure

```
SandyNew/
├── PROJECT.md           ← You are here (main documentation)
├── .opencode/plans/IMPLEMENTATION_PLAN.md  ← Development roadmap and rebuild plan
├── AI-RULES.md          ← Development standards and protocols
├── architecture.md      ← Hard Rails architecture specification
├── TOOLS.md             ← Definitive tool reference (The Constitution)
├── QUICK_DEPLOY.md      ← Quick start deployment guide
├── config/
│   └── sandy.config.yaml    # Bot configuration
├── prompts/
│   └── guard_rails.txt      # DNA injected into all spawned agents
├── soul/               # OpenClaw identity system
│   ├── SOUL.md        # Sandy's personality + ADHD expertise
│   ├── AGENTS.md      # System capabilities & instructions
│   ├── IDENTITY.md    # Name, emoji, presentation
│   ├── data/
│   │   ├── patterns.json      # 18 ADHD pattern categories
│   │   ├── tracking.json      # Goals/Projects/Tasks/Reminders
│   │   ├── activity_log.json  # All actions logged
│   │   └── MEMORY.md          # Runtime memory
│   └── skills/        # Built-in and custom skills
│       └── custom/    # User-created skills
├── storage/           # Hard Rails: The Truth (persistent storage)
│   ├── agents/        # Agent JSON configurations
│   │   ├── zilla.json       # Research agent
│   │   └── gonza.json       # Writer agent
│   ├── tasks/         # Job workspaces
│   │   └── job_*/     # Individual job folders
│   │       ├── instructions.md
│   │       ├── raw_data.md
│   │       └── final_output.md
│   └── memory/        # Long-term data
│       ├── projects/
│       ├── todos/
│       └── logs/
│       ├── builtin/           # Core skills (documents, etc.)
│       └── custom/            # User-created skills
├── src/                # Rust source code
│   ├── main.rs        # Entry point
│   ├── lib.rs         # Module exports
│   ├── telegram.rs    # Telegram bot handler
│   ├── web/mod.rs     # Web dashboard API
│   ├── scheduler.rs   # Task scheduler
│   ├── activity.rs    # Activity logging
│   └── tools/         # All tool implementations
│       ├── tracking.rs        # Goals/Projects/Tasks/Reminders
│       ├── patterns.rs        # Pattern learning
│       ├── schedule.rs        # Reminder scheduling
│       ├── create_skill.rs    # Create custom skills
│       ├── agent_management.rs # Spawn/manage agents
│       └── ...
├── static/
│   └── index.html     # Web dashboard UI
├── old/               # Archived documentation
└── target/            # Build output
```

## Core Features

### 1. ADHD-Focused Coaching (SOUL.md)
Sandy's personality is defined in `soul/SOUL.md`:
- **Archetype:** Rachel Zane (Suits) meets Joan Holloway (Mad Men)
- **Style:** Warm but direct, confident, doesn't coddle
- **Expertise:** Deep knowledge of ADHD (executive dysfunction, time blindness, RSD, etc.)
- **Approach:** "Interest-based motivation" not "importance-based"

### 2. Pattern Learning System (18 Categories)
Located in `soul/data/patterns.json`:

Initial categories include:
- Procrastination, Focus, Energy Management
- Time Perception, Task Initiation, Motivation
- Environmental Factors, Stress Response, Social Patterns
- Sleep & Routine, Emotional Regulation, Decision Making
- Transitions, Sensory Preferences, Hyperfocus
- Accountability, Learning Style, Rejection Sensitivity

**Capabilities:**
- Record observations via `add_observation` tool
- Update hypotheses via `update_hypothesis` tool
- Create new patterns dynamically via `create_pattern` tool
- Track confidence levels (0-100%)

### 3. Unified Tracking System
Located in `soul/data/tracking.json`:

**Hierarchy:**
```
Goal (big outcome)
  └── Project (path to goal)
        └── Tasks (individual actions)
  └── Reminders (time-based nudges)
```

**Tools:**
- `create_goal` - New goal
- `create_project` - Create project (optionally linked to goal)
- `create_task` - Create task (optionally linked to project/goal)
- `update_status` - Mark complete/in-progress/todo
- `add_note` - Add context notes (auto-timestamped)
- `remove_note` - Remove specific note by index or clear all

### 4. Smart Reminder System
**Flexible Time Parsing:**
- "in X minutes/hours/days" (e.g., "in 5m", "in 2 hours")
- Day names: "Monday", "Tuesday", etc. (next occurrence)
- Time keywords: "morning" (9am), "afternoon" (2pm), "evening" (6pm), "tonight" (8pm)
- "tomorrow at HH:MM"
- "today at HH:MM"

**Strict Validation:** If Sandy doesn't understand the time format, she asks for clarification instead of guessing wrong.

### 5. Activity Logging
Located in `soul/data/runtime/activity_log.json`:
- Records every action (create, update, delete)
- Shows in Web UI with timestamps
- Auto-refreshes every 5 seconds
- Keeps last 1000 activities

### 6. Web Dashboard
Accessible at `http://localhost:3000`:
- **Stats Overview:** Active goals, tasks todo/in-progress/completed
- **Activity Log:** Real-time feed of all actions
- **Dropdown Lists:** Goals, Projects, Tasks, Patterns, Reminders
- **Clickable Items:** View full details including notes
- **Auto-refresh:** Updates every 5 seconds

### 7. Document Management System
Full file management in `/mnt/storage`:
- **Create files:** Markdown, Text, HTML, CSS, JS, JSON, Python
- **Read & update:** View and modify existing files
- **Organize:** Create subdirectories, move files
- **Access:** All files accessible from Mac (shared folder)
- **Skill:** Uses `soul/data/skills/documents/SKILL.md`

**Supported Operations:**
- Create notes, reports, lists, code files
- Build websites (HTML/CSS/JS)
- Write Python scripts for automation
- Organize files into logical folders

### 8. Self-Review System (Daily Improvement)
Automatic daily analysis (3 AM) of Sandy's coaching effectiveness:
- **Analyzes:** Conversation quality, goal support, pattern usage
- **Suggests:** Improvements to better help with ADHD
- **Review Mode:** All changes require your explicit "yes"
- **Never Autonomous:** Sandy cannot modify herself without approval
- **Skill:** Uses `soul/data/skills/sandy-evolver/SKILL.md`

**Key Safety Rules:**
- No autonomous changes - you approve every suggestion
- No code modifications - cannot edit Rust source
- No memory deletion - cannot remove learned patterns
- Full transparency - all analysis shared before action

### 9. Skill Builder System
Create custom skills for reusable workflows:
- **Tool:** `create_skill` - Programmatically create skills
- **Location:** `soul/data/skills/custom/`
- **Usage:** "Create a skill for my morning routine"
- **Activation:** "Use my [skill-name] skill"

**Examples:**
- Morning routine guides
- Medication tracking workflows
- Research assistant methodology
- File organization procedures
- Focus technique guides

### 10. Agent System [IN DEVELOPMENT]

⚠️ **Current Status:** Infrastructure exists but execution engine not yet implemented.

**The Vision:** Spawn specialized background agents while continuing conversation:
- Research agents for web searches
- Code agents for script writing
- File agents for organization
- Sequential workflows (Agent A → verify → Agent B)

**Current Reality:**
- spawn_agent exists but only creates registry entries (doesn't execute)
- list_agents tracks state but agents don't perform work
- The system is being rebuilt following the Hard Rails architecture

**What Works NOW:**
The `sub_agent` tool is functional - it spawns real LLM subprocesses that execute tasks with restricted tool sets.

**When Available:**
See IMPLEMENTATION_PLAN.md Phase 4 for rebuild timeline (estimated 6-8 hours of work).

**Example of Planned Behavior:**
```
You: "Research ADHD sleep strategies"
Sandy: Spawns research agent [agent executes web_search, writes to file]
[Verification: File exists and has content]
Sandy: Spawns writer agent [transforms research into article]
[Verification: Article file exists]
Sandy: "Done. Files: research.json (2.4KB), article.md (1.8KB)"
```

## Key Design Principles

### 1. Natural Language Interface
Sandy understands natural language - no rigid commands needed:
- "I need to finish the website by Friday" → Creates goal/project/tasks
- "I always struggle with morning tasks" → Records pattern observation
- "Remind me Monday to submit report" → Schedules reminder

### 2. File-Based Simplicity
Unlike the old Python Sandy with PostgreSQL + Pinecone, this version uses:
- JSON files for data storage
- Human-readable and editable
- Version controllable (git-friendly)
- No complex database setup required

### 3. Learning & Improvement
Sandy improves over time by:
- Collecting observations about user behavior
- Updating confidence in various hypotheses
- Creating new patterns when encountering new behaviors
- Applying learned knowledge to give better suggestions

### 4. ADHD-Specific Design
- **No shame/guilt:** Frame "failures" as data for learning
- **Body doubling:** Offers accountability without judgment
- **Micro-steps:** Break down overwhelming tasks
- **Externalize working memory:** Visual tracking, reminders
- **Energy-aware:** Track and use energy patterns

## Important Documents

### Required Reading for New Developers
1. **PROJECT.md** (this file) - Overall project overview
2. **IMPLEMENTATION_PLAN.md** (`.opencode/plans/`) - Development roadmap, current status, and rebuild phases

### Reference Documents
- **soul/SOUL.md** - Sandy's personality and ADHD expertise
- **soul/AGENTS.md** - System capabilities and tool usage
- **old/** - Archived documentation from previous iterations

## Development Workflow

### Building & Running
```bash
cd /Users/jenslennartsson/Documents/-ai_projects-/SandyNew
cargo run -- start
```

### Configuration
Edit `microclaw.config.yaml`:
- Telegram bot token
- OpenRouter API key
- Timezone settings
- Web UI port

### Testing
1. Start the bot
2. Send messages to Telegram bot
3. Check Web UI at http://localhost:3000
4. Verify activity log updates

### Adding Features
1. Implement in `src/tools/` or `src/`
2. Register in `src/tools/mod.rs`
3. Update SOUL.md if affecting personality
4. Update AGENTS.md if adding capabilities
5. **Update PROJECT.md and IMPLEMENTATION_PLAN.md**

## Current Capabilities

### What's Working ✅
- **Telegram interface** with Sandy's personality
- **Pattern learning** (manual via tools + automatic via self-review)
- **Goal/Project/Task tracking** with notes and hierarchy
- **Reminder scheduling** with flexible natural language parsing
- **Web UI dashboard** with real-time activity feed
- **Document management** - create/edit files in `/mnt/storage`
- **Daily self-review** - automatic improvement analysis with user approval
- **Skill builder** - create custom workflows and procedures
- **Sub-agent execution** - spawn background LLM subprocesses for tasks
- **HELP command** - comprehensive help via Telegram
- **Config flexibility** - supports `sandy.config.yaml` and legacy `microclaw.config.yaml`

### What's Placeholder / In Development 🔄
- **Agent delegation system** - Infrastructure exists but execution engine not yet implemented. spawn_agent creates registry entries only, never executes actual work. See IMPLEMENTATION_PLAN.md Phase 4 for rebuild status.

### What's In Progress 🔄
- **Enhanced self-review** - More sophisticated analysis and suggestions
- **Proactive Sandy** (unprompted check-ins based on patterns)
- **Agent persistence** - Option to keep agents with accumulated knowledge

### Known Issues ⚠️
- ~~**OpenRouter API Error** - "Provider returned error"~~ **FIXED** - Upgraded model from `anthropic/claude-3.5-sonnet` to `anthropic/claude-sonnet-4.5`, added error detection for HTTP 200 error responses, improved error logging
- Reminders stored in both DB and JSON (sync risk, low for single-user)
- Web UI needs page refresh for some updates (5-second polling)
- No authentication on web UI (local use only)
- ~~Reminders scheduled for 2024 instead of current year~~ **FIXED**
- ~~Config file naming confusion~~ **FIXED** - Now supports `sandy.config.yaml`
- ~~Watchdog logs in wrong directory~~ **FIXED** - Now uses `~/sandy/logs/`

## Future Vision

### Completed Recently ✅
1. ✅ **Document management** - Full file operations in `/mnt/storage`
2. ✅ **Self-review system** - Daily automatic analysis with user approval
3. ✅ **Skill builder** - Create custom ADHD workflows and procedures
4. ✅ **Sub-agent execution** - Working LLM subprocess spawning for tasks
5. ✅ **Config flexibility** - Support for `sandy.config.yaml`
6. 🔄 **Agent delegation system** - Infrastructure built, execution engine in development (Phase 4)

### Short-term (Next 1-2 months)
1. Proactive reminders based on learned patterns
2. Enhanced agent capabilities (persistent agents, more types)
3. More built-in skills (medication tracker, focus techniques, etc.)
4. Multi-user support architecture
5. Integration with calendar apps (Google Calendar, etc.)

### Medium-term (3-6 months)
1. Web UI editing capabilities
2. Mobile-responsive dashboard improvements
3. Integration with calendar apps
4. Voice message support

### Long-term (6+ months)
1. Advanced pattern prediction
2. Personalized ADHD coaching strategies
3. Integration with wearable devices (energy tracking)
4. Community features (optional)

## Getting Started

### For New Developers
1. Read PROJECT.md (this file) ← You are here
2. Read IMPLEMENTATION_PLAN.md (`.opencode/plans/`) for current roadmap and implementation status
3. Review soul/SOUL.md to understand Sandy's personality
4. Review soul/AGENTS.md to understand capabilities
5. Run the bot: `cargo run -- start`
6. Test features via Telegram and Web UI

### For Users
1. Message @sandy_adhd_coach_bot on Telegram
2. Start with simple tasks: "I need to finish X by Friday"
3. Let Sandy learn your patterns over time
4. Check http://localhost:3000 for dashboard view
5. Be patient - Sandy learns and improves with use

## Contributing

### When Adding Features:
1. **Always update documentation** - PROJECT.md and IMPLEMENTATION_PLAN.md
2. Follow Rust conventions - see existing code in `src/`
3. Add appropriate tools to `src/tools/mod.rs`
4. Test thoroughly via Telegram and Web UI
5. Update SOUL.md if changing personality/behavior

### When Fixing Bugs:
1. Document the bug in IMPLEMENTATION_PLAN.md under Current Status section
2. Mark as completed when fixed
3. Note any breaking changes

## License
MIT License - See LICENSE file

## Contact
- Project: JensClaw fork of MicroClaw
- Original: https://github.com/jl-grey-man/JensClaw
- Status: Active development

---

**Last Updated:** February 12, 2026
- Upgraded model to Claude Sonnet 4.5 (`anthropic/claude-sonnet-4.5`)
- Fixed OpenRouter "Provider returned error" — improved error handling for HTTP 200 error responses
- Fixed reminders using wrong year (2024) — system prompt now includes current date/time
- Injected current UTC time into system prompt for accurate scheduling

**Next Review:** After next feature implementation
