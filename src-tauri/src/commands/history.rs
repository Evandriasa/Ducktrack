use rusqlite::{params, Row};
use tauri::State;

use super::super::database::AppState;
use super::super::error::AppError;
use super::super::models::{HistoryEntry, HistoryFilter};

pub fn history_entry_from_row(row: &Row) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        task_id: row.get(1)?,
        task_key: row.get(2)?,
        task_title: row.get(3)?,
        project_key: row.get(4)?,
        action: row.get(5)?,
        field: row.get(6)?,
        old_value: row.get(7)?,
        new_value: row.get(8)?,
        created_at: row.get(9)?,
        user_id: row.get(10)?,
    })
}

const HISTORY_SELECT: &str = "SELECT h.id, h.task_id, t.key, t.title, p.key,
            h.action, h.field, h.old_value, h.new_value, h.created_at, h.user_id
     FROM task_history h
     JOIN tasks t ON t.id = h.task_id
     JOIN projects p ON p.id = t.project_id";

#[tauri::command(rename_all = "snake_case")]
pub fn get_task_history(state: State<'_, AppState>, task_id: i64) -> Result<Vec<HistoryEntry>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(&format!("{HISTORY_SELECT} WHERE h.task_id = ?1 AND p.user_id = ?2 ORDER BY h.created_at ASC, h.id ASC"))?;
    let rows = stmt.query_map(params![task_id, uid], history_entry_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_project_history(
    state: State<'_, AppState>,
    project_id: i64,
    limit: Option<i64>,
) -> Result<Vec<HistoryEntry>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let limit = limit.unwrap_or(200);

    let mut stmt = db.prepare(&format!(
        "{HISTORY_SELECT} WHERE h.task_id IN (
            SELECT id FROM tasks WHERE project_id = ?1
            UNION
            SELECT id FROM tasks WHERE project_id IN (
                WITH RECURSIVE sub(id) AS (
                    SELECT id FROM projects WHERE id = ?1 AND user_id = ?2
                    UNION ALL
                    SELECT p.id FROM projects p JOIN sub s ON p.parent_id = s.id
                ) SELECT id FROM sub
            )
        )
        AND p.user_id = ?2
        ORDER BY h.created_at DESC, h.id DESC LIMIT ?3"
    ))?;
    let rows = stmt.query_map(params![project_id, uid, limit], history_entry_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_global_history(
    state: State<'_, AppState>,
    filter: Option<HistoryFilter>,
) -> Result<Vec<HistoryEntry>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let filter = filter.unwrap_or(HistoryFilter {
        project_id: None,
        task_id: None,
        action: None,
        limit: Some(200),
    });

    let mut sql = HISTORY_SELECT.to_string();
    let mut values: Vec<rusqlite::types::Value> = Vec::new();
    let mut where_clauses: Vec<String> = vec!["p.user_id = ?".to_string()];
    values.push(rusqlite::types::Value::Integer(uid));

    if let Some(pid) = filter.project_id {
        where_clauses.push("p.id = ?".to_string());
        values.push(rusqlite::types::Value::Integer(pid));
    }
    if let Some(tid) = filter.task_id {
        where_clauses.push("h.task_id = ?".to_string());
        values.push(rusqlite::types::Value::Integer(tid));
    }
    if let Some(action) = &filter.action {
        where_clauses.push("h.action = ?".to_string());
        values.push(rusqlite::types::Value::Text(action.clone()));
    }

    sql.push_str(" WHERE ");
    sql.push_str(&where_clauses.join(" AND "));
    sql.push_str(" ORDER BY h.created_at DESC, h.id DESC LIMIT ?");
    values.push(rusqlite::types::Value::Integer(filter.limit.unwrap_or(200)));

    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(values.iter()), history_entry_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}