use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

use crate::activity::{ActivityEntry, ActivityLogger};
use crate::claude::ToolDefinition;

use super::{schema_object, Tool, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String, // "active", "completed", "paused"
    pub created_at: String,
    pub target_date: Option<String>,
    pub completed_at: Option<String>,
    pub notes: Option<String>, // Sandy's ongoing context and observations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub goal_id: Option<String>,
    pub status: String, // "active", "completed", "paused"
    pub created_at: String,
    pub target_date: Option<String>,
    pub completed_at: Option<String>,
    pub notes: Option<String>, // Sandy's ongoing context and observations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub goal_id: Option<String>,
    pub status: String, // "todo", "in_progress", "done"
    pub created_at: String,
    pub due_date: Option<String>,
    pub completed_at: Option<String>,
    pub notes: Option<String>, // Sandy's ongoing context and observations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub message: String,
    pub schedule: String, // ISO 8601 datetime or cron expression
    pub linked_to: Option<String>, // "task_id|project_id|goal_id"
    pub is_recurring: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingData {
    pub version: String,
    pub user_id: String,
    pub goals: Vec<Goal>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub reminders: Vec<Reminder>,
    pub meta: TrackingMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingMeta {
    pub last_updated: String,
    pub schema_version: String,
}

pub fn tracking_path(data_dir: &Path) -> PathBuf {
    data_dir.join("tracking.json")
}

pub fn read_tracking(data_dir: &Path) -> TrackingData {
    let path = tracking_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| default_tracking()),
        Err(_) => default_tracking(),
    }
}

pub fn write_tracking(data_dir: &Path, data: &TrackingData) -> std::io::Result<()> {
    let path = tracking_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(data).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

pub fn default_tracking() -> TrackingData {
    TrackingData {
        version: "1.0".to_string(),
        user_id: "default".to_string(),
        goals: vec![],
        projects: vec![],
        tasks: vec![],
        reminders: vec![],
        meta: TrackingMeta {
            last_updated: chrono::Utc::now().to_rfc3339(),
            schema_version: "1.0".to_string(),
        },
    }
}

fn generate_id(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4().to_string().split('-').next().unwrap())
}

fn format_tracking_summary(data: &TrackingData) -> String {
    let mut out = String::new();
    
    out.push_str(&format!("📊 Tracking Summary\n\n"));
    out.push_str(&format!("🎯 Goals: {} active\n", 
        data.goals.iter().filter(|g| g.status == "active").count()));
    out.push_str(&format!("📚 Projects: {} active\n",
        data.projects.iter().filter(|p| p.status == "active").count()));
    out.push_str(&format!("✅ Tasks: {} todo, {} in progress, {} done\n",
        data.tasks.iter().filter(|t| t.status == "todo").count(),
        data.tasks.iter().filter(|t| t.status == "in_progress").count(),
        data.tasks.iter().filter(|t| t.status == "done").count()));
    out.push_str(&format!("⏰ Reminders: {} scheduled\n", data.reminders.len()));
    
    out
}

fn format_goals(goals: &[Goal]) -> String {
    if goals.is_empty() {
        return "No goals set yet.".into();
    }

    let mut out = String::new();
    for goal in goals {
        let icon = match goal.status.as_str() {
            "completed" => "✅",
            "paused" => "⏸️",
            _ => "🎯",
        };
        out.push_str(&format!("{} {} ({}.)\n", icon, goal.title, goal.id));
        if let Some(ref desc) = goal.description {
            out.push_str(&format!("   {}\n", desc));
        }
        if let Some(ref target) = goal.target_date {
            out.push_str(&format!("   Target: {}\n", target));
        }
        out.push('\n');
    }
    out
}

