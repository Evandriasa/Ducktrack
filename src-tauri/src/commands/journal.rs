use rusqlite::params;
use tauri::State;

use super::super::database::AppState;
use super::super::error::AppError;
use super::super::models::{
    BlockedTask, DailySummary, HistoryEntry, PeriodSummary, ProjectHours, WorkLogWithRefs,
};

use super::history::history_entry_from_row;
use super::worklogs::worklog_with_refs_from_row;

const WORKLOG_SELECT: &str = "SELECT w.id, w.project_id, w.task_id, w.description, w.duration_minutes, w.created_at,
        p.key AS pkey, p.name AS pname, t.key AS tkey, t.title AS ttitle
     FROM work_logs w
     LEFT JOIN projects p ON p.id = w.project_id
     LEFT JOIN tasks t ON t.id = w.task_id";

#[tauri::command(rename_all = "snake_case")]
pub fn get_daily_summary(
    state: State<'_, AppState>,
    date: Option<String>,
) -> Result<DailySummary, AppError> {
    let uid = state.active_user()?;
    let day = date.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let mut logs_stmt = db.prepare(&format!(
        "{WORKLOG_SELECT} WHERE date(w.created_at, 'localtime') = ?1 AND COALESCE(p.user_id, w.user_id) = ?2 ORDER BY w.created_at DESC, w.id DESC"
    ))?;
    let work_logs: Vec<WorkLogWithRefs> = logs_stmt
        .query_map(params![day, uid], worklog_with_refs_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let (minutes, log_count): (i64, i64) = db.query_row(
        "SELECT COALESCE(SUM(w.duration_minutes), 0), COUNT(*)
         FROM work_logs w LEFT JOIN projects p ON p.id = w.project_id
         WHERE date(w.created_at, 'localtime') = ?1 AND COALESCE(p.user_id, w.user_id) = ?2",
        params![day, uid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    let history: Vec<HistoryEntry> = {
        let mut stmt = db.prepare(&format!(
            "SELECT h.id, h.task_id, t.key, t.title, p.key,
                    h.action, h.field, h.old_value, h.new_value, h.created_at, h.user_id
             FROM task_history h
             JOIN tasks t ON t.id = h.task_id
             JOIN projects p ON p.id = t.project_id
             WHERE date(h.created_at, 'localtime') = ?1 AND p.user_id = ?2
             ORDER BY h.created_at ASC, h.id ASC"
        ))?;
        let rows = stmt.query_map(params![day, uid], history_entry_from_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    Ok(DailySummary {
        date: day,
        minutes,
        log_count,
        work_logs,
        history,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_summary_report(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<PeriodSummary, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let (minutes, log_count): (i64, i64) = db.query_row(
        "SELECT COALESCE(SUM(w.duration_minutes), 0), COUNT(*)
         FROM work_logs w LEFT JOIN projects p ON p.id = w.project_id
         WHERE date(w.created_at, 'localtime') BETWEEN ?1 AND ?2 AND COALESCE(p.user_id, w.user_id) = ?3",
        params![from, to, uid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    let tasks_completed: i64 = db.query_row(
        "SELECT COUNT(*) FROM task_history h
         JOIN tasks t ON t.id = h.task_id
         JOIN projects p ON p.id = t.project_id
         WHERE h.action = 'completed' AND date(h.created_at, 'localtime') BETWEEN ?1 AND ?2 AND p.user_id = ?3",
        params![from, to, uid],
        |r| r.get(0),
    )?;

    let mut stmt = db.prepare(
        "SELECT p.key, p.name, COALESCE(SUM(w.duration_minutes), 0), COUNT(*)
         FROM work_logs w LEFT JOIN projects p ON p.id = w.project_id
         WHERE date(w.created_at, 'localtime') BETWEEN ?1 AND ?2 AND COALESCE(p.user_id, w.user_id) = ?3
         GROUP BY p.id, p.key, p.name
         ORDER BY SUM(w.duration_minutes) DESC, p.name ASC",
    )?;
    let by_project: Vec<ProjectHours> = stmt
        .query_map(params![from, to, uid], |r| {
            Ok(ProjectHours {
                project_key: r.get::<_, Option<String>>(0)?.unwrap_or_else(|| "-".into()),
                project_name: r.get::<_, Option<String>>(1)?.unwrap_or_else(|| "No project".into()),
                minutes: r.get(2)?,
                logs: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PeriodSummary {
        minutes,
        log_count,
        tasks_completed,
        by_project,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_blocked_tasks(state: State<'_, AppState>) -> Result<Vec<BlockedTask>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let mut stmt = db.prepare(
        "SELECT t.id, t.key, t.title, p.key
         FROM tasks t JOIN projects p ON p.id = t.project_id
         WHERE t.status = 'Blocked' AND p.user_id = ?1
         ORDER BY t.updated_at DESC",
    )?;
    let tasks: Vec<(i64, String, String, String)> = stmt
        .query_map([uid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut result: Vec<BlockedTask> = Vec::new();
    for (task_id, key, title, project_key) in tasks {
        let mut blockers_stmt = db.prepare(
            "SELECT t.key FROM relationships r
             JOIN tasks t ON t.id = CASE
                 WHEN r.type = 'blocks' THEN r.source_task_id
                 WHEN r.type = 'blocked_by' THEN r.target_task_id
                 ELSE r.source_task_id END
             WHERE (r.type = 'blocks' AND r.target_task_id = ?1)
                OR (r.type = 'blocked_by' AND r.source_task_id = ?1)",
        )?;
        let blockers: Vec<String> = blockers_stmt
            .query_map([task_id], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        result.push(BlockedTask {
            task_id,
            key,
            title,
            project_key,
            blockers,
        });
    }
    Ok(result)
}