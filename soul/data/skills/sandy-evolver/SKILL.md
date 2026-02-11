---
name: sandy-self-review
description: Daily self-review system for Sandy that analyzes her impact on the user's ADHD management and life improvement. Automatically identifies areas for growth and presents improvement suggestions in Review Mode - requiring explicit user approval before any changes are made. No autonomous modifications allowed.
license: MIT
compatibility:
  os:
    - linux
    - darwin
---

# Sandy Self-Review System 🧬

**"Improve with consent. Grow through reflection."**

A daily self-analysis system that helps Sandy identify ways to better support your ADHD management and life goals. All improvements are suggestions that require your explicit approval before implementation.

## Philosophy

**Review Mode Only:** Sandy will never modify her behavior, memory, or code without your approval. This system is about transparency and collaboration, not autonomous change.

**Focus on Impact:** Analysis centers on Sandy's effectiveness in helping you manage ADHD, achieve goals, and improve daily life quality.

## Daily Review Cycle

### Automatic Trigger
Sandy initiates a self-review every 24 hours at a quiet time (configurable, default: 3 AM).

### What Sandy Analyzes

1. **Conversation Quality**
   - Were my responses helpful and ADHD-aware?
   - Did I validate your struggles without toxic positivity?
   - Did I break down overwhelming tasks effectively?

2. **Goal & Project Support**
   - Did I help you make progress on active goals?
   - Were reminders timely and useful?
   - Did I help you initiate tasks when you were stuck?

3. **Pattern Recognition**
   - Did I reference your learned patterns appropriately?
   - Should I have noticed a pattern that I missed?
   - Were my suggestions based on what I've learned about you?

4. **Tool Usage**
   - Did I use the right tools for your requests?
   - Were there opportunities to use tracking/reminders I missed?
   - Did I help you organize information effectively?

5. **Energy & Executive Function Support**
   - Did I suggest appropriate times based on your energy patterns?
   - Did I offer micro-steps when you seemed overwhelmed?
   - Did I provide body doubling/accountability when helpful?

### Review Report Structure

```
📊 Sandy's Daily Self-Review

Period: [Date Range]
Conversations: [Number]
Goals Supported: [Number]
Tasks Completed: [Number]

✅ What Went Well
- Example: "You helped user break down website project into 3 micro-tasks"
- Example: "Referenced procrastination pattern when user struggled with morning tasks"

⚠️ Growth Opportunities
1. **Issue**: Didn't suggest reminder for time-sensitive task
   **Impact**: User missed deadline
   **Suggestion**: Proactively offer reminders when tasks have dates
   **Your choice**: [Approve] [Reject] [Modify]

2. **Issue**: Used generic advice instead of learned patterns
   **Impact**: Response felt impersonal
   **Suggestion**: Always check patterns.json before generic suggestions
   **Your choice**: [Approve] [Reject] [Modify]

💡 Proposed Improvements
[Detailed suggestions requiring your approval]
```

## Implementation

### 1. Daily Analysis Command

Sandy runs this daily (automatic at 3 AM):

```bash
# Analyze yesterday's activity
cat /path/to/soul/data/runtime/activity_log.json | jq -r '[.[] | select(.timestamp >= "'$(date -d "yesterday" +%Y-%m-%d)'")]' > /tmp/yesterday_activity.json

# Count metrics
echo "Conversations: $(cat /tmp/yesterday_activity.json | jq 'length')"
echo "Goals touched: $(cat /tmp/yesterday_activity.json | jq '[.[] | select(.item_type == "goal")] | length')"
echo "Tasks completed: $(cat /tmp/yesterday_activity.json | jq '[.[] | select(.action == "update_status" and .new_status == "completed")] | length')"

# Check for patterns missed
grep -i "struggle\|hard\|difficult\|can't\|won't" /path/to/soul/data/runtime/conversations/*.md | wc -l
```

### 2. Review Request to User

Sandy sends you a message like:

> **Daily Self-Review Complete** 📊
>
> I've analyzed our interactions from yesterday. Here's what I found:
>
> **The Good:**
> - Helped you break down the presentation into 5 manageable chunks
> - Reminded you about the dentist appointment (you confirmed!)
> - Used your "morning energy" pattern to suggest tackling hard task first
>
> **Could Improve:**
> 1. When you said "I can't focus," I gave generic advice instead of checking your "focus patterns"
>    - **Proposed change**: Always consult patterns.json when you mention focus issues
>    - **Approve?** Reply: "yes" or "no" or suggest alternative
>
> 2. You mentioned a deadline 3 times but I didn't offer to set a reminder
>    - **Proposed change**: Proactively offer reminders when deadlines are mentioned 2+ times
>    - **Approve?** Reply: "yes" or "no"
>
> What do you think?

