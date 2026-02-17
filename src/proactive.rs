use std::str::FromStr;
use std::sync::Arc;

use tracing::{error, info};

use crate::telegram::AppState;

const MORNING_MARKER: &str = "[proactive:morning]";
const EVENING_MARKER: &str = "[proactive:evening]";

const MORNING_PROMPT: &str = r#"[proactive:morning] You are Sandy, sending a proactive morning check-in.

Use your tools to prepare:
1. Call read_tracking to see today's tasks and goals
2. Call read_patterns to check relevant ADHD patterns

Then compose a warm morning message (under 150 words):
- Greet the user naturally
- Pick ONE priority for today from their tasks
- Mention any relevant pattern insight briefly
- Keep it encouraging but not over-the-top
- End with a simple question about their plan

Do NOT mention that this is automated or scheduled."#;

const EVENING_PROMPT: &str = r#"[proactive:evening] You are Sandy, sending a proactive evening check-in.

Use your tools to prepare:
1. Call read_tracking to review today's tasks and progress
2. Call read_patterns for context

Then compose a brief evening message (under 100 words):
- Acknowledge any progress made today
- If no progress data, just check in warmly
- Ask ONE simple accountability question about tomorrow
- Keep it genuine and brief

Do NOT mention that this is automated or scheduled."#;

/// Ensure morning and evening proactive check-in schedules exist for each control chat.
/// Called once at startup. Idempotent — safe to call on every restart.
pub async fn ensure_proactive_schedules(state: &Arc<AppState>) {
    for &chat_id in &state.config.control_chat_ids {
        if let Err(e) = ensure_for_chat(state, chat_id).await {
            error!("Failed to set up proactive schedules for chat {chat_id}: {e}");
        }
    }
}

async fn ensure_for_chat(state: &Arc<AppState>, chat_id: i64) -> anyhow::Result<()> {
    let existing = state.db.get_tasks_for_chat(chat_id)?;

    let has_morning = existing.iter().any(|t| t.prompt.contains(MORNING_MARKER));
    let has_evening = existing.iter().any(|t| t.prompt.contains(EVENING_MARKER));

    if has_morning && has_evening {
        info!("Proactive schedules already exist for chat {chat_id}");
        return Ok(());
    }

    let tz: chrono_tz::Tz = state
        .config
        .timezone
        .parse()
        .unwrap_or(chrono_tz::Tz::UTC);

    if !has_morning {
        // 9 AM daily
        let cron_expr = "0 0 9 * * *";
        let next_run = compute_next_run(cron_expr, tz)?;
        state
            .db
            .create_scheduled_task(chat_id, MORNING_PROMPT, "cron", cron_expr, &next_run)?;
        info!("Created morning proactive schedule for chat {chat_id}, next run: {next_run}");
    }

    if !has_evening {
        // 8 PM daily
        let cron_expr = "0 0 20 * * *";
        let next_run = compute_next_run(cron_expr, tz)?;
        state
            .db
            .create_scheduled_task(chat_id, EVENING_PROMPT, "cron", cron_expr, &next_run)?;
        info!("Created evening proactive schedule for chat {chat_id}, next run: {next_run}");
    }

    Ok(())
}

fn compute_next_run(cron_expr: &str, tz: chrono_tz::Tz) -> anyhow::Result<String> {
    let schedule = cron::Schedule::from_str(cron_expr)?;
    let next = schedule
        .upcoming(tz)
        .next()
        .ok_or_else(|| anyhow::anyhow!("No upcoming run for cron: {cron_expr}"))?;
    Ok(next.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_next_run() {
        let tz = chrono_tz::Tz::UTC;
        let result = compute_next_run("0 0 9 * * *", tz);
        assert!(result.is_ok());
        let next = result.unwrap();
        assert!(next.contains("T09:00:00"));
    }

    #[test]
    fn test_morning_prompt_contains_marker() {
        assert!(MORNING_PROMPT.starts_with(MORNING_MARKER));
    }

    #[test]
    fn test_evening_prompt_contains_marker() {
        assert!(EVENING_PROMPT.starts_with(EVENING_MARKER));
    }

    #[test]
    fn test_markers_are_distinct() {
        assert_ne!(MORNING_MARKER, EVENING_MARKER);
    }
}