fn format_projects(projects: &[Project], goals: &[Goal]) -> String {
    if projects.is_empty() {
        return "No projects yet.".into();
    }

    let mut out = String::new();
    for project in projects {
        let icon = match project.status.as_str() {
            "completed" => "✅",
            "paused" => "⏸️",
            _ => "📚",
        };
        
        let goal_info = project.goal_id.as_ref().and_then(|gid| {
            goals.iter().find(|g| g.id == *gid).map(|g| format!(" → {}", g.title))
        }).unwrap_or_default();
        
        out.push_str(&format!("{} {} ({}){}\n", icon, project.title, project.id, goal_info));
        if let Some(ref desc) = project.description {
            out.push_str(&format!("   {}\n", desc));
        }
        out.push('\n');
    }
    out
}

fn format_tasks(tasks: &[Task], projects: &[Project]) -> String {
    if tasks.is_empty() {
        return "No tasks yet.".into();
    }

    let mut out = String::new();
    
    // Group by status
    let todo: Vec<_> = tasks.iter().filter(|t| t.status == "todo").collect();
    let in_progress: Vec<_> = tasks.iter().filter(|t| t.status == "in_progress").collect();
    let done: Vec<_> = tasks.iter().filter(|t| t.status == "done").collect();

    if !in_progress.is_empty() {
        out.push_str("In Progress:\n");
        for task in in_progress {
            let project_info = task.project_id.as_ref().and_then(|pid| {
                projects.iter().find(|p| p.id == *pid).map(|p| format!(" [{}]", p.title))
            }).unwrap_or_default();
            out.push_str(&format!("  [~] {}{} ({}.)\n", task.title, project_info, task.id));
        }
        out.push('\n');
    }

    if !todo.is_empty() {
        out.push_str("To Do:\n");
        for task in todo {
            let project_info = task.project_id.as_ref().and_then(|pid| {
                projects.iter().find(|p| p.id == *pid).map(|p| format!(" [{}]", p.title))
            }).unwrap_or_default();
            out.push_str(&format!("  [ ] {}{} ({}.)\n", task.title, project_info, task.id));
        }
        out.push('\n');
    }

    if !done.is_empty() {
        out.push_str(&format!("Completed ({} tasks)\n", done.len()));
    }

    out
}

fn format_reminders(reminders: &[Reminder]) -> String {
    if reminders.is_empty() {
        return "No reminders set.".into();
    }

    let mut out = String::new();
    for reminder in reminders {
        let icon = if reminder.is_recurring { "🔄" } else { "⏰" };
        out.push_str(&format!("{} {} - {} ({}.)\n", 
            icon, reminder.schedule, reminder.message, reminder.id));
    }
    out
}

// --- ReadTrackingTool ---

pub struct ReadTrackingTool {
    data_dir: PathBuf,
}

impl ReadTrackingTool {
    pub fn new(data_dir: &str) -> Self {
        ReadTrackingTool {
            data_dir: PathBuf::from(data_dir),
        }
    }
}