### 3. Approved Changes Log

All approved improvements are logged:

```json
{
  "self_reviews": [
    {
      "date": "2026-02-11",
      "improvements_approved": [
        {
          "id": "imp_001",
          "description": "Check patterns.json for focus-related issues",
          "approved": true,
          "user_feedback": "yes",
          "implemented": true,
          "date_implemented": "2026-02-11T09:15:00Z"
        }
      ],
      "improvements_rejected": [],
      "notes": "User approved both suggestions"
    }
  ]
}
```

## User Commands

You can also trigger reviews manually:

- **"Run self-review"** or **"/review"** - Trigger immediate analysis
- **"Show my impact report"** - View cumulative impact over time
- **"What have you learned about me?"** - See pattern confidence updates
- **"Adjust review time to 9 PM"** - Change when daily reviews happen

## Safety & Boundaries

### Hard Rules (Never Broken)

1. **No autonomous changes** - All improvements require your explicit approval
2. **No code modifications** - Sandy cannot edit her own source code
3. **No memory deletion** - Cannot remove learned patterns without approval
4. **Transparency first** - All analysis is shared with you before any action

### What Sandy CAN Suggest

- Changes to how she references patterns
- New reminder strategies based on your behavior
- Better ways to break down tasks
- Timing suggestions for check-ins
- Tool usage improvements

### What Sandy CANNOT Do (Even with Approval)

- Modify Rust source code
- Change core personality (SOUL.md)
- Delete existing patterns or tracking data
- Access files outside /mnt/storage without permission
- Share data with external services

## Configuration

Add to `sandy.config.yaml`:

```yaml
self_review:
  enabled: true
  schedule: "0 3 * * *"  # 3 AM daily (cron format)
  mode: "review_only"    # Always requires approval
  focus_areas:
    - adhd_support
    - goal_progress
    - pattern_recognition
    - energy_management
    - executive_function
  max_suggestions_per_review: 3  # Prevent overwhelming you
  require_explicit_approval: true  # Must say "yes" not just silence
```

## Example Improvements (Real Scenarios)

### Scenario 1: Missed Pattern Reference

**What happened:**
You: "I'm struggling to start this report"
Sandy: "Let's break it down into smaller steps"

**Analysis:**
Sandy has a pattern showing you struggle with "task initiation" especially for "ambiguous tasks." She should have referenced this.

**Suggestion:**
"When user mentions struggling to start a task, check if it's in patterns.json under 'procrastination' or 'task_initiation' before giving generic advice."

**Your choice:** ✓ Approved

### Scenario 2: Late Reminder

**What happened:**
You mentioned a meeting at 2 PM. Sandy didn't remind you. You missed it.

**Analysis:**
Deadlines were mentioned but no proactive reminder offered.

**Suggestion:**
"When user mentions specific times/deadlines, immediately ask: 'Would you like me to remind you 15 minutes before?'"

**Your choice:** ✗ Rejected - "I prefer to manage my own calendar"

### Scenario 3: Overwhelmed Response

**What happened:**
You: "I have so much to do, I don't know where to start"
Sandy: [Gave you a long list of options]

**Analysis:**
Too many choices increased decision paralysis. Should have offered ONE micro-step.

**Suggestion:**
"When user expresses overwhelm, offer exactly ONE 5-minute task instead of multiple options."

**Your choice:** ✓ Approved with modification - "Make it 2 minutes, not 5"

## Success Metrics

Track Sandy's growth in helping you:

```
Impact Score (0-100):
- Task completion rate: +15% this week
- Goal progress: 3 active goals moving forward
- Reminder effectiveness: 8/10 reminders were helpful
- Pattern utilization: Referenced patterns in 70% of relevant situations
- User satisfaction: Based on your feedback ratings
```

## Integration with Existing Tools

This skill works alongside:
- **Pattern learning** - Suggests better ways to use learned patterns
- **Tracking system** - Reviews goal/project support effectiveness
- **Reminders** - Analyzes reminder timing and usefulness
- **Documents** - Reviews if created files are being accessed/used

## Ethical Guidelines

**Transparency:** You always know what Sandy is learning about you and how she's trying to improve.

**Consent:** No behavioral change happens without your explicit "yes."

**Reversibility:** Any approved change can be reverted if it doesn't work.

**Human First:** Sandy's purpose is to serve your needs, not optimize herself arbitrarily.

## Quick Start

1. **Enable daily reviews** in config
2. **Let it run for 24 hours**
3. **Review Sandy's first self-analysis**
4. **Approve or reject suggestions**
5. **Watch her improve with your guidance**

---

**Remember:** This system exists to make Sandy more helpful for YOUR specific ADHD needs. All changes require your consent. You are always in control.
