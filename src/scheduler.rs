use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use teloxide::prelude::*;
use tracing::{error, info};

use crate::db::ScheduledTask;
use crate::telegram::AppState;

pub fn spawn_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        info!("Scheduler started");
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            run_due_tasks(&state).await;
        }
    });
}

async fn run_due_tasks(state: &Arc<AppState>) {
    let now = Utc::now().to_rfc3339();
    info!("Scheduler: checking for due tasks at {}", now);
    let tasks = match state.db.get_due_tasks(&now) {
        Ok(t) => {
            info!("Scheduler: found {} due tasks", t.len());
            for task in &t {
                info!(
                    "Scheduler: task #{} scheduled for {}, current time {}",
                    task.id, task.next_run, now
                );
            }
            t
        }
        Err(e) => {
            error!("Scheduler: failed to query due tasks: {e}");
            return;
        }
    };

    for task in tasks {
        info!(
            "Scheduler: executing task #{} for chat {}",
            task.id, task.chat_id
        );

        let started_at = Utc::now();
        let started_at_str = started_at.to_rfc3339();

        // Check if there's recent conversation activity so the LLM can adapt its tone
        let prompt_with_context = build_context_aware_prompt(state, &task, &started_at);

        // Run agent loop with the task prompt
        let (success, result_summary) = match crate::telegram::process_with_claude_mode(
            state,
            task.chat_id,
            "scheduler",
            "private",
            Some(&prompt_with_context),
            None,
            crate::telegram::PromptMode::Minimal,
        )
        .await
        {
            Ok((response, _meta)) => {
                if !response.is_empty() {
                    crate::telegram::send_response(&state.bot, ChatId(task.chat_id), &response)
                        .await;
                }
                let summary = if response.len() > 200 {
                    format!("{}...", &response[..response.floor_char_boundary(200)])
                } else {
                    response
                };
                (true, Some(summary))
            }
            Err(e) => {
                error!("Scheduler: task #{} failed: {e}", task.id);
                let _ = state
                    .bot
                    .send_message(
                        ChatId(task.chat_id),
                        format!("Scheduled task #{} failed: {e}", task.id),
                    )
                    .await;
                (false, Some(format!("Error: {e}")))
            }
        };

        let finished_at = Utc::now();
        let finished_at_str = finished_at.to_rfc3339();
        let duration_ms = (finished_at - started_at).num_milliseconds();

        // Log the task run
        if let Err(e) = state.db.log_task_run(
            task.id,
            task.chat_id,
            &started_at_str,
            &finished_at_str,
            duration_ms,
            success,
            result_summary.as_deref(),
        ) {
            error!("Scheduler: failed to log task run for #{}: {e}", task.id);
        }

        // Compute next run
        let tz: chrono_tz::Tz = state.config.timezone.parse().unwrap_or_else(|_| {
            tracing::warn!("Invalid timezone '{}' in config, falling back to UTC", state.config.timezone);
            chrono_tz::Tz::UTC
        });
        let next_run = if task.schedule_type == "cron" {
            match cron::Schedule::from_str(&task.schedule_value) {
                Ok(schedule) => schedule.upcoming(tz).next().map(|t| t.to_rfc3339()),
                Err(e) => {
                    error!("Scheduler: invalid cron for task #{}: {e}", task.id);
                    None
                }
            }
        } else {
            None // one-shot
        };

        if let Err(e) =
            state
                .db
                .update_task_after_run(task.id, &started_at_str, next_run.as_deref())
        {
            error!("Scheduler: failed to update task #{}: {e}", task.id);
        }
    }
}

/// Check for recent conversation activity and prepend context to the scheduler prompt
/// so the LLM knows whether it's interrupting an active conversation.
fn build_context_aware_prompt(state: &AppState, task: &ScheduledTask, now: &DateTime<Utc>) -> String {
    let recency_note = match state.db.get_last_message_timestamp(task.chat_id) {
        Ok(Some(ts)) => {
            if let Ok(last_msg_time) = DateTime::parse_from_rfc3339(&ts) {
                let minutes_ago = (*now - last_msg_time.with_timezone(&Utc)).num_minutes();
                if minutes_ago < 5 {
                    Some("CONTEXT: You are currently in an active conversation with the user (last message was less than 5 minutes ago). Do NOT greet them as if starting a new conversation. Instead, naturally weave this into the ongoing chat - e.g. 'By the way...' or 'Quick reminder while we're talking...' or just state the info directly.".to_string())
                } else if minutes_ago < 30 {
                    Some(format!("CONTEXT: The user was active about {} minutes ago. Skip the big greeting - keep it casual and brief, as if continuing a recent chat.", minutes_ago))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    };

    match recency_note {
        Some(note) => format!("{}\n\n{}", note, task.prompt),
        None => task.prompt.clone(),
    }
}
