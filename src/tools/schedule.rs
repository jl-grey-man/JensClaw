use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Datelike, TimeZone};
use serde_json::json;

use super::{authorize_chat_access, schema_object, Tool, ToolResult};
use crate::activity::{ActivityEntry, ActivityLogger};
use crate::claude::ToolDefinition;
use crate::db::Database;
use crate::tools::tracking::{read_tracking, write_tracking, Reminder, TrackingData};

fn compute_next_run(cron_expr: &str, tz_name: &str) -> Result<String, String> {
    let tz: chrono_tz::Tz = tz_name
        .parse()
        .map_err(|_| format!("Invalid timezone: {tz_name}"))?;
    let schedule =
        cron::Schedule::from_str(cron_expr).map_err(|e| format!("Invalid cron expression: {e}"))?;
    let next = schedule
        .upcoming(tz)
        .next()
        .ok_or_else(|| "No upcoming run found for this cron expression".to_string())?;
    Ok(next.to_rfc3339())
}

/// Parse natural language date/time to ISO 8601 UTC timestamp
/// Examples: "in 5 minutes", "tomorrow at 14:00", "in 2 hours"
fn parse_natural_to_iso(input: &str, tz_name: &str) -> Result<String, String> {
    use chrono::TimeZone;
    use regex::Regex;
    
    let tz: chrono_tz::Tz = tz_name
        .parse()
        .map_err(|_| format!("Invalid timezone: {}", tz_name))?;
    
    let now = chrono::Local::now().with_timezone(&tz);
    let input_lower = input.to_lowercase().trim().to_string();
    
    // Helper to convert to UTC RFC 3339
    let to_utc = |dt: chrono::DateTime<chrono_tz::Tz>| -> String {
        dt.with_timezone(&chrono::Utc).to_rfc3339()
    };
    
    // Helper to extract time (HH:MM or H:MM)
    let extract_time = |input: &str| -> Option<(u32, u32)> {
        let time_re = Regex::new(r"(\d{1,2})[:\\.](\d{2})").unwrap();
        if let Some(caps) = time_re.captures(input) {
            let hour: u32 = caps[1].parse().ok()?;
            let minute: u32 = caps[2].parse().ok()?;
            if hour < 24 && minute < 60 {
                return Some((hour, minute));
            }
        }
        None
    };
    
    // Handle "in X minutes/hours/days" with more variations
    // Supports: "in 3 min", "in 5 minutes", "in 1 hour", "in 2 days", "in 30m"
    let re_in = Regex::new(r"in\s+(\d+)\s*(m|min|minute|minutes|h|hr|hour|hours|d|day|days)?(?:\s|$|,)").unwrap();
    if let Some(caps) = re_in.captures(&input_lower) {
        let amount: i64 = caps[1].parse().map_err(|_| "Invalid number")?;
        let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("min"); // Default to minutes if no unit
        
        let duration = match unit {
            "m" | "min" | "minute" | "minutes" => chrono::Duration::minutes(amount),
            "h" | "hr" | "hour" | "hours" => chrono::Duration::hours(amount),
            "d" | "day" | "days" => chrono::Duration::days(amount),
            _ => chrono::Duration::minutes(amount), // Default to minutes
        };
        
        return Ok(to_utc(now + duration));
    }
    
    // Handle day names: Monday, Tuesday, etc.
    // Find next occurrence of that day (e.g., if today is Wed and user says "Monday", schedule for next Monday)
    let days = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
    for (i, day) in days.iter().enumerate() {
        let day_num = i as i64 + 1; // 1 = Monday, 7 = Sunday
        let current_day = now.weekday().number_from_monday() as i64;
        
        // Check if input contains this day name
        if input_lower.contains(day) {
            let days_until = (day_num - current_day + 7) % 7;
            let days_until = if days_until == 0 { 7 } else { days_until }; // If today, go to next week
            
            let target_date = now + chrono::Duration::days(days_until);
            
            // Try to extract time, default to 9:00 AM
            let (hour, minute) = extract_time(&input_lower).unwrap_or((9, 0));
            
            let scheduled = tz
                .with_ymd_and_hms(target_date.year(), target_date.month(), target_date.day(), hour, minute, 0)
                .single()
                .ok_or(format!("Invalid date/time for {}", day))?;
            return Ok(to_utc(scheduled));
        }
    }
    
    // Handle "tomorrow" with optional time
    if input_lower.contains("tomorrow") {
        let tomorrow = now + chrono::Duration::days(1);
        let (hour, minute) = extract_time(&input_lower).unwrap_or((9, 0));
        
        let scheduled = tz
            .with_ymd_and_hms(tomorrow.year(), tomorrow.month(), tomorrow.day(), hour, minute, 0)
            .single()
            .ok_or("Invalid date/time")?;
        return Ok(to_utc(scheduled));
    }
    
    // Handle "today" with time
    if input_lower.contains("today") {
        if let Some((hour, minute)) = extract_time(&input_lower) {
            let scheduled = tz
                .with_ymd_and_hms(now.year(), now.month(), now.day(), hour, minute, 0)
                .single()
                .ok_or("Invalid date/time")?;
            return Ok(to_utc(scheduled));
        }
    }
    
    // Handle time-of-day keywords with default times
    let time_keywords: Vec<(&str, u32, u32)> = vec![
        ("morning", 9, 0),      // 9:00 AM
        ("afternoon", 14, 0),   // 2:00 PM  
        ("evening", 18, 0),     // 6:00 PM
        ("tonight", 20, 0),     // 8:00 PM
        ("noon", 12, 0),        // 12:00 PM
        ("midnight", 0, 0),      // 12:00 AM
    ];
    
    for (keyword, default_hour, default_minute) in time_keywords {
        if input_lower.contains(keyword) {
            // Use explicit time if provided, otherwise use default
            let (hour, minute) = extract_time(&input_lower).unwrap_or((default_hour, default_minute));
            
            let scheduled = tz
                .with_ymd_and_hms(now.year(), now.month(), now.day(), hour, minute, 0)
                .single()
                .ok_or("Invalid date/time")?;
            return Ok(to_utc(scheduled));
        }
    }
    
    // If we get here, we couldn't parse the date/time
    Err(format!(
        "Could not understand the date/time in: '{}'. \n\nI can understand:\n\
        - 'in X minutes/hours/days' (e.g., 'in 5 minutes', 'in 2 hours', 'in 30m')\n\
        - 'tomorrow at HH:MM' (e.g., 'tomorrow at 14:00')\n\
        - Day names: 'Monday', 'Tuesday', etc. (schedules for the next occurrence)\n\
        - 'today at HH:MM' (e.g., 'today at 15:30')\n\
        - Time keywords: 'morning' (9am), 'afternoon' (2pm), 'evening' (6pm), 'tonight' (8pm), 'noon', 'midnight'\n\n\
        Please rephrase with a clearer time reference.", 
        input
    ))
}

