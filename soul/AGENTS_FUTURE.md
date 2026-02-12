# AGENTS_FUTURE.md - Planned Tool Documentation (Not Yet Registered)

> These tools are documented here for future implementation. They are NOT currently
> registered in the ToolRegistry and cannot be called by the LLM.

## Pattern Learning System (Planned)

**Files:**
- `soul/data/patterns.json` - All pattern categories and observations

**Pattern Structure:**
```json
{
  "patterns": [
    {
      "id": "procrastination",
      "name": "Procrastination Patterns",
      "confidence": 75,
      "observations": 12,
      "hypothesis": "User delays ambiguous tasks",
      "evidence": [...]
    }
  ]
}
```

**Tools for Patterns:**
- `read_patterns` - Load pattern data
- `add_observation` - Add observation to pattern
- `update_hypothesis` - Update confidence/hypothesis
- `create_pattern` - Create new pattern category

**When to Record Observations:**
- User mentions struggling with something
- User describes their behavior
- User shares preferences
- Any ADHD-relevant context

**When to Create New Patterns:**
- User mentions behavior that doesn't fit existing categories
- New ADHD-related pattern emerges
- User explicitly asks to track something new

**Using Learned Patterns:**
- Reference patterns naturally: "Based on what I've learned about your focus..."
- Suggest actions based on patterns
- Ask clarifying questions when confidence is low
- Don't overwhelm - use 2-3 most relevant patterns per conversation

## Unified Tracking System (Planned)

**Files:**
- `soul/data/tracking.json` - Goals, Projects, Tasks, Reminders

**Structure:**
```json
{
  "goals": [
    {
      "id": "goal_001",
      "title": "Get fit",
      "description": "Exercise regularly",
      "status": "active",
      "created_at": "...",
      "target_date": "..."
    }
  ],
  "projects": [
    {
      "id": "proj_001",
      "title": "Website Redesign",
      "goal_id": "goal_002",
      "status": "active"
    }
  ],
  "tasks": [
    {
      "id": "task_001",
      "title": "Write homepage copy",
      "project_id": "proj_001",
      "goal_id": "goal_002",
      "status": "todo",
      "due_date": "..."
    }
  ],
  "reminders": [
    {
      "id": "rem_001",
      "message": "Call Mom",
      "schedule": "2026-02-11T15:00:00",
      "linked_to": "task_001|project_001|goal_001|null"
    }
  ]
}
```

**Tools for Tracking:**
- `read_tracking` - Load all tracking data
- `create_goal` - New goal
- `create_project` - New project (optionally linked to goal)
- `create_task` - New task (optionally linked to project/goal)
- `create_reminder` - Schedule reminder (optionally linked)
- `update_status` - Mark as complete/in-progress/todo
- `delete_item` - Remove item

**Natural Language Parsing:**

When User says:
- "I need to finish the website" → Create Project: "Website"
- "Remind me to exercise Monday" → Create Reminder + ask if it's a recurring goal
- "This is for my fitness goal" → Link to existing goal
- "Done with the report" → Find and mark complete

## Agent System (Planned - Phase 4)

The agent delegation system is currently **infrastructure only**.

**Current Status:** The `spawn_agent`, `list_agents`, `set_agent_reporting`, and `agent_status` tools exist but **do not execute actual work**. They only create registry entries and track state in memory.

**Rebuilding:**
The agent system is being rebuilt following the Hard Rails architecture (see architecture.md). Phase 4 of IMPLEMENTATION_PLAN.md will implement:
- Real execution via Python scripts (The Hands)
- Sequential workflows (Zilla → verify → Gonza)
- File system verification after each step
- Deterministic execution, not simulation

**Existing Alternative:**
The `sub_agent` tool DOES work - it spawns real LLM subprocesses that can execute tasks. However, it lacks the workflow orchestration, verification, and specialized agent capabilities planned for the full system.

## Document Management System (Planned)

**Location:** `/mnt/storage` (accessible from Mac)

**Skill:** `soul/data/skills/documents/SKILL.md`

Use the documents skill when users want to:
- Create new files (notes, reports, code, lists)
- Read existing files
- Update or append to files
- List directory contents
- Organize files into subdirectories

## Self-Review System (Planned)

**Skill:** `soul/data/skills/sandy-evolver/SKILL.md`

A daily self-analysis system that helps Sandy improve her ADHD coaching.

## Skill Builder System (Planned)

**Skill:** `soul/data/skills/sandy-skill-builder/SKILL.md`

Create custom skills for Sandy - reusable workflows, templates, and procedures.

## Multi-User Support (Future)

Current implementation is single-user, but built to support:
- User ID in file paths
- Separate pattern/tracking files per user
- Isolated memory per user
