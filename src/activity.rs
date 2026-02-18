use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,    // "created", "updated", "deleted"
    pub item_type: String, // "goal", "project", "task", "pattern", "reminder"
    pub item_id: String,
    pub item_name: String,
    pub details: Option<String>,
}

pub struct ActivityLogger {
    log_file: PathBuf,
    entries: Mutex<Vec<ActivityEntry>>,
}

impl ActivityLogger {
    pub fn new(data_dir: &str) -> Self {
        let log_file = PathBuf::from(data_dir).join("activity_log.json");
        let entries = if log_file.exists() {
            match fs::read_to_string(&log_file) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        ActivityLogger {
            log_file,
            entries: Mutex::new(entries),
        }
    }

    pub fn log(&self, entry: ActivityEntry) {
        let mut entries = self.entries.lock().unwrap();
        entries.push(entry.clone());

        // Keep only last 1000 entries to prevent file bloat
        if entries.len() > 1000 {
            entries.remove(0);
        }

        // Write to file atomically
        let _ = crate::atomic_io::atomic_write_json(&self.log_file, &*entries);
    }

    pub fn get_entries(&self, limit: usize) -> Vec<ActivityEntry> {
        // Always re-read from file to get latest entries
        let fresh_entries = if self.log_file.exists() {
            match fs::read_to_string(&self.log_file) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        // Update cached entries
        if let Ok(mut entries) = self.entries.lock() {
            *entries = fresh_entries.clone();
        }

        fresh_entries.iter().rev().take(limit).cloned().collect()
    }

    pub fn log_goal_created(&self, id: &str, name: &str) {
        self.log(ActivityEntry {
            timestamp: Utc::now(),
            action: "created".to_string(),
            item_type: "goal".to_string(),
            item_id: id.to_string(),
            item_name: name.to_string(),
            details: None,
        });
    }

    pub fn log_project_created(&self, id: &str, name: &str, goal_id: Option<&str>) {
        let details = goal_id.map(|gid| format!("Linked to goal: {}", gid));
        self.log(ActivityEntry {
            timestamp: Utc::now(),
            action: "created".to_string(),
            item_type: "project".to_string(),
            item_id: id.to_string(),
            item_name: name.to_string(),
            details,
        });
    }

    pub fn log_task_created(&self, id: &str, name: &str, project_id: Option<&str>) {
        let details = project_id.map(|pid| format!("Linked to project: {}", pid));
        self.log(ActivityEntry {
            timestamp: Utc::now(),
            action: "created".to_string(),
            item_type: "task".to_string(),
            item_id: id.to_string(),
            item_name: name.to_string(),
            details,
        });
    }

    pub fn log_status_update(&self, item_type: &str, id: &str, name: &str, status: &str) {
        self.log(ActivityEntry {
            timestamp: Utc::now(),
            action: "updated".to_string(),
            item_type: item_type.to_string(),
            item_id: id.to_string(),
            item_name: name.to_string(),
            details: Some(format!("Status changed to: {}", status)),
        });
    }

    pub fn log_observation_added(&self, pattern_id: &str, pattern_name: &str) {
        self.log(ActivityEntry {
            timestamp: Utc::now(),
            action: "updated".to_string(),
            item_type: "pattern".to_string(),
            item_id: pattern_id.to_string(),
            item_name: pattern_name.to_string(),
            details: Some("New observation added".to_string()),
        });
    }

    pub fn log_pattern_created(&self, id: &str, name: &str) {
        self.log(ActivityEntry {
            timestamp: Utc::now(),
            action: "created".to_string(),
            item_type: "pattern".to_string(),
            item_id: id.to_string(),
            item_name: name.to_string(),
            details: None,
        });
    }
}