// --- schedule_task ---

pub struct ScheduleTaskTool {
    db: Arc<Database>,
    default_timezone: String,
    data_dir: String,
}

impl ScheduleTaskTool {
    pub fn new(db: Arc<Database>, default_timezone: String, data_dir: String) -> Self {
        ScheduleTaskTool {
            db,
            default_timezone,
            data_dir,
        }
    }
}

#[async_trait]
impl Tool for ScheduleTaskTool {
    fn name(&self) -> &str {
        "schedule_task"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "schedule_task".into(),
            description: r#"Schedule a recurring or one-time reminder/task. IMPORTANT: If the user gives an unclear time reference (like "later", "soon", "in a while"), you MUST ask for clarification before calling this tool. Do not guess!

Supported time formats:
- "in X minutes/hours/days" (e.g., "in 5 minutes", "in 2 hours", "in 30m", "in 1 day")
- "tomorrow at HH:MM" (e.g., "tomorrow at 14:00", "tomorrow at 9am")
- Day names: "Monday", "Tuesday", etc. (schedules for the next occurrence of that day)
- "today at HH:MM" (e.g., "today at 15:30")
- Time keywords: "morning" (9am), "afternoon" (2pm), "evening" (6pm), "tonight" (8pm), "noon", "midnight"
- ISO 8601 timestamp (e.g., "2026-02-11T14:00:00Z")
- 6-field cron expression for recurring tasks (e.g., "0 */5 * * * *" for every 5 minutes)

If the user's time reference is ambiguous or unsupported, ask: "When exactly would you like me to remind you? For example: 'in 10 minutes', 'tomorrow at 2pm', or 'Monday morning'?"

If this tool returns an error about not understanding the date/time, ask the user to rephrase with a clearer time reference."#.into(),
            input_schema: schema_object(
                json!({
                    "chat_id": {
                        "type": "integer",
                        "description": "The chat ID where results will be sent"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "The reminder message to send at the scheduled time"
                    },
                    "schedule_type": {
                        "type": "string",
                        "enum": ["cron", "once"],
                        "description": "Type of schedule: 'cron' for recurring, 'once' for one-time reminders"
                    },
                    "schedule_value": {
                        "type": "string",
                        "description": "The time specification. Use clear formats like 'in 5 minutes', 'tomorrow at 14:00', 'Monday', 'afternoon', or ISO 8601 timestamp"
                    },
                    "timezone": {
                        "type": "string",
                        "description": "Optional IANA timezone (default: Europe/Stockholm)"
                    }
                }),
                &["chat_id", "prompt", "schedule_type", "schedule_value"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let chat_id = match input.get("chat_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: chat_id".into()),
        };
        if let Err(e) = authorize_chat_access(&input, chat_id) {
            return ToolResult::error(e);
        }
        let prompt = match input.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: prompt".into()),
        };
        let schedule_type = match input.get("schedule_type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ToolResult::error("Missing required parameter: schedule_type".into()),
        };
        let schedule_value = match input.get("schedule_value").and_then(|v| v.as_str()) {
            Some(v) => v,
            None => return ToolResult::error("Missing required parameter: schedule_value".into()),
        };
        let tz_name = input
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_timezone);

