# Implementation Plan: Sandy Skill System Adaptation

## Executive Summary

Three skills analyzed. Recommendation: **Implement adapted versions of Skill Creator and Agent Config**. Agent Council (multi-agent system) is not suitable for Sandy's single-user ADHD coaching architecture.

---

## 1. SKILL CREATOR → "Sandy Skill Builder"

### Original Purpose
Guides creation of modular skills with scripts, references, and assets for extending AI capabilities.

### Sandy Adaptation
**Create a simplified skill builder that:**
- Helps you design custom skills for your specific ADHD workflows
- Creates skills in `/mnt/storage/skills/` (accessible from Mac)
- Stores medication trackers, routine checklists, energy pattern tools, etc.
- Uses Sandy's existing bash/file tools (no new infrastructure needed)

### Key Adaptations

**Simplified Structure:**
```
/mnt/storage/skills/
├── medication-tracker/
│   └── SKILL.md (instructions + simple tracking)
├── morning-routine/
│   └── SKILL.md (step-by-step morning routine)
├── energy-patterns/
│   └── SKILL.md (energy management guidance)
└── focus-techniques/
    └── SKILL.md (ADHD focus strategies)
```

**No Infrastructure Required:**
- ❌ No packaging scripts
- ❌ No validation tools
- ❌ No references/ or assets/ folders (keep it simple)
- ✅ Just SKILL.md files with markdown instructions
- ✅ Sandy reads them when you say "use my medication skill"

### Implementation Approach

**New Tool:** `create_skill`
- Input: skill name, description, content
- Output: Creates `/mnt/storage/skills/{name}/SKILL.md`
- Example: "Create a skill for tracking my ADHD medication"

**Usage:**
```
You: "Create a skill for my morning routine"
Sandy: "What's your ideal morning routine?"
You: [describe routine]
Sandy: [creates skill with step-by-step instructions]

Later: "Use my morning routine skill" 
→ Sandy loads it and guides you through
```

---

## 2. AGENT CONFIG → "Sandy Configuration Assistant"

### Original Purpose
Intelligently modify agent core files (AGENTS.md, SOUL.md, etc.) with validation and size checking.

### Sandy Adaptation
**Extend the existing Self-Review system to:**
- Allow Sandy to suggest improvements to her own configuration
- Help you understand and modify her behavior files
- Always requires your approval (Review Mode)
- Focus on files that actually matter for ADHD coaching

### Key Adaptations

**Relevant Files Only:**
- ✅ `soul/SOUL.md` - Sandy's personality and ADHD expertise
- ✅ `soul/AGENTS.md` - System capabilities and instructions
- ✅ `soul/IDENTITY.md` - Name, emoji, presentation
- ❌ `USER.md` - Not needed (Sandy tracks patterns in patterns.json)
- ❌ `TOOLS.md` - Not needed (tools are code, not config)
- ❌ `MEMORY.md` - Not needed (patterns.json handles this)
- ❌ `HEARTBEAT.md` - Not needed (scheduler handles this)

**Simplified Workflow:**
1. **Identify Target:** Which file needs updating?
2. **Check Current:** Read the file to understand current state
3. **Propose Change:** Present suggestion to you
4. **Get Approval:** Wait for your "yes"
5. **Apply Change:** Make the edit
6. **Document:** Log the change

### Implementation Approach

**Extend existing self-review skill:**
- Add "Propose Configuration Change" capability
- Sandy can say: "I notice I haven't been referencing your sleep patterns. Should I update AGENTS.md to check patterns.json for sleep-related issues?"
- You approve → Sandy edits the file

**New Commands:**
- `"Show me SOUL.md"` - Sandy displays her personality file
- `"Update my patterns reference"` - Propose AGENTS.md change
- `"Change your tone to be more direct"` - Propose SOUL.md change

---

## 3. AGENT COUNCIL → **NOT RECOMMENDED**

### Original Purpose
Create and manage multiple autonomous AI agents with Discord integration and agent coordination.

### Why Not Suitable for Sandy

**Architectural Mismatch:**
- Sandy is a **single-user, single-agent system** for personal ADHD coaching
- Multi-agent coordination adds complexity without benefit for one person
- Discord integration not relevant (Sandy uses Telegram)

**No Use Case:**
- You don't need a "research agent" + "health agent" + "finance agent"
- Sandy already handles all these domains with her existing tools
- Creating separate agents would fragment your ADHD management

**Infrastructure Burden:**
- Requires gateway configuration
- Requires Discord bot setup
- Requires workspace management for each agent
- Adds maintenance overhead

### Recommendation
**Skip entirely.** Sandy's existing architecture (single agent with skills and tools) is optimal for personal ADHD coaching.

---

## Implementation Priority

### Phase 1: Sandy Skill Builder (High Value)
**Timeline:** 1-2 hours to implement
**Value:** High - lets you create custom ADHD workflows

**What you'll be able to do:**
- "Create a skill for tracking my daily energy levels"
- "Create a skill for my evening wind-down routine"
- "Create a skill for managing my medication schedule"
- "Use my medication skill" → Sandy loads and guides you

**Files to Create:**
1. `soul/data/skills/sandy-skill-builder/SKILL.md` - Instructions for Sandy
2. New tool: `create_skill` - Tool to create skill files
3. Update `AGENTS.md` - Document the skill builder capability

### Phase 2: Configuration Assistant (Medium Value)
**Timeline:** 2-3 hours to implement
**Value:** Medium - extends existing self-review system

**What you'll be able to do:**
- Sandy proposes behavior improvements during self-review
- You can ask Sandy to show/modify her personality files
- Changes are tracked and reversible

**Files to Create:**
1. Extend `soul/data/skills/sandy-evolver/SKILL.md` - Add config change capability
2. New tool: `propose_config_change` - Tool to suggest edits
3. Update `AGENTS.md` - Document configuration management

---

## Questions for You

### About Skill Builder:
1. **What ADHD workflows do you want to create skills for?**
   - Medication tracking?
   - Morning routines?
   - Energy management?
   - Focus techniques?

2. **Where should skills be stored?**
   - `/mnt/storage/skills/` (accessible from Mac)?
   - Or `soul/data/skills/custom/` (with built-in skills)?

### About Configuration Assistant:
3. **Do you want Sandy to be able to modify her own personality (SOUL.md)?**
   - Or only operational instructions (AGENTS.md)?

4. **Should configuration changes be part of daily self-review, or a separate system?**

---

## Recommended Next Steps

1. **Approve the plan** - Confirm which components to implement
2. **Prioritize Skill Builder** - Higher immediate value for ADHD management
3. **Define first skill** - What custom workflow do you want to create first?
4. **Implement and test** - Create the skill builder and test with real use case

---

## Technical Notes

**No New Infrastructure Needed:**
- Uses existing bash/file/memory tools
- Stores skills in filesystem (accessible from Mac)
- Leverages existing skill loading system
- No packaging, validation, or distribution needed

**Backwards Compatible:**
- Doesn't break existing functionality
- Built-in skills continue working
- New skills live alongside existing ones

**Mac Integration:**
- Skills stored in `/mnt/storage/` appear on your Mac
- You can edit skill files directly if needed
- Git can track skill changes if desired
