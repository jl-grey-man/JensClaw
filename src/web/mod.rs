use axum::{
    extract::State,
    response::Html,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::activity::{ActivityEntry, ActivityLogger};

#[derive(Clone)]
pub struct WebState {
    pub data_dir: String,
    pub activity_logger: Arc<ActivityLogger>,
}

#[derive(Serialize)]
struct DashboardData {
    goals: Vec<GoalView>,
    projects: Vec<ProjectView>,
    tasks: Vec<TaskView>,
    patterns: Vec<PatternView>,
    reminders: Vec<ReminderView>,
    recent_activity: Vec<ActivityEntry>,
}

#[derive(Serialize)]
struct GoalView {
    id: String,
    title: String,
    status: String,
    progress: i32,
    notes: Option<String>,
}

#[derive(Serialize)]
struct ProjectView {
    id: String,
    title: String,
    status: String,
    goal_id: Option<String>,
    goal_title: Option<String>,
    notes: Option<String>,
}

#[derive(Serialize)]
struct TaskView {
    id: String,
    title: String,
    status: String,
    project_id: Option<String>,
    project_title: Option<String>,
    due_date: Option<String>,
    notes: Option<String>,
}

#[derive(Serialize)]
struct PatternView {
    id: String,
    name: String,
    confidence: i32,
    observations_count: i32,
    category: String,
}

#[derive(Serialize)]
struct ReminderView {
    id: i64,
    prompt: String,
    schedule_type: String,
    schedule_value: String,
    next_run: String,
    status: String,
}

pub async fn start_web_server(data_dir: String, port: u16) {
    let activity_logger = Arc::new(ActivityLogger::new(&data_dir));
    
    let state = WebState {
        data_dir: data_dir.clone(),
        activity_logger,
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/dashboard", get(dashboard_handler))
        .route("/api/activity", get(activity_handler))
        .route("/api/tasks", get(tasks_handler))
        .route("/api/goals", get(goals_handler))
        .route("/api/projects", get(projects_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🌐 Sandy Web UI running at http://localhost:{}", port);
    println!("🌐 Access from other devices: http://<your-ip>:{}", port);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> Html<String> {
    Html(include_str!("../../static/index.html").to_string())
}

async fn dashboard_handler(State(state): State<WebState>) -> Json<DashboardData> {
    let tracking_data = load_tracking_data(&state.data_dir);
    let patterns_data = load_patterns_data(&state.data_dir);
    let recent_activity = state.activity_logger.get_entries(50);

    let goals: Vec<GoalView> = tracking_data.goals.iter().map(|g| {
        let total_tasks = tracking_data.tasks.iter().filter(|t| t.goal_id == Some(g.id.clone())).count();
        let completed_tasks = tracking_data.tasks.iter().filter(|t| t.goal_id == Some(g.id.clone()) && t.status == "done").count();
        let progress = if total_tasks > 0 {
            (completed_tasks as i32 * 100) / total_tasks as i32
        } else {
            0
        };

        GoalView {
            id: g.id.clone(),
            title: g.title.clone(),
            status: g.status.clone(),
            progress,
            notes: g.notes.clone(),
        }
    }).collect();

    let projects: Vec<ProjectView> = tracking_data.projects.iter().map(|p| {
        let goal_title = p.goal_id.as_ref().and_then(|gid| {
            tracking_data.goals.iter().find(|g| g.id == *gid).map(|g| g.title.clone())
        });

        ProjectView {
            id: p.id.clone(),
            title: p.title.clone(),
            status: p.status.clone(),
            goal_id: p.goal_id.clone(),
            goal_title,
            notes: p.notes.clone(),
        }
    }).collect();

    let tasks: Vec<TaskView> = tracking_data.tasks.iter().map(|t| {
        let project_title = t.project_id.as_ref().and_then(|pid| {
            tracking_data.projects.iter().find(|p| p.id == *pid).map(|p| p.title.clone())
        });

        TaskView {
            id: t.id.clone(),
            title: t.title.clone(),
            status: t.status.clone(),
            project_id: t.project_id.clone(),
            project_title,
            due_date: t.due_date.clone(),
            notes: t.notes.clone(),
        }
    }).collect();

    let patterns: Vec<PatternView> = patterns_data.patterns.iter().map(|p| {
        PatternView {
            id: p.id.clone(),
            name: p.name.clone(),
            confidence: p.confidence,
            observations_count: p.observations_count,
            category: p.category.clone(),
        }
    }).collect();

    // Load reminders from tracking.json
    let reminders: Vec<ReminderView> = tracking_data.reminders.iter().map(|r| {
        ReminderView {
            id: r.id.replace("rem_", "").parse().unwrap_or(0),
            prompt: r.message.clone(),
            schedule_type: if r.is_recurring { "cron".to_string() } else { "once".to_string() },
            schedule_value: r.schedule.clone(),
            next_run: r.schedule.clone(),
            status: "active".to_string(),
        }
    }).collect();

    Json(DashboardData {
        goals,
        projects,
        tasks,
        patterns,
        reminders,
        recent_activity,
    })
}

async fn activity_handler(State(state): State<WebState>) -> Json<Vec<ActivityEntry>> {
    Json(state.activity_logger.get_entries(100))
}

async fn tasks_handler(State(state): State<WebState>) -> Json<Vec<TaskView>> {
    let tracking_data = load_tracking_data(&state.data_dir);
    
    let tasks: Vec<TaskView> = tracking_data.tasks.iter().map(|t| {
        let project_title = t.project_id.as_ref().and_then(|pid| {
            tracking_data.projects.iter().find(|p| p.id == *pid).map(|p| p.title.clone())
        });

        TaskView {
            id: t.id.clone(),
            title: t.title.clone(),
            status: t.status.clone(),
            project_id: t.project_id.clone(),
            project_title,
            due_date: t.due_date.clone(),
            notes: t.notes.clone(),
        }
    }).collect();

    Json(tasks)
}

async fn goals_handler(State(state): State<WebState>) -> Json<Vec<GoalView>> {
    let tracking_data = load_tracking_data(&state.data_dir);
    
    let goals: Vec<GoalView> = tracking_data.goals.iter().map(|g| {
        let total_tasks = tracking_data.tasks.iter().filter(|t| t.goal_id == Some(g.id.clone())).count();
        let completed_tasks = tracking_data.tasks.iter().filter(|t| t.goal_id == Some(g.id.clone()) && t.status == "done").count();
        let progress = if total_tasks > 0 {
            (completed_tasks as i32 * 100) / total_tasks as i32
        } else {
            0
        };

        GoalView {
            id: g.id.clone(),
            title: g.title.clone(),
            status: g.status.clone(),
            progress,
            notes: g.notes.clone(),
        }
    }).collect();

    Json(goals)
}

async fn projects_handler(State(state): State<WebState>) -> Json<Vec<ProjectView>> {
    let tracking_data = load_tracking_data(&state.data_dir);
    
    let projects: Vec<ProjectView> = tracking_data.projects.iter().map(|p| {
        let goal_title = p.goal_id.as_ref().and_then(|gid| {
            tracking_data.goals.iter().find(|g| g.id == *gid).map(|g| g.title.clone())
        });

        ProjectView {
            id: p.id.clone(),
            title: p.title.clone(),
            status: p.status.clone(),
            goal_id: p.goal_id.clone(),
            goal_title,
            notes: p.notes.clone(),
        }
    }).collect();

    Json(projects)
}

#[derive(serde::Deserialize)]
struct TrackingData {
    goals: Vec<TrackingGoal>,
    projects: Vec<TrackingProject>,
    tasks: Vec<TrackingTask>,
    reminders: Vec<TrackingReminder>,
}

#[derive(serde::Deserialize)]
struct TrackingReminder {
    id: String,
    message: String,
    schedule: String,
    linked_to: Option<String>,
    is_recurring: bool,
    created_at: String,
}

#[derive(serde::Deserialize)]
struct TrackingGoal {
    id: String,
    title: String,
    status: String,
    notes: Option<String>,
}

#[derive(serde::Deserialize)]
struct TrackingProject {
    id: String,
    title: String,
    status: String,
    goal_id: Option<String>,
    notes: Option<String>,
}

#[derive(serde::Deserialize)]
struct TrackingTask {
    id: String,
    title: String,
    status: String,
    project_id: Option<String>,
    goal_id: Option<String>,
    due_date: Option<String>,
    notes: Option<String>,
}

#[derive(serde::Deserialize)]
struct PatternsData {
    patterns: Vec<PatternData>,
}

#[derive(serde::Deserialize)]
struct PatternData {
    id: String,
    name: String,
    confidence: i32,
    observations_count: i32,
    category: String,
}

fn load_tracking_data(data_dir: &str) -> TrackingData {
    let path = PathBuf::from(data_dir).join("tracking.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| TrackingData {
            goals: vec![],
            projects: vec![],
            tasks: vec![],
            reminders: vec![],
        }),
        Err(_) => TrackingData {
            goals: vec![],
            projects: vec![],
            tasks: vec![],
            reminders: vec![],
        },
    }
}

fn load_patterns_data(data_dir: &str) -> PatternsData {
    let path = PathBuf::from(data_dir).join("patterns.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| PatternsData {
            patterns: vec![],
        }),
        Err(_) => PatternsData {
            patterns: vec![],
        },
    }
}