#[async_trait]
impl Tool for ReadTrackingTool {
    fn name(&self) -> &str {
        "read_tracking"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_tracking".into(),
            description: "Read the tracking data (goals, projects, tasks, reminders). Shows summary by default. Use 'type' parameter to see specific sections: 'goals', 'projects', 'tasks', 'reminders', or 'all' for everything.".into(),
            input_schema: schema_object(
                json!({
                    "type": {
                        "type": "string",
                        "enum": ["summary", "goals", "projects", "tasks", "reminders", "all"],
                        "description": "What to show: summary (default), goals, projects, tasks, reminders, or all"
                    }
                }),
                &[],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let show_type = input.get("type").and_then(|v| v.as_str()).unwrap_or("summary");

        info!("Reading tracking data (type: {})", show_type);
        let data = read_tracking(&self.data_dir);

        let result = match show_type {
            "summary" => format_tracking_summary(&data),
            "goals" => format_goals(&data.goals),
            "projects" => format_projects(&data.projects, &data.goals),
            "tasks" => format_tasks(&data.tasks, &data.projects),
            "reminders" => format_reminders(&data.reminders),
            "all" => {
                let mut full = format_tracking_summary(&data);
                full.push_str("\n🎯 GOALS\n");
                full.push_str(&format_goals(&data.goals));
                full.push_str("\n📚 PROJECTS\n");
                full.push_str(&format_projects(&data.projects, &data.goals));
                full.push_str("\n✅ TASKS\n");
                full.push_str(&format_tasks(&data.tasks, &data.projects));
                full.push_str("\n⏰ REMINDERS\n");
                full.push_str(&format_reminders(&data.reminders));
                full
            }
            _ => format_tracking_summary(&data),
        };

        ToolResult::success(result)
    }
}

// --- CreateGoalTool ---

pub struct CreateGoalTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl CreateGoalTool {
    pub fn new(data_dir: &str) -> Self {
        CreateGoalTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for CreateGoalTool {
    fn name(&self) -> &str {
        "create_goal"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_goal".into(),
            description: "Create a new goal. Goals are big outcomes User wants to achieve. Can have projects and tasks linked to it.".into(),
            input_schema: schema_object(
                json!({
                    "title": {
                        "type": "string",
                        "description": "The goal title"
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional description"
                    },
                    "target_date": {
                        "type": "string",
                        "description": "Optional target date (ISO 8601 format)"
                    }
                }),
                &["title"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let title = match input.get("title").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'title' parameter".into()),
        };

        let description = input.get("description").and_then(|v| v.as_str()).map(String::from);
        let target_date = input.get("target_date").and_then(|v| v.as_str()).map(String::from);

        let mut data = read_tracking(&self.data_dir);

        let goal = Goal {
            id: generate_id("goal"),
            title,
            description,
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            target_date,
            completed_at: None,
            notes: None,
        };

        info!("Creating goal: {}", goal.title);
        let goal_id = goal.id.clone();
        let goal_title = goal.title.clone();
        data.goals.push(goal);
        data.meta.last_updated = chrono::Utc::now().to_rfc3339();

        // Log activity
        self.activity_logger.log_goal_created(&goal_id, &goal_title);

        match write_tracking(&self.data_dir, &data) {
            Ok(()) => ToolResult::success(format!(
                "🎯 Goal created: {} ({}.)
Total goals: {}",
                data.goals.last().unwrap().title,
                data.goals.last().unwrap().id,
                data.goals.len()
            )),
            Err(e) => ToolResult::error(format!("Failed to create goal: {e}")),
        }
    }
}

// --- CreateProjectTool ---

pub struct CreateProjectTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl CreateProjectTool {
    pub fn new(data_dir: &str) -> Self {
        CreateProjectTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for CreateProjectTool {
    fn name(&self) -> &str {
        "create_project"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_project".into(),
            description: "Create a new project. Projects are paths to goals, containing multiple tasks. Optionally link to a goal.".into(),
            input_schema: schema_object(
                json!({
                    "title": {
                        "type": "string",
                        "description": "The project title"
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional description"
                    },
                    "goal_id": {
                        "type": "string",
                        "description": "Optional goal ID to link this project to"
                    }
                }),
                &["title"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let title = match input.get("title").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'title' parameter".into()),
        };

        let description = input.get("description").and_then(|v| v.as_str()).map(String::from);
        let goal_id = input.get("goal_id").and_then(|v| v.as_str()).map(String::from);

        let mut data = read_tracking(&self.data_dir);

        // Validate goal_id if provided
        if let Some(ref gid) = goal_id {
            if !data.goals.iter().any(|g| g.id == *gid) {
                return ToolResult::error(format!("Goal '{}' not found. Use read_tracking to see available goals.", gid));
            }
        }

        let project = Project {
            id: generate_id("proj"),
            title,
            description,
            goal_id,
            status: "active".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            target_date: None,
            completed_at: None,
            notes: None,
        };

        info!("Creating project: {}", project.title);
        let project_id = project.id.clone();
        let project_title = project.title.clone();
        let project_goal_id = project.goal_id.clone();
        data.projects.push(project);
        data.meta.last_updated = chrono::Utc::now().to_rfc3339();

        // Log activity
        self.activity_logger.log_project_created(&project_id, &project_title, project_goal_id.as_deref());

        let goal_info = data.projects.last().unwrap().goal_id.as_ref()
            .and_then(|gid| data.goals.iter().find(|g| g.id == *gid))
            .map(|g| format!(" (linked to: {})", g.title))
            .unwrap_or_default();

        match write_tracking(&self.data_dir, &data) {
            Ok(()) => ToolResult::success(format!(
                "📚 Project created: {}{} ({}.)
Total projects: {}",
                data.projects.last().unwrap().title,
                goal_info,
                data.projects.last().unwrap().id,
                data.projects.len()
            )),
            Err(e) => ToolResult::error(format!("Failed to create project: {e}")),
        }
    }
}

// --- CreateTaskTool ---

pub struct CreateTaskTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl CreateTaskTool {
    pub fn new(data_dir: &str) -> Self {
        CreateTaskTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for CreateTaskTool {
    fn name(&self) -> &str {
        "create_task"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_task".into(),
            description: "Create a new task. Tasks are individual actions. Can link to a project and/or goal. Status starts as 'todo'.".into(),
            input_schema: schema_object(
                json!({
                    "title": {
                        "type": "string",
                        "description": "The task title"
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional description"
                    },
                    "project_id": {
                        "type": "string",
                        "description": "Optional project ID to link to"
                    },
                    "goal_id": {
                        "type": "string",
                        "description": "Optional goal ID to link to"
                    },
                    "due_date": {
                        "type": "string",
                        "description": "Optional due date (ISO 8601 format)"
                    }
                }),
                &["title"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let title = match input.get("title").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'title' parameter".into()),
        };

        let description = input.get("description").and_then(|v| v.as_str()).map(String::from);
        let project_id = input.get("project_id").and_then(|v| v.as_str()).map(String::from);
        let goal_id = input.get("goal_id").and_then(|v| v.as_str()).map(String::from);
        let due_date = input.get("due_date").and_then(|v| v.as_str()).map(String::from);

        let mut data = read_tracking(&self.data_dir);

        // Validate IDs if provided
        if let Some(ref pid) = project_id {
            if !data.projects.iter().any(|p| p.id == *pid) {
                return ToolResult::error(format!("Project '{}' not found", pid));
            }
        }
        if let Some(ref gid) = goal_id {
            if !data.goals.iter().any(|g| g.id == *gid) {
                return ToolResult::error(format!("Goal '{}' not found", gid));
            }
        }

        let task = Task {
            id: generate_id("task"),
            title,
            description,
            project_id,
            goal_id,
            status: "todo".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            due_date,
            completed_at: None,
            notes: None,
        };

        info!("Creating task: {}", task.title);
        let task_id = task.id.clone();
        let task_title = task.title.clone();
        let task_project_id = task.project_id.clone();
        data.tasks.push(task);
        data.meta.last_updated = chrono::Utc::now().to_rfc3339();

        // Log activity
        self.activity_logger.log_task_created(&task_id, &task_title, task_project_id.as_deref());

        match write_tracking(&self.data_dir, &data) {
            Ok(()) => ToolResult::success(format!(
                "✅ Task created: {} ({}.)
Total tasks: {}",
                data.tasks.last().unwrap().title,
                data.tasks.last().unwrap().id,
                data.tasks.len()
            )),
            Err(e) => ToolResult::error(format!("Failed to create task: {e}")),
        }
    }
}

// --- UpdateStatusTool ---

pub struct UpdateStatusTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl UpdateStatusTool {
    pub fn new(data_dir: &str) -> Self {
        UpdateStatusTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for UpdateStatusTool {
    fn name(&self) -> &str {
        "update_status"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "update_status".into(),
            description: "Update the status of a goal, project, or task. Use to mark items as completed, in progress, paused, or todo.".into(),
            input_schema: schema_object(
                json!({
                    "item_type": {
                        "type": "string",
                        "enum": ["goal", "project", "task"],
                        "description": "Type of item to update"
                    },
                    "item_id": {
                        "type": "string",
                        "description": "ID of the item"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["active", "completed", "paused", "todo", "in_progress", "done"],
                        "description": "New status"
                    }
                }),
                &["item_type", "item_id", "status"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let item_type = match input.get("item_type").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'item_type' parameter".into()),
        };

        let item_id = match input.get("item_id").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'item_id' parameter".into()),
        };

        let status = match input.get("status").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'status' parameter".into()),
        };

        let mut data = read_tracking(&self.data_dir);

        let updated = match item_type.as_str() {
            "goal" => {
                if let Some(goal) = data.goals.iter_mut().find(|g| g.id == item_id) {
                    goal.status = status.clone();
                    if status == "completed" {
                        goal.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                    Some(goal.title.clone())
                } else {
                    None
                }
            }
            "project" => {
                if let Some(project) = data.projects.iter_mut().find(|p| p.id == item_id) {
                    project.status = status.clone();
                    if status == "completed" {
                        project.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                    Some(project.title.clone())
                } else {
                    None
                }
            }
            "task" => {
                if let Some(task) = data.tasks.iter_mut().find(|t| t.id == item_id) {
                    task.status = status.clone();
                    if status == "done" {
                        task.completed_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                    Some(task.title.clone())
                } else {
                    None
                }
            }
            _ => return ToolResult::error(format!("Invalid item_type: {}", item_type)),
        };

        match updated {
            Some(title) => {
                data.meta.last_updated = chrono::Utc::now().to_rfc3339();
                
                // Log activity
                self.activity_logger.log_status_update(&item_type, &item_id, &title, &status);
                
                match write_tracking(&self.data_dir, &data) {
                    Ok(()) => ToolResult::success(format!(
                        "✓ {} '{}' marked as {}",
                        item_type, title, status
                    )),
                    Err(e) => ToolResult::error(format!("Failed to update: {e}")),
                }
            }
            None => ToolResult::error(format!("{} '{}' not found", item_type, item_id)),
        }
    }
}

// --- AddNoteTool ---

pub struct AddNoteTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl AddNoteTool {
    pub fn new(data_dir: &str) -> Self {
        AddNoteTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for AddNoteTool {
    fn name(&self) -> &str {
        "add_note"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "add_note".into(),
            description: "Add a note to a goal, project, or task. Use this to record context, observations, or updates about an item. Notes are appended to existing notes with timestamps.".into(),
            input_schema: schema_object(
                json!({
                    "item_type": {
                        "type": "string",
                        "enum": ["goal", "project", "task"],
                        "description": "Type of item to add note to"
                    },
                    "item_id": {
                        "type": "string",
                        "description": "ID of the item"
                    },
                    "note": {
                        "type": "string",
                        "description": "The note to add. Will be appended with timestamp."
                    }
                }),
                &["item_type", "item_id", "note"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let item_type = match input.get("item_type").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'item_type' parameter".into()),
        };

        let item_id = match input.get("item_id").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'item_id' parameter".into()),
        };

        let note_text = match input.get("note").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'note' parameter".into()),
        };

        let mut data = read_tracking(&self.data_dir);
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();
        let new_note = format!("[{}] {}", timestamp, note_text);

        let updated = match item_type.as_str() {
            "goal" => {
                if let Some(goal) = data.goals.iter_mut().find(|g| g.id == item_id) {
                    let current_notes = goal.notes.clone().unwrap_or_default();
                    goal.notes = Some(if current_notes.is_empty() {
                        new_note
                    } else {
                        format!("{}\n{}", current_notes, new_note)
                    });
                    Some(goal.title.clone())
                } else {
                    None
                }
            }
            "project" => {
                if let Some(project) = data.projects.iter_mut().find(|p| p.id == item_id) {
                    let current_notes = project.notes.clone().unwrap_or_default();
                    project.notes = Some(if current_notes.is_empty() {
                        new_note
                    } else {
                        format!("{}\n{}", current_notes, new_note)
                    });
                    Some(project.title.clone())
                } else {
                    None
                }
            }
            "task" => {
                if let Some(task) = data.tasks.iter_mut().find(|t| t.id == item_id) {
                    let current_notes = task.notes.clone().unwrap_or_default();
                    task.notes = Some(if current_notes.is_empty() {
                        new_note
                    } else {
                        format!("{}\n{}", current_notes, new_note)
                    });
                    Some(task.title.clone())
                } else {
                    None
                }
            }
            _ => return ToolResult::error(format!("Invalid item_type: {}", item_type)),
        };

        match updated {
            Some(title) => {
                data.meta.last_updated = chrono::Utc::now().to_rfc3339();
                
                // Log activity
                self.activity_logger.log(ActivityEntry {
                    timestamp: chrono::Utc::now(),
                    action: "added_note".to_string(),
                    item_type: item_type.clone(),
                    item_id: item_id.clone(),
                    item_name: title.clone(),
                    details: Some(note_text),
                });
                
                match write_tracking(&self.data_dir, &data) {
                    Ok(()) => ToolResult::success(format!(
                        "📝 Note added to {} '{}'",
                        item_type, title
                    )),
                    Err(e) => ToolResult::error(format!("Failed to add note: {e}")),
                }
            }
            None => ToolResult::error(format!("{} '{}' not found", item_type, item_id)),
        }
    }
}

// --- RemoveNoteTool ---

pub struct RemoveNoteTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl RemoveNoteTool {
    pub fn new(data_dir: &str) -> Self {
        RemoveNoteTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for RemoveNoteTool {
    fn name(&self) -> &str {
        "remove_note"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "remove_note".into(),
            description: "Remove a specific note from a goal, project, or task by its index (0-based), or clear all notes if index is -1. Returns the updated list of remaining notes.".into(),
            input_schema: schema_object(
                json!({
                    "item_type": {
                        "type": "string",
                        "enum": ["goal", "project", "task"],
                        "description": "Type of item to remove note from"
                    },
                    "item_id": {
                        "type": "string",
                        "description": "ID of the item"
                    },
                    "note_index": {
                        "type": "integer",
                        "description": "Index of note to remove (0 = first, 1 = second, etc.). Use -1 to clear all notes."
                    }
                }),
                &["item_type", "item_id", "note_index"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let item_type = match input.get("item_type").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'item_type' parameter".into()),
        };

        let item_id = match input.get("item_id").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'item_id' parameter".into()),
        };

        let note_index = match input.get("note_index").and_then(|v| v.as_i64()) {
            Some(idx) => idx as i32,
            None => return ToolResult::error("Missing 'note_index' parameter".into()),
        };

        let mut data = read_tracking(&self.data_dir);

        let result = match item_type.as_str() {
            "goal" => {
                if let Some(goal) = data.goals.iter_mut().find(|g| g.id == item_id) {
                    // Clone notes to avoid borrow issues
                    let notes_clone = goal.notes.clone();
                    if let Some(notes) = notes_clone {
                        let note_lines: Vec<&str> = notes.lines().collect();
                        let total_count = note_lines.len();
                        if note_index == -1 {
                            // Clear all notes
                            goal.notes = None;
                            Some((goal.title.clone(), 0, total_count))
                        } else if note_index >= 0 && (note_index as usize) < note_lines.len() {
                            // Remove specific note
                            let mut new_lines: Vec<&str> = note_lines.clone();
                            new_lines.remove(note_index as usize);
                            goal.notes = if new_lines.is_empty() {
                                None
                            } else {
                                Some(new_lines.join("\n"))
                            };
                            Some((goal.title.clone(), new_lines.len(), 1))
                        } else {
                            None // Invalid index
                        }
                    } else {
                        None // No notes to remove
                    }
                } else {
                    None // Goal not found
                }
            }
            "project" => {
                if let Some(project) = data.projects.iter_mut().find(|p| p.id == item_id) {
                    let notes_clone = project.notes.clone();
                    if let Some(notes) = notes_clone {
                        let note_lines: Vec<&str> = notes.lines().collect();
                        let total_count = note_lines.len();
                        if note_index == -1 {
                            project.notes = None;
                            Some((project.title.clone(), 0, total_count))
                        } else if note_index >= 0 && (note_index as usize) < note_lines.len() {
                            let mut new_lines: Vec<&str> = note_lines.clone();
                            new_lines.remove(note_index as usize);
                            project.notes = if new_lines.is_empty() {
                                None
                            } else {
                                Some(new_lines.join("\n"))
                            };
                            Some((project.title.clone(), new_lines.len(), 1))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "task" => {
                if let Some(task) = data.tasks.iter_mut().find(|t| t.id == item_id) {
                    let notes_clone = task.notes.clone();
                    if let Some(notes) = notes_clone {
                        let note_lines: Vec<&str> = notes.lines().collect();
                        let total_count = note_lines.len();
                        if note_index == -1 {
                            task.notes = None;
                            Some((task.title.clone(), 0, total_count))
                        } else if note_index >= 0 && (note_index as usize) < note_lines.len() {
                            let mut new_lines: Vec<&str> = note_lines.clone();
                            new_lines.remove(note_index as usize);
                            task.notes = if new_lines.is_empty() {
                                None
                            } else {
                                Some(new_lines.join("\n"))
                            };
                            Some((task.title.clone(), new_lines.len(), 1))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => return ToolResult::error(format!("Invalid item_type: {}", item_type)),
        };

        match result {
            Some((title, remaining_count, removed_count)) => {
                data.meta.last_updated = chrono::Utc::now().to_rfc3339();
                
                // Log activity
                let action_details = if note_index == -1 {
                    format!("Cleared all {} notes", removed_count)
                } else {
                    format!("Removed note #{}. {} remaining.", note_index + 1, remaining_count)
                };
                
                self.activity_logger.log(ActivityEntry {
                    timestamp: chrono::Utc::now(),
                    action: "removed_note".to_string(),
                    item_type: item_type.clone(),
                    item_id: item_id.clone(),
                    item_name: title.clone(),
                    details: Some(action_details),
                });
                
                match write_tracking(&self.data_dir, &data) {
                    Ok(()) => {
                        let msg = if note_index == -1 {
                            format!("🗑️ All notes cleared from {} '{}'", item_type, title)
                        } else {
                            format!("🗑️ Note removed from {} '{}'. {} notes remaining.", item_type, title, remaining_count)
                        };
                        ToolResult::success(msg)
                    },
                    Err(e) => ToolResult::error(format!("Failed to remove note: {e}")),
                }
            }
            None => ToolResult::error(format!("{} '{}' not found, has no notes, or invalid note index", item_type, item_id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!("sandy_tracking_test_{}", uuid::Uuid::new_v4()))
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn test_create_goal() {
        let dir = test_dir();
        let tool = CreateGoalTool::new(dir.to_str().unwrap());
        
        let result = tool.execute(json!({
            "title": "Get Fit",
            "description": "Exercise regularly"
        })).await;
        
        assert!(!result.is_error);
        assert!(result.content.contains("Get Fit"));
        
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_create_project() {
        let dir = test_dir();
        
        // First create a goal
        let goal_tool = CreateGoalTool::new(dir.to_str().unwrap());
        goal_tool.execute(json!({"title": "Test Goal"})).await;
        
        let data = read_tracking(&dir);
        let goal_id = &data.goals[0].id;
        
        // Then create project linked to goal
        let proj_tool = CreateProjectTool::new(dir.to_str().unwrap());
        let result = proj_tool.execute(json!({
            "title": "Website Redesign",
            "goal_id": goal_id
        })).await;
        
        assert!(!result.is_error);
        
        cleanup(&dir);
    }
}
