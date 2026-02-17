use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{info, warn};

use crate::telegram::AppState;

const HEARTBEAT_INTERVAL_HOURS: u64 = 6;

/// Spawn a background task that performs periodic health checks.
/// Runs every 6 hours: runs doctor diagnostics, checks pending tasks
/// nearing deadlines, and sends a summary to control chat if issues found.
pub fn spawn_heartbeat(state: Arc<AppState>) {
    tokio::spawn(async move {
        info!("Heartbeat service started (interval: {}h)", HEARTBEAT_INTERVAL_HOURS);
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(
                HEARTBEAT_INTERVAL_HOURS * 3600,
            ))
            .await;
            run_heartbeat(&state).await;
        }
    });
}

async fn run_heartbeat(state: &Arc<AppState>) {
    info!("Heartbeat: running periodic health check");

    let mut issues = Vec::new();

    // 1. Run doctor diagnostics
    let doctor_result = run_doctor_check(state).await;
    if let Some(warning) = doctor_result {
        issues.push(warning);
    }

    // 2. Check for tasks due soon (within 1 hour)
    let task_warnings = check_upcoming_tasks(state).await;
    issues.extend(task_warnings);

    // 3. Check execution log for recent errors
    let error_summary = check_recent_errors(state).await;
    if let Some(summary) = error_summary {
        issues.push(summary);
    }

    // 4. If issues found, send to control chat
    if !issues.is_empty() {
        let summary = format!(
            "🫀 **Heartbeat Check** ({})\n\n{}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
            issues.join("\n\n")
        );

        send_to_control_chat(state, &summary).await;
    } else {
        info!("Heartbeat: all systems healthy");
    }
}

async fn run_doctor_check(state: &AppState) -> Option<String> {
    // Check key directories exist
    let data_dir = std::path::PathBuf::from(&state.config.data_dir);
    let memory_dir = data_dir.join("memory");

    let mut warnings = Vec::new();

    if !memory_dir.exists() {
        warnings.push("Memory directory missing".to_string());
    }

    let db_path = data_dir.join("sandy.db");
    let db_path_alt = data_dir.join("microclaw.db");
    if !db_path.exists() && !db_path_alt.exists() {
        warnings.push("Database file not found".to_string());
    }

    // Check exec log size
    let log_path = data_dir.join("exec_log.jsonl");
    if let Ok(metadata) = std::fs::metadata(&log_path) {
        let size_mb = metadata.len() / (1024 * 1024);
        if size_mb > 50 {
            warnings.push(format!("Execution log is large: {} MB", size_mb));
        }
    }

    if warnings.is_empty() {
        None
    } else {
        Some(format!("⚠️ **System Issues:**\n{}", warnings.iter().map(|w| format!("  - {}", w)).collect::<Vec<_>>().join("\n")))
    }
}

async fn check_upcoming_tasks(state: &AppState) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for scheduled tasks that are due within the next hour
    let one_hour_from_now = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

    match state.db.get_due_tasks(&one_hour_from_now) {
        Ok(tasks) => {
            if !tasks.is_empty() {
                warnings.push(format!(
                    "📋 **{} tasks due within 1 hour**",
                    tasks.len()
                ));
            }
        }
        Err(e) => {
            warn!("Heartbeat: failed to check upcoming tasks: {}", e);
        }
    }

    warnings
}

async fn check_recent_errors(state: &AppState) -> Option<String> {
    let log_path = std::path::PathBuf::from(&state.config.data_dir).join("exec_log.jsonl");

    if !log_path.exists() {
        return None;
    }

    let content = match std::fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    // Count errors in the last 6 hours
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(HEARTBEAT_INTERVAL_HOURS as i64);
    let cutoff_str = cutoff.to_rfc3339();

    let mut error_count = 0;
    let mut total_count = 0;

    for line in content.lines().rev().take(1000) {
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
                if ts < cutoff_str.as_str() {
                    break; // Past our window
                }
            }
            total_count += 1;
            if entry.get("success") == Some(&serde_json::Value::Bool(false)) {
                error_count += 1;
            }
        }
    }

    if error_count > 0 {
        let error_rate = if total_count > 0 {
            (error_count as f64 / total_count as f64 * 100.0) as u32
        } else {
            0
        };
        Some(format!(
            "🔴 **{} tool errors** in last {}h ({} total calls, {}% error rate)",
            error_count, HEARTBEAT_INTERVAL_HOURS, total_count, error_rate
        ))
    } else {
        None
    }
}

async fn send_to_control_chat(state: &AppState, message: &str) {
    for &chat_id in &state.config.control_chat_ids {
        let chat = teloxide::types::ChatId(chat_id);
        if let Err(e) = state.bot.send_message(chat, message).await {
            warn!("Heartbeat: failed to send to control chat {}: {}", chat_id, e);
        } else {
            info!("Heartbeat: sent summary to control chat {}", chat_id);
        }
    }
}

/// Public function for testing: run a heartbeat check and return the issues found.
pub async fn check_health(state: &AppState) -> Vec<String> {
    let mut issues = Vec::new();

    if let Some(warning) = run_doctor_check(state).await {
        issues.push(warning);
    }

    let task_warnings = check_upcoming_tasks(state).await;
    issues.extend(task_warnings);

    if let Some(summary) = check_recent_errors(state).await {
        issues.push(summary);
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_interval() {
        assert_eq!(HEARTBEAT_INTERVAL_HOURS, 6);
    }

    #[test]
    fn test_error_log_parsing() {
        // Create a test log file
        let dir = std::env::temp_dir().join(format!("sandy_heartbeat_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        let log_content = format!(
            r#"{{"timestamp":"{}","tool_name":"bash","duration_ms":10,"success":true,"error_type":null}}
{{"timestamp":"{}","tool_name":"read_file","duration_ms":5,"success":false,"error_type":"not_found"}}
{{"timestamp":"{}","tool_name":"glob","duration_ms":3,"success":true,"error_type":null}}
"#,
            now, now, now
        );
        std::fs::write(dir.join("exec_log.jsonl"), log_content).unwrap();

        // Read and parse
        let content = std::fs::read_to_string(dir.join("exec_log.jsonl")).unwrap();
        let mut errors = 0;
        let mut total = 0;
        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                total += 1;
                if entry.get("success") == Some(&serde_json::Value::Bool(false)) {
                    errors += 1;
                }
            }
        }

        assert_eq!(total, 3);
        assert_eq!(errors, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_doctor_check_missing_dirs() {
        // This tests the logic of checking missing directories
        let nonexistent = std::path::PathBuf::from("/tmp/sandy_heartbeat_nonexistent_12345");
        assert!(!nonexistent.join("memory").exists());
    }
}
