use super::super::database::AppState;
use super::super::error::AppError;
use super::super::models::{DashboardStats, OverdueTask};
use tauri::{AppHandle, Emitter, State};

#[tauri::command(rename_all = "snake_case")]
pub fn get_dashboard_stats(state: State<'_, AppState>) -> Result<DashboardStats, AppError> {
    use rusqlite::params;
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let one = |sql: &str| -> Result<i64, AppError> {
        db.query_row(sql, params![uid], |r| r.get(0)).map_err(AppError::from)
    };

    Ok(DashboardStats {
        open_tasks: one("SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.status IN ('Todo','In Progress','Blocked') AND p.user_id = ?1")?,
        in_progress: one("SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.status = 'In Progress' AND p.user_id = ?1")?,
        completed_tasks: one("SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.status = 'Completed' AND p.user_id = ?1")?,
        blocked: one("SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.status = 'Blocked' AND p.user_id = ?1")?,
        projects: one("SELECT COUNT(*) FROM projects WHERE user_id = ?1")?,
        work_log_count: one("SELECT COUNT(*) FROM work_logs w LEFT JOIN projects p ON p.id = w.project_id WHERE COALESCE(p.user_id, w.user_id) = ?1")?,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_overdue_tasks(state: State<'_, AppState>) -> Result<Vec<OverdueTask>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(
        "SELECT t.id, t.key, t.title, p.key, t.due_date
         FROM tasks t JOIN projects p ON p.id = t.project_id
         WHERE t.status IN ('Todo','In Progress','Blocked')
           AND t.due_date IS NOT NULL AND t.due_date != ''
           AND date(t.due_date) < date('now', 'localtime')
           AND p.user_id = ?1
         ORDER BY date(t.due_date) ASC",
    )?;
    let rows = stmt.query_map([uid], |r| {
        Ok(OverdueTask {
            id: r.get(0)?,
            key: r.get(1)?,
            title: r.get(2)?,
            project_key: r.get(3)?,
            due_date: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_workspace_info(state: State<'_, AppState>) -> Result<String, AppError> {
    Ok(state.workspace.root.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn reset_workspace(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    tx.execute("DELETE FROM search_index", [])?;
    tx.execute("DELETE FROM attachments", [])?;
    tx.execute("DELETE FROM work_logs", [])?;
    tx.execute("DELETE FROM task_history", [])?;
    tx.execute("DELETE FROM documents", [])?;
    tx.execute("DELETE FROM comments", [])?;
    tx.execute("DELETE FROM five_whys", [])?;
    tx.execute("DELETE FROM timers", [])?;
    tx.execute("DELETE FROM task_tags", [])?;
    tx.execute("DELETE FROM project_tags", [])?;
    tx.execute("DELETE FROM relationships", [])?;
    tx.execute("DELETE FROM tags", [])?;
    tx.execute("DELETE FROM inbox", [])?;
    tx.execute("DELETE FROM tasks", [])?;
    tx.execute("DELETE FROM projects", [])?;
    tx.commit()?;
    drop(db);
    let _ = app.emit("workspace-reset", ());
    Ok(())
}