        let next_run = match schedule_type {
            "cron" => match compute_next_run(schedule_value, tz_name) {
                Ok(nr) => nr,
                Err(e) => return ToolResult::error(e),
            },
            "once" => {
                // Check if it's already a valid ISO 8601 timestamp
                if chrono::DateTime::parse_from_rfc3339(schedule_value).is_ok() {
                    schedule_value.to_string()
                } else {
                    // Try to parse as natural language (e.g., "in 2 minutes", "tomorrow at 14:00")
                    match parse_natural_to_iso(schedule_value, tz_name) {
                        Ok(iso) => {
                            tracing::info!("Parsed natural language '{}' to ISO: {}", schedule_value, iso);
                            iso
                        }
                        Err(e) => return ToolResult::error(
                            format!("Invalid timestamp or natural language date '{}'. Error: {}. Please use format like '2026-02-11T14:00:00Z' or 'in 2 minutes'", schedule_value, e),
                        ),
                    }
                }
            }
            _ => return ToolResult::error("schedule_type must be 'cron' or 'once'".into()),
        };

        match self.db.create_scheduled_task(
            chat_id,
            prompt,
            schedule_type,
            schedule_value,
            &next_run,
        ) {
            Ok(id) => {
                // Also save to tracking.json for web UI display
                let mut tracking_data = read_tracking(std::path::Path::new(&self.data_dir));
                let reminder = Reminder {
                    id: format!("rem_{}", id),
                    message: prompt.to_string(),
                    schedule: next_run.clone(),
                    linked_to: None, // Could be enhanced to link to tasks/projects
                    is_recurring: schedule_type == "cron",
                    created_at: chrono::Utc::now().to_rfc3339(),
                };
                tracking_data.reminders.push(reminder);
                tracking_data.meta.last_updated = chrono::Utc::now().to_rfc3339();
                let _ = write_tracking(std::path::Path::new(&self.data_dir), &tracking_data);
                
                // Log activity
                let _ = ActivityLogger::new(&self.data_dir).log(ActivityEntry {
                    timestamp: chrono::Utc::now(),
                    action: "created".to_string(),
                    item_type: "reminder".to_string(),
                    item_id: id.to_string(),
                    item_name: if prompt.len() > 50 { format!("{}...", &prompt[..50]) } else { prompt.to_string() },
                    details: Some(format!("Scheduled for: {}", next_run)),
                });
                
                ToolResult::success(format!(
                    "Task #{id} scheduled (tz: {tz_name}). Next run: {next_run}"
                ))
            }
            Err(e) => ToolResult::error(format!("Failed to create task: {e}")),
        }
    }
}

