# UPDATED Implementation Plan: Sandy Multi-Agent System

## User Clarification

- Sandy is both: **ADHD coach + general assistant**
- **No Discord** - all communication through Telegram
- User believes multi-agent coordination **can be useful**

---

## REVISED Recommendations

### Implement ALL THREE - Adapted for Sandy's Architecture

1. **Skill Creator** → Sandy Skill Builder ✅
2. **Agent Config** → Configuration Assistant ✅  
3. **Agent Council** → **Sandy Agent Delegation System** ✅ (REVISED)

---

## 3. AGENT COUNCIL → "Sandy Agent Delegation System"

### Reconsidered: Why It CAN Work

Even for single-user personal assistance, multi-agent delegation makes sense:

**Use Cases:**
- **Research Agent**: "Find me 5 articles about ADHD sleep strategies" (runs in background while Sandy continues chatting)
- **Code Agent**: "Write a Python script to organize my files" (executes while Sandy talks)
- **File Agent**: "Organize my downloads folder by file type" (background task)
- **Analysis Agent**: "Analyze my activity logs for patterns" (long-running analysis)

**Benefits:**
- Sandy stays responsive while heavy tasks run
- Specialized agents can have domain-specific knowledge
- Parallel processing for complex workflows
- Better context management (separate sessions)

---

### Sandy-Specific Adaptations

**Architecture:**
```
Sandy (Main Agent)
├── Telegram Interface
├── ADHD Coaching Expertise
├── Task Coordination
└── Can spawn sub-agents:
    ├── Research Agent (web search, data gathering)
    ├── Code Agent (scripting, automation)
    ├── File Agent (organization, cleanup)
    └── Analysis Agent (pattern recognition, reports)
```

**Key Differences from Original:**
- ❌ No Discord integration (use Telegram)
- ❌ No gateway config complexity
- ✅ Spawn sub-agents as background Telegram sessions
- ✅ Agents report back to Sandy (not separate channels)
- ✅ All communication through existing Telegram bot

**Agent Creation:**
- Store in `soul/agents/{agent-id}/`
- Each with: SOUL.md (specialty), AGENTS.md (capabilities)
- Spawn via tool: `spawn_agent`
- Monitor via tool: `list_agents`, `agent_status`

---

## UPDATED IMPLEMENTATION PRIORITY

### Phase 1: Skill Builder (Foundation)
**Timeline:** 2 hours
**Enables:** Custom ADHD workflows + general assistant workflows

**First skills you might create:**
- `adhd-morning-routine` - Step-by-step morning startup
- `medication-tracker` - ADHD medication logging
- `focus-session` - Pomodoro-style focus timer with ADHD breaks
- `file-organizer` - Downloads folder cleanup workflow
- `research-assistant` - Web search and summarization template

### Phase 2: Agent Delegation System
**Timeline:** 3-4 hours
**Enables:** Background task execution

**First agents to create:**
- `research-agent` - Deep web research, data gathering
- `code-agent` - Write scripts, automation tools
- `file-agent` - File organization, cleanup tasks
- `analysis-agent` - Pattern analysis from your activity logs

**Example usage:**
```
You: "Research the best note-taking apps for ADHD"
Sandy: "I'll have my research agent look into this. Continue chatting with me while they work on it."
[spawns research agent in background]

... 5 minutes later ...

Research Agent: "Completed research. Found: Obsidian, Notion, Roam Research. Summary: [results]"
Sandy: "Great! My research agent found these options. Based on your 'digital overwhelm' pattern, I'd recommend Obsidian for its local-first approach. Want me to help you set it up?"
```

### Phase 3: Configuration Assistant
**Timeline:** 2 hours
**Enables:** Self-improvement with your approval

**Integrated with daily self-review:**
- Sandy analyzes her performance
- Proposes improvements to her files
- You approve/reject each suggestion
- Tracks what works and what doesn't

---

## QUESTIONS FOR REFINED IMPLEMENTATION

### About Multi-Agent Delegation:

1. **How should agents report back?**
   - A) Agent sends message directly to you in Telegram
   - B) Agent reports to Sandy, Sandy summarizes to you
   - C) Both (detailed in logs, summary from Sandy)

2. **Agent persistence:**
   - A) Agents auto-delete after task completion (clean)
   - B) Agents stay alive, accumulate knowledge over time
   - C) User decides per task

3. **Agent awareness:**
   - A) You know agents exist and can address them directly
   - B) Sandy manages all agents transparently, you just talk to Sandy
   - C) Hybrid (Sandy manages, but you can say "ask the research agent")

4. **First use case:** What task would you delegate to a sub-agent first?
   - Web research on a topic?
   - Write a Python script?
   - Organize files?
   - Something else?

### About Skills:

5. **What are the first 2-3 skills you want to create?**

6. **Skill storage location:**
   - A) `/mnt/storage/skills/` (accessible from Mac)
   - B) `soul/data/skills/custom/` (with built-in skills)
   - C) Both locations (different purposes?)

---

## RECOMMENDED APPROACH

**Hybrid: Skills + Agents**

- **Skills**: Reusable workflows and knowledge (like templates)
- **Agents**: Background task execution (like assistants)

**Example:**
```
You: "Help me establish a morning routine"

Sandy: 
1. Loads "adhd-morning-routine" skill (pre-built workflow)
2. Guides you through creating your routine
3. Spawns "file-agent" to create tracking files in background
4. All while continuing conversation with you
```

---

## NEXT STEPS

1. **Answer the 6 questions above**
2. **I'll implement Phase 1 (Skill Builder)** - immediately useful
3. **Test with your first custom skill**
4. **Implement Phase 2 (Agent Delegation)** - adds background processing
5. **Implement Phase 3 (Config Assistant)** - enables self-improvement

---

## Technical Architecture (Telegram-Based)

```
┌─────────────────────────────────────┐
│         Telegram Chat                │
│            (You)                    │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│          Sandy (Main)             │
│   • ADHD Coaching                 │
│   • General Assistance            │
│   • Task Coordination             │
│   • Skill Management              │
└──────┬──────────────┬─────────────┘
       │              │
┌──────▼─────┐  ┌─────▼──────┐
│   Skills   │  │  Agents    │
│  (Static)  │  │ (Dynamic)  │
│            │  │            │
│• Morning   │  │• Research │
│  Routine   │  │• Code      │
│• Meds      │  │• Files     │
│• Focus     │  │• Analysis  │
└────────────┘  └────────────┘
```

All through one Telegram bot interface.
