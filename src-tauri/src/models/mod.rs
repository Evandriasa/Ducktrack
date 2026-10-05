use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub key: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub total: i64,
    pub completed: i64,
    pub in_progress: i64,
    pub blocked: i64,
    pub open: i64,
    pub progress: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectNode {
    #[serde(flatten)]
    pub project: Project,
    pub children: Vec<ProjectNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectInput {
    pub parent_id: Option<i64>,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub parent_id: Option<i64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub project_id: i64,
    pub parent_task_id: Option<i64>,
    pub key: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub task_type: String,
    pub due_date: Option<String>,
    pub assignee: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskInput {
    pub project_id: i64,
    pub parent_task_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub task_type: Option<String>,
    pub due_date: Option<String>,
    pub assignee: Option<String>,
    pub estimated_minutes: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub task_type: Option<String>,
    pub due_date: Option<String>,
    pub assignee: Option<String>,
    pub parent_task_id: Option<i64>,
    pub estimated_minutes: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct TaskFilter {
    pub project_id: Option<i64>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub tag_id: Option<i64>,
    pub include_subtasks: Option<bool>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskWithProject {
    #[serde(flatten)]
    pub task: Task,
    pub project_key: String,
    pub project_name: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub task_id: i64,
    pub task_key: String,
    pub task_title: String,
    pub project_key: String,
    pub action: String,
    pub field: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: String,
    pub user_id: Option<i64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkLog {
    pub id: i64,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub description: String,
    pub duration_minutes: Option<i64>,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkLogWithRefs {
    #[serde(flatten)]
    pub work_log: WorkLog,
    pub project_key: Option<String>,
    pub project_name: Option<String>,
    pub task_key: Option<String>,
    pub task_title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkLogInput {
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub description: String,
    pub duration_minutes: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkLogUpdate {
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub description: Option<String>,
    pub duration_minutes: Option<i64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentWithProject {
    #[serde(flatten)]
    pub document: Document,
    pub project_key: String,
    pub project_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DocumentInput {
    pub project_id: i64,
    pub title: String,
    pub content: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DocumentUpdate {
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HistoryFilter {
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub action: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub open_tasks: i64,
    pub in_progress: i64,
    pub completed_tasks: i64,
    pub blocked: i64,
    pub projects: i64,
    pub work_log_count: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UserInput {
    pub name: String,
    pub password: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct UserUpdate {
    pub name: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub category: String,
    pub items: Vec<SearchItem>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub title: String,
    pub body: String,
    pub ref_id: i64,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TagWithCount {
    pub id: i64,
    pub name: String,
    pub task_count: i64,
    pub project_count: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TagInput {
    pub name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TagUpdate {
    pub name: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: i64,
    pub task_id: i64,
    pub user_id: Option<i64>,
    pub user_name: Option<String>,
    pub content: String,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CommentInput {
    pub task_id: i64,
    pub content: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub id: i64,
    pub source_task_id: i64,
    pub target_task_id: i64,
    pub rel_type: String,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDetail {
    pub id: i64,
    pub rel_type: String,
    pub direction: String,
    pub task_id: i64,
    pub task_key: String,
    pub task_title: String,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RelationshipInput {
    pub source_task_id: i64,
    pub target_task_id: i64,
    pub rel_type: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    pub id: i64,
    pub text: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub promoted_task_id: Option<i64>,
    pub promoted_project_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InboxInput {
    pub text: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InboxUpdate {
    pub text: Option<String>,
    pub completed: Option<bool>,
    pub promoted_task_id: Option<i64>,
    pub promoted_project_id: Option<i64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTimer {
    pub id: i64,
    pub task_id: i64,
    pub task_key: String,
    pub task_title: String,
    pub started_at: String,
    pub note: String,
    pub elapsed_minutes: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TimerStartInput {
    pub task_id: i64,
    pub note: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerStopped {
    #[serde(flatten)]
    pub work_log: WorkLog,
    pub task_key: Option<String>,
    pub elapsed_minutes: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OverdueTask {
    pub id: i64,
    pub key: String,
    pub title: String,
    pub project_key: String,
    pub due_date: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockedTask {
    pub task_id: i64,
    pub key: String,
    pub title: String,
    pub project_key: String,
    pub blockers: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DailySummary {
    pub date: String,
    pub minutes: i64,
    pub log_count: i64,
    pub work_logs: Vec<WorkLogWithRefs>,
    pub history: Vec<HistoryEntry>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PeriodSummary {
    pub minutes: i64,
    pub log_count: i64,
    pub tasks_completed: i64,
    pub by_project: Vec<ProjectHours>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHours {
    pub project_key: String,
    pub project_name: String,
    pub minutes: i64,
    pub logs: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FiveWhy {
    pub id: i64,
    pub user_id: i64,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub project_key: Option<String>,
    pub project_name: Option<String>,
    pub task_key: Option<String>,
    pub task_title: Option<String>,
    pub problem: String,
    pub why_1: Option<String>,
    pub why_2: Option<String>,
    pub why_3: Option<String>,
    pub why_4: Option<String>,
    pub why_5: Option<String>,
    pub root_cause: Option<String>,
    pub corrective_action: Option<String>,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FiveWhyInput {
    pub problem: String,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub why_1: Option<String>,
    pub why_2: Option<String>,
    pub why_3: Option<String>,
    pub why_4: Option<String>,
    pub why_5: Option<String>,
    pub root_cause: Option<String>,
    pub corrective_action: Option<String>,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct FiveWhyUpdate {
    pub problem: Option<String>,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub why_1: Option<String>,
    pub why_2: Option<String>,
    pub why_3: Option<String>,
    pub why_4: Option<String>,
    pub why_5: Option<String>,
    pub root_cause: Option<String>,
    pub corrective_action: Option<String>,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub status: Option<String>,
}