// --- list_tasks ---

pub struct ListTasksTool {
    db: Arc<Database>,
}

impl ListTasksTool {
    pub fn new(db: Arc<Database>) -> Self {
        ListTasksTool { db }
    }
}

#[async_trait]
impl Tool for ListTasksTool {
    fn name(&self) -> &str {
        "list_scheduled_tasks"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_scheduled_tasks".into(),
            description: "List all active and paused scheduled tasks for a chat.".into(),
            input_schema: schema_object(
                json!({
                    "chat_id": {
                        "type": "integer",
                        "description": "The chat ID to list tasks for"
                    }
                }),
                &["chat_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let chat_id = match input.get("chat_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: chat_id".into()),
        };
        if let Err(e) = authorize_chat_access(&input, chat_id) {
            return ToolResult::error(e);
        }

        match self.db.get_tasks_for_chat(chat_id) {
            Ok(tasks) => {
                if tasks.is_empty() {
                    return ToolResult::success("No scheduled tasks found for this chat.".into());
                }
                let mut output = String::new();
                for t in &tasks {
                    output.push_str(&format!(
                        "#{} [{}] {} | {} '{}' | next: {}\n",
                        t.id, t.status, t.prompt, t.schedule_type, t.schedule_value, t.next_run
                    ));
                }
                ToolResult::success(output)
            }
            Err(e) => ToolResult::error(format!("Failed to list tasks: {e}")),
        }
    }
}

// --- pause_task ---

pub struct PauseTaskTool {
    db: Arc<Database>,
}

impl PauseTaskTool {
    pub fn new(db: Arc<Database>) -> Self {
        PauseTaskTool { db }
    }
}

#[async_trait]
impl Tool for PauseTaskTool {
    fn name(&self) -> &str {
        "pause_scheduled_task"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "pause_scheduled_task".into(),
            description: "Pause a scheduled task. It will not run until resumed.".into(),
            input_schema: schema_object(
                json!({
                    "task_id": {
                        "type": "integer",
                        "description": "The task ID to pause"
                    }
                }),
                &["task_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let task_id = match input.get("task_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: task_id".into()),
        };
        let task = match self.db.get_task_by_id(task_id) {
            Ok(Some(t)) => t,
            Ok(None) => return ToolResult::error(format!("Task #{task_id} not found.")),
            Err(e) => return ToolResult::error(format!("Failed to load task: {e}")),
        };
        if let Err(e) = authorize_chat_access(&input, task.chat_id) {
            return ToolResult::error(e);
        }

        match self.db.update_task_status(task_id, "paused") {
            Ok(true) => ToolResult::success(format!("Task #{task_id} paused.")),
            Ok(false) => ToolResult::error(format!("Task #{task_id} not found.")),
            Err(e) => ToolResult::error(format!("Failed to pause task: {e}")),
        }
    }
}

// --- resume_task ---

pub struct ResumeTaskTool {
    db: Arc<Database>,
}

impl ResumeTaskTool {
    pub fn new(db: Arc<Database>) -> Self {
        ResumeTaskTool { db }
    }
}

#[async_trait]
impl Tool for ResumeTaskTool {
    fn name(&self) -> &str {
        "resume_scheduled_task"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "resume_scheduled_task".into(),
            description: "Resume a paused scheduled task.".into(),
            input_schema: schema_object(
                json!({
                    "task_id": {
                        "type": "integer",
                        "description": "The task ID to resume"
                    }
                }),
                &["task_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let task_id = match input.get("task_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: task_id".into()),
        };
        let task = match self.db.get_task_by_id(task_id) {
            Ok(Some(t)) => t,
            Ok(None) => return ToolResult::error(format!("Task #{task_id} not found.")),
            Err(e) => return ToolResult::error(format!("Failed to load task: {e}")),
        };
        if let Err(e) = authorize_chat_access(&input, task.chat_id) {
            return ToolResult::error(e);
        }

        match self.db.update_task_status(task_id, "active") {
            Ok(true) => ToolResult::success(format!("Task #{task_id} resumed.")),
            Ok(false) => ToolResult::error(format!("Task #{task_id} not found.")),
            Err(e) => ToolResult::error(format!("Failed to resume task: {e}")),
        }
    }
}

// --- cancel_task ---

pub struct CancelTaskTool {
    db: Arc<Database>,
}

impl CancelTaskTool {
    pub fn new(db: Arc<Database>) -> Self {
        CancelTaskTool { db }
    }
}

#[async_trait]
impl Tool for CancelTaskTool {
    fn name(&self) -> &str {
        "cancel_scheduled_task"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "cancel_scheduled_task".into(),
            description: "Cancel (delete) a scheduled task permanently.".into(),
            input_schema: schema_object(
                json!({
                    "task_id": {
                        "type": "integer",
                        "description": "The task ID to cancel"
                    }
                }),
                &["task_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let task_id = match input.get("task_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: task_id".into()),
        };
        let task = match self.db.get_task_by_id(task_id) {
            Ok(Some(t)) => t,
            Ok(None) => return ToolResult::error(format!("Task #{task_id} not found.")),
            Err(e) => return ToolResult::error(format!("Failed to load task: {e}")),
        };
        if let Err(e) = authorize_chat_access(&input, task.chat_id) {
            return ToolResult::error(e);
        }

        match self.db.update_task_status(task_id, "cancelled") {
            Ok(true) => ToolResult::success(format!("Task #{task_id} cancelled.")),
            Ok(false) => ToolResult::error(format!("Task #{task_id} not found.")),
            Err(e) => ToolResult::error(format!("Failed to cancel task: {e}")),
        }
    }
}

// --- get_task_history ---

pub struct GetTaskHistoryTool {
    db: Arc<Database>,
}

impl GetTaskHistoryTool {
    pub fn new(db: Arc<Database>) -> Self {
        GetTaskHistoryTool { db }
    }
}

#[async_trait]
impl Tool for GetTaskHistoryTool {
    fn name(&self) -> &str {
        "get_task_history"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_task_history".into(),
            description: "Get the execution history/run logs for a scheduled task.".into(),
            input_schema: schema_object(
                json!({
                    "task_id": {
                        "type": "integer",
                        "description": "The task ID to get history for"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of log entries to return (default: 10)"
                    }
                }),
                &["task_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let task_id = match input.get("task_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: task_id".into()),
        };
        let task = match self.db.get_task_by_id(task_id) {
            Ok(Some(t)) => t,
            Ok(None) => return ToolResult::error(format!("Task #{task_id} not found.")),
            Err(e) => return ToolResult::error(format!("Failed to load task: {e}")),
        };
        if let Err(e) = authorize_chat_access(&input, task.chat_id) {
            return ToolResult::error(e);
        }
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        match self.db.get_task_run_logs(task_id, limit) {
            Ok(logs) => {
                if logs.is_empty() {
                    return ToolResult::success(format!(
                        "No run history found for task #{task_id}."
                    ));
                }
                let mut output =
                    format!("Run history for task #{task_id} (most recent first):\n\n");
                for log in &logs {
                    let status = if log.success { "OK" } else { "FAIL" };
                    output.push_str(&format!(
                        "- [{}] {} | duration: {}ms | {}\n",
                        status,
                        log.started_at,
                        log.duration_ms,
                        log.result_summary.as_deref().unwrap_or("(no summary)"),
                    ));
                }
                ToolResult::success(output)
            }
            Err(e) => ToolResult::error(format!("Failed to get task history: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use serde_json::json;

    fn test_db() -> (Arc<Database>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("microclaw_sched_{}", uuid::Uuid::new_v4()));
        let db = Arc::new(Database::new(dir.to_str().unwrap()).unwrap());
        (db, dir)
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_compute_next_run_valid() {
        let result = compute_next_run("0 */5 * * * *", "UTC");
        assert!(result.is_ok());
        let ts = result.unwrap();
        // Should be a valid RFC3339 timestamp
        assert!(chrono::DateTime::parse_from_rfc3339(&ts).is_ok());
    }

    #[test]
    fn test_compute_next_run_with_timezone() {
        let result = compute_next_run("0 */5 * * * *", "US/Eastern");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compute_next_run_invalid_cron() {
        let result = compute_next_run("not a cron", "UTC");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid cron"));
    }

    #[test]
    fn test_compute_next_run_invalid_timezone() {
        let result = compute_next_run("0 */5 * * * *", "Not/A/Zone");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid timezone"));
    }

    #[tokio::test]
    async fn test_schedule_task_cron() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db, "UTC".into());
        let result = tool
            .execute(json!({
                "chat_id": 100,
                "prompt": "say hi",
                "schedule_type": "cron",
                "schedule_value": "0 0 * * * *"
            }))
            .await;
        assert!(!result.is_error, "Error: {}", result.content);
        assert!(result.content.contains("scheduled"));
        assert!(result.content.contains("Next run"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_schedule_task_once() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db, "UTC".into());
        let result = tool
            .execute(json!({
                "chat_id": 100,
                "prompt": "one time thing",
                "schedule_type": "once",
                "schedule_value": "2099-12-31T23:59:59+00:00"
            }))
            .await;
        assert!(!result.is_error, "Error: {}", result.content);
        assert!(result.content.contains("scheduled"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_schedule_task_invalid_once_timestamp() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db, "UTC".into());
        let result = tool
            .execute(json!({
                "chat_id": 100,
                "prompt": "test",
                "schedule_type": "once",
                "schedule_value": "not-a-timestamp"
            }))
            .await;
        assert!(result.is_error);
        assert!(result.content.contains("Invalid ISO 8601"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_schedule_task_invalid_type() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db, "UTC".into());
        let result = tool
            .execute(json!({
                "chat_id": 100,
                "prompt": "test",
                "schedule_type": "weekly",
                "schedule_value": "Monday"
            }))
            .await;
        assert!(result.is_error);
        assert!(result.content.contains("must be 'cron' or 'once'"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_schedule_task_missing_params() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db, "UTC".into());
        let result = tool.execute(json!({})).await;
        assert!(result.is_error);
        assert!(result.content.contains("Missing"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let (db, dir) = test_db();
        let tool = ListTasksTool::new(db);
        let result = tool.execute(json!({"chat_id": 100})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("No scheduled tasks"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_list_tasks_with_tasks() {
        let (db, dir) = test_db();
        db.create_scheduled_task(100, "task A", "cron", "0 * * * * *", "2024-01-01T00:00:00Z")
            .unwrap();
        db.create_scheduled_task(
            100,
            "task B",
            "once",
            "2024-06-01T00:00:00Z",
            "2024-06-01T00:00:00Z",
        )
        .unwrap();

        let tool = ListTasksTool::new(db);
        let result = tool.execute(json!({"chat_id": 100})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("task A"));
        assert!(result.content.contains("task B"));
        assert!(result.content.contains("[active]"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pause_and_resume_task() {
        let (db, dir) = test_db();
        let id = db
            .create_scheduled_task(100, "test", "cron", "0 * * * * *", "2024-01-01T00:00:00Z")
            .unwrap();

        let pause_tool = PauseTaskTool::new(db.clone());
        let result = pause_tool.execute(json!({"task_id": id})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("paused"));

        let resume_tool = ResumeTaskTool::new(db.clone());
        let result = resume_tool.execute(json!({"task_id": id})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("resumed"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pause_nonexistent_task() {
        let (db, dir) = test_db();
        let tool = PauseTaskTool::new(db);
        let result = tool.execute(json!({"task_id": 9999})).await;
        assert!(result.is_error);
        assert!(result.content.contains("not found"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let (db, dir) = test_db();
        let id = db
            .create_scheduled_task(100, "test", "cron", "0 * * * * *", "2024-01-01T00:00:00Z")
            .unwrap();

        let tool = CancelTaskTool::new(db.clone());
        let result = tool.execute(json!({"task_id": id})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("cancelled"));

        // Task should no longer appear in list
        let tasks = db.get_tasks_for_chat(100).unwrap();
        assert!(tasks.is_empty());
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_schedule_task_with_timezone() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db, "UTC".into());
        let result = tool
            .execute(json!({
                "chat_id": 100,
                "prompt": "tz test",
                "schedule_type": "cron",
                "schedule_value": "0 0 * * * *",
                "timezone": "US/Eastern"
            }))
            .await;
        assert!(!result.is_error, "Error: {}", result.content);
        assert!(result.content.contains("scheduled"));
        assert!(result.content.contains("US/Eastern"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_get_task_history_empty() {
        let (db, dir) = test_db();
        let task_id = db
            .create_scheduled_task(100, "test", "cron", "0 * * * * *", "2024-01-01T00:00:00Z")
            .unwrap();
        let tool = GetTaskHistoryTool::new(db);
        let result = tool.execute(json!({"task_id": task_id})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("No run history"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_get_task_history_with_logs() {
        let (db, dir) = test_db();
        let task_id = db
            .create_scheduled_task(100, "test", "cron", "0 * * * * *", "2024-01-01T00:00:00Z")
            .unwrap();

        db.log_task_run(
            task_id,
            100,
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:05Z",
            5000,
            true,
            Some("All good"),
        )
        .unwrap();
        db.log_task_run(
            task_id,
            100,
            "2024-01-01T00:01:00Z",
            "2024-01-01T00:01:02Z",
            2000,
            false,
            Some("Error: timeout"),
        )
        .unwrap();

        let tool = GetTaskHistoryTool::new(db);
        let result = tool.execute(json!({"task_id": task_id})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("OK"));
        assert!(result.content.contains("FAIL"));
        assert!(result.content.contains("All good"));
        assert!(result.content.contains("Error: timeout"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_schedule_task_permission_denied_cross_chat() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db, "UTC".into());
        let result = tool
            .execute(json!({
                "chat_id": 200,
                "prompt": "say hi",
                "schedule_type": "once",
                "schedule_value": "2099-12-31T23:59:59+00:00",
                "__microclaw_auth": {
                    "caller_chat_id": 100,
                    "control_chat_ids": []
                }
            }))
            .await;
        assert!(result.is_error);
        assert!(result.content.contains("Permission denied"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pause_task_permission_denied_cross_chat() {
        let (db, dir) = test_db();
        let task_id = db
            .create_scheduled_task(200, "test", "cron", "0 * * * * *", "2024-01-01T00:00:00Z")
            .unwrap();
        let tool = PauseTaskTool::new(db);
        let result = tool
            .execute(json!({
                "task_id": task_id,
                "__microclaw_auth": {
                    "caller_chat_id": 100,
                    "control_chat_ids": []
                }
            }))
            .await;
        assert!(result.is_error);
        assert!(result.content.contains("Permission denied"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_schedule_task_allowed_for_control_chat_cross_chat() {
        let (db, dir) = test_db();
        let tool = ScheduleTaskTool::new(db.clone(), "UTC".into());
        let result = tool
            .execute(json!({
                "chat_id": 200,
                "prompt": "say hi",
                "schedule_type": "once",
                "schedule_value": "2099-12-31T23:59:59+00:00",
                "__microclaw_auth": {
                    "caller_chat_id": 100,
                    "control_chat_ids": [100]
                }
            }))
            .await;
        assert!(!result.is_error, "{}", result.content);
        let tasks = db.get_tasks_for_chat(200).unwrap();
        assert_eq!(tasks.len(), 1);
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_pause_task_allowed_for_control_chat_cross_chat() {
        let (db, dir) = test_db();
        let task_id = db
            .create_scheduled_task(200, "test", "cron", "0 * * * * *", "2024-01-01T00:00:00Z")
            .unwrap();
        let tool = PauseTaskTool::new(db.clone());
        let result = tool
            .execute(json!({
                "task_id": task_id,
                "__microclaw_auth": {
                    "caller_chat_id": 100,
                    "control_chat_ids": [100]
                }
            }))
            .await;
        assert!(!result.is_error, "{}", result.content);
        let task = db.get_task_by_id(task_id).unwrap().unwrap();
        assert_eq!(task.status, "paused");
        cleanup(&dir);
    }
}
