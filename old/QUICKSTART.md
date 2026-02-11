# Sandy - ADHD Coach Quick Start Guide

## What Was Built

**Sandy** is an ADHD coach Telegram bot built on JensClaw (Rust). She helps neurodivergent users manage tasks, track goals, learn patterns, and stay accountable.

## Project Structure

```
SandyNew/
├── soul/                      # OpenClaw identity files
│   ├── SOUL.md               # Sandy's personality + ADHD expertise
│   ├── IDENTITY.md           # Name, emoji, presentation
│   ├── AGENTS.md             # System capabilities & instructions
│   └── data/
│       ├── patterns.json     # 18 ADHD pattern categories
│       ├── tracking.json     # Goals/Projects/Tasks/Reminders
│       └── MEMORY.md         # Runtime memory for Sandy
├── config/
│   └── sandy.config.yaml     # Configuration with OpenRouter
├── src/                       # Rust source code
│   ├── tools/
│   │   ├── patterns.rs       # Pattern learning tools
│   │   └── tracking.rs       # Tracking system tools
│   └── ...
└── target/
    └── release/
        └── microclaw         # Compiled binary
```

## New Tools Added

### Pattern Learning Tools
- `read_patterns` - View all 18 ADHD patterns with confidence scores
- `add_observation` - Record observations about user's behavior
- `update_hypothesis` - Update pattern hypotheses based on learning
- `create_pattern` - Create new pattern categories dynamically

### Tracking Tools
- `read_tracking` - View goals, projects, tasks, and reminders
- `create_goal` - Create new goals
- `create_project` - Create projects (optionally linked to goals)
- `create_task` - Create tasks (optionally linked to projects/goals)
- `update_status` - Mark items as complete/in-progress/todo

## Configuration

Edit `config/sandy.config.yaml`:

```yaml
# Telegram (already configured)
telegram_bot_token: "7730634323:AAEkPeHcLoNxvBi0Iqlrbk6uEKHDmEhfY_c"
bot_username: "sandy_adhd_coach_bot"

# LLM (OpenRouter with Claude 3.5 Sonnet)
llm_provider: "openrouter"
api_key: "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"
model: "anthropic/claude-3.5-sonnet"

# OpenClaw files (automatically loaded into system prompt)
soul_file: "./soul/SOUL.md"
identity_file: "./soul/IDENTITY.md"
agents_file: "./soul/AGENTS.md"
memory_file: "./soul/data/MEMORY.md"
```

## Running Sandy

### Development Mode
```bash
cd /Users/jenslennartsson/Documents/-ai_projects-/SandyNew
cargo run -- start
```

### Production Mode
```bash
cd /Users/jenslennartsson/Documents/-ai_projects-/SandyNew
./target/release/microclaw start
```

Or install globally:
```bash
cargo install --path .
microclaw start
```

## How Sandy Works

### 1. Natural Language Understanding
Sandy understands natural language for:
- Task creation: "I need to finish the report by Friday"
- Goal setting: "My goal is to exercise 3x per week"
- Pattern discovery: "I always struggle with morning tasks"

### 2. Pattern Learning System
Sandy tracks 18 ADHD pattern categories:
- Procrastination, Focus, Energy Management
- Time Perception, Task Initiation, Motivation
- Environmental Factors, Stress Response, Social Patterns
- Sleep & Routine, Emotional Regulation, Decision Making
- Transitions, Sensory Preferences, Hyperfocus
- Accountability, Learning Style, Rejection Sensitivity

As Sandy collects observations, confidence increases and hypotheses form.

### 3. Unified Tracking
```
Goal (big outcome)
  └── Project (path to goal)
        └── Tasks (individual actions)
  └── Reminders (time-based nudges)
```

### 4. ADHD-Specific Features
- Energy-aware task suggestions (tracked through patterns)
- Body doubling and accountability
- Breaking overwhelming tasks into micro-steps
- Externalizing working memory
- Interest-based (not importance-based) motivation
- Time blindness support with strategic reminders

## Key Differences from Old Sandy

| Feature | Old Sandy | New Sandy |
|---------|-----------|-----------|
| Language | Python | Rust |
| Database | PostgreSQL (17 tables) | SQLite (4 tables) + JSON files |
| Memory | Pinecone vectors + SQL | File-based (OpenClaw style) |
| AI Provider | Together.ai | OpenRouter (Claude 3.5 Sonnet) |
| Dashboard | WebSocket real-time | Not included (Telegram only) |
| Patterns | 18 categories, rigid schema | 18 categories + dynamic creation |
| Energy Tracking | Separate field | Via pattern observations |

## Testing

1. Start the bot: `cargo run -- start`
2. Send a message to @sandy_adhd_coach_bot on Telegram
3. Test pattern learning: "I always struggle with morning tasks"
4. Test tracking: "I need to finish the website redesign"
5. Test reminders: "Remind me to call Mom tomorrow at 3pm"

## File-Based Memory

Unlike the old system with complex Pinecone integration, Sandy uses simple files:
- **SOUL.md** - Who Sandy is (personality, values, ADHD expertise)
- **AGENTS.md** - What Sandy can do (tools, capabilities)
- **IDENTITY.md** - How Sandy presents herself
- **patterns.json** - Learned patterns about User
- **tracking.json** - Goals, projects, tasks, reminders
- **MEMORY.md** - Runtime memory and observations

These files are:
- Human-readable and editable
- Version controllable (git-friendly)
- No database setup required
- Persist across sessions naturally

## Next Steps

1. **Test the bot** - Send some messages and verify Sandy responds with her personality
2. **Verify pattern learning** - Make observations and see them recorded
3. **Test tracking** - Create goals, projects, and tasks
4. **Iterate on SOUL.md** - Adjust Sandy's personality as needed
5. **Add more patterns** - Let Sandy create new pattern categories as she learns

## Architecture

```
User Message → Telegram → Sandy Bot → OpenRouter (Claude 3.5 Sonnet)
                                    ↓
                              System Prompt (SOUL.md + AGENTS.md + IDENTITY.md)
                                    ↓
                              Tools (patterns.rs, tracking.rs, etc.)
                                    ↓
                              Files (patterns.json, tracking.json)
                                    ↓
                              Response → Telegram
```

## Troubleshooting

**Bot not responding:**
- Check Telegram bot token in config
- Verify OpenRouter API key
- Check logs: `cargo run -- start` shows errors

**Patterns not saving:**
- Check file permissions on soul/data/
- Verify patterns.json exists and is valid JSON

**Configuration issues:**
- Config file must be named sandy.config.yaml
- Located in config/ directory
- All required fields must be filled

## Multi-User Ready (Future)

Current implementation is single-user but built for expansion:
- User ID in file paths
- Separate pattern/tracking files per user
- Chat ID-based routing

To add multi-user support later:
1. Create user_id field in config
2. Add user_id parameter to all tools
3. Create soul/data/{user_id}/ directories
4. Route messages by chat_id to user profiles

---

**Sandy is ready to help you manage your ADHD!** 🧠
