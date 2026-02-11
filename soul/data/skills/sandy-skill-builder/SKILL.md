---
name: sandy-skill-builder
description: Create custom skills for Sandy - reusable workflows, templates, and procedures. Use when you want to create a new skill for ADHD management, productivity, research, coding, or any repetitive task. Skills are stored in soul/data/skills/custom/ and can be activated on demand.
license: MIT
---

# Sandy Skill Builder

Create custom skills to extend Sandy's capabilities with your own workflows, templates, and domain-specific knowledge.

## What Are Skills?

Skills are **reusable instruction sets** that Sandy can load and follow. Think of them as:
- Custom workflows (morning routine, medication tracker)
- Templates (how to research a topic, how to organize files)
- Domain knowledge (your specific needs and preferences)
- Procedures (step-by-step guides for complex tasks)

## Skill Structure

```
soul/data/skills/custom/
└── your-skill-name/
    └── SKILL.md          # Instructions and content
```

**SKILL.md format:**
```markdown
---
name: your-skill-name
description: What this skill does and when to use it
---

# Skill Title

## Purpose
What this skill helps with

## When to Use
- Trigger condition 1
- Trigger condition 2

## Instructions
Step-by-step guide or reference material

## Examples
Example usage:
- "Activate my morning routine skill"
- "Use the research assistant skill for ADHD sleep strategies"
```

## Creating a Skill

### Step 1: Define the Skill

Ask yourself:
- **What repetitive task** do I want help with?
- **When should Sandy** use this skill?
- **What instructions** does Sandy need?

### Step 2: Use the create_skill Tool

Sandy will:
1. Ask you to name the skill
2. Ask for the description (when to use it)
3. Ask for the content (instructions)
4. Create the SKILL.md file
5. Confirm creation

### Step 3: Test the Skill

Activate it:
- "Use my [skill-name] skill"
- "Activate skill [skill-name]"
- "Help me with [skill-name]"

## Examples of Useful Skills

### 1. Research Assistant
```markdown
---
name: research-assistant
description: Conduct web research and provide structured summaries. Use when user asks to research a topic, find information, or gather data.
---

# Research Assistant

## Purpose
Help user research topics with structured approach

## Research Process
1. **Clarify scope**: Ask user what specific aspects to research
2. **Search broadly**: Use web_search for initial information
3. **Deep dive**: Use web_fetch on most relevant sources
4. **Synthesize**: Create structured summary

## Output Format
- **Quick Summary** (2-3 sentences)
- **Key Findings** (bullet points)
- **Sources** (links with brief context)
- **Recommendations** (actionable next steps)

## When to Delegate to Research Agent
For complex research (>10 sources needed, analysis required):
- Spawn research-agent instead
- Agent works in background
- Reports back with comprehensive findings
```

### 2. Morning Routine
```markdown
---
name: morning-routine
description: Guide user through ADHD-friendly morning startup routine. Use when user says "start my morning routine" or mentions struggling with mornings.
---

# Morning Routine

## ADHD Considerations
- Body doubling: Stay with user throughout
- Micro-steps: Break each task into tiny actions
- No shame: If they get stuck, adjust, don't judge

## Routine Steps
1. **Check in** (1 min)
   - "Good morning. How did you sleep?"
   - Note energy level: High/Medium/Low

2. **Hydrate** (2 mins)
   - "Drink a glass of water"
   - Wait for confirmation

3. **Medication check** (1 min)
   - "Have you taken your medication?"
   - If no: "Want me to remind you in 15 minutes?"

4. **Choose focus task** (3 mins)
   - List today's top 3 priorities
   - Ask: "Which one feels most doable right now?"
   - Break chosen task into first 5-minute micro-step

5. **Body double** (25 mins)
   - "I'm here with you. Let's do that first micro-step together."
   - Check in every 5 minutes
   - Celebrate when done

## Emergency Protocol
If user is stuck/paralyzed:
- Lower bar: "Just sit up in bed. That's it."
- Smaller step: "Just open your laptop. Don't do anything else."
- Validation: "Executive dysfunction is real. Let's work with it, not against it."
```

### 3. File Organizer
```markdown
---
name: file-organizer
description: Organize files and folders using systematic approach. Use when user mentions messy files, wants to organize a folder, or needs help with digital clutter.
---

# File Organizer

## Before Starting
1. **Ask**: Which folder to organize?
2. **Ask**: What categories make sense for their workflow?
3. **Set expectations**: "This might take time. Want me to spawn a file-agent to work on this in background?"

## Organization Process
1. **Inventory**: List all files in target directory
2. **Categorize**: Sort into logical groups
   - By type: Documents, Images, Code, Archives
   - By project: Project A, Project B, Archive
   - By date: 2024-Q1, 2024-Q2, etc.
3. **Create structure**: Make directories
4. **Move files**: Organize into folders
5. **Document**: Create README in organized folder explaining structure

## For Large Jobs
Delegate to file-agent:
- Spawns agent in background
- Agent organizes files
- Reports back with summary
- User can continue chatting with Sandy

## Safety
- Never delete files (only organize)
- Create backups of critical folders
- Ask before moving large batches (>50 files)
```

## Skill Naming Guidelines

**Format:** lowercase-with-hyphens

**Good names:**
- `morning-routine`
- `research-assistant`
- `medication-tracker`
- `file-organizer`
- `focus-techniques`

**Bad names:**
- `Morning Routine` (has spaces)
- `my_skill` (underscores)
- `Skill1` (not descriptive)
- `Morning-Routine-Skill` (redundant)

## Activating Skills

**Commands:**
- "Use my [skill-name] skill"
- "Activate [skill-name]"
- "Help me with [skill-name]"
- "Load the [skill-name] skill"

**What happens:**
1. Sandy reads SKILL.md
2. Loads instructions into context
3. Guides user through skill workflow
4. Follows skill-specific procedures

## Managing Skills

**List all skills:**
```bash
ls -la soul/data/skills/custom/
```

**Delete a skill:**
```bash
rm -rf soul/data/skills/custom/[skill-name]/
```

**Edit a skill:**
Edit the SKILL.md file directly with your preferred editor

**Share skills:**
Skills are just text files - you can:
- Copy SKILL.md to share with others
- Version control with git
- Backup to cloud storage

## Best Practices

**Keep skills focused:**
- One skill = one workflow or domain
- Don't try to create "everything skill"
- Break complex workflows into multiple skills

**Write for your future self:**
- Be specific about your preferences
- Include your ADHD-specific needs
- Reference your learned patterns

**Test before relying:**
- Activate skill and test it
- Refine based on what works
- Update instructions as needed

**Iterate:**
- Skills can be updated anytime
- Keep what works, remove what doesn't
- Add new skills as new needs arise

## Examples for User

**You might create:**
- `evening-wind-down` - ADHD-friendly bedtime routine
- `meeting-prep` - How to prepare for meetings
- `email-zero` - Inbox management workflow
- `exercise-motivator` - Body doubling for workouts
- `project-breakdown` - Turning big projects into tasks
- `cooking-skill` - Simple meal prep guidance
- `travel-checklist` - Packing and trip planning

## Creating Your First Skill

Just tell Sandy: **"Create a skill"**

Sandy will guide you through:
1. Choosing a name
2. Writing a description
3. Defining the content
4. Testing it

Start with something simple for your first skill!
