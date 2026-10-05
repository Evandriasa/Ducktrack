use rusqlite::{params, Row};
use tauri::{AppHandle, Emitter, State};

use super::super::database::{now, AppState};
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{InboxInput, InboxItem, InboxUpdate};

fn inbox_from_row(row: &Row) -> rusqlite::Result<InboxItem> {
    Ok(InboxItem {
        id: row.get(0)?,
        text: row.get(1)?,
        created_at: row.get(2)?,
        completed_at: row.get(3)?,
        promoted_task_id: row.get(4)?,
        promoted_project_id: row.get(5)?,
    })
}

const INBOX_SELECT: &str = "SELECT id, text, created_at, completed_at, promoted_task_id, promoted_project_id FROM inbox";

fn list_open_inner(db: &rusqlite::Connection, uid: i64) -> Result<Vec<InboxItem>, AppError> {
    let mut stmt = db.prepare(&format!(
        "{INBOX_SELECT} WHERE user_id = ?1 AND completed_at IS NULL ORDER BY created_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map([uid], inbox_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_inbox(
    state: State<'_, AppState>,
    include_completed: Option<bool>,
) -> Result<Vec<InboxItem>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    if include_completed.unwrap_or(false) {
        let mut stmt = db.prepare(&format!(
            "{INBOX_SELECT} WHERE user_id = ?1 ORDER BY created_at DESC, id DESC"
        ))?;
        let rows = stmt.query_map([uid], inbox_from_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    } else {
        list_open_inner(&db, uid)
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_inbox_item(
    app: AppHandle,
    state: State<'_, AppState>,
    input: InboxInput,
) -> Result<InboxItem, AppError> {
    ensure_non_empty(&input.text, "Text")?;
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let ts = now();
    tx.execute(
        "INSERT INTO inbox (user_id, text, created_at) VALUES (?1, ?2, ?3)",
        params![uid, input.text.trim(), ts],
    )?;
    let id = tx.last_insert_rowid();
    let item: InboxItem = tx.query_row(&format!("{INBOX_SELECT} WHERE id = ?1"), [id], inbox_from_row)?;
    tx.commit()?;
    drop(db);
    let _ = app.emit("inbox-updated", ());
    Ok(item)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_inbox_item(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: InboxUpdate,
) -> Result<InboxItem, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let owned: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM inbox WHERE id = ?1 AND user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Inbox item {id} not found.")));
    }

    let mut sets: Vec<&str> = Vec::new();
    let mut values: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(text) = &input.text {
        if text.trim().is_empty() {
            return Err(AppError::Validation("Text must not be empty.".into()));
        }
        sets.push("text = ?");
        values.push(rusqlite::types::Value::Text(text.trim().to_string()));
    }
    if let Some(completed) = input.completed {
        sets.push("completed_at = ?");
        if completed {
            values.push(rusqlite::types::Value::Text(now()));
        } else {
            values.push(rusqlite::types::Value::Null);
        }
    }
    if let Some(tid) = input.promoted_task_id {
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
            params![tid, uid],
            |r| r.get(0),
        )?;
        if !valid {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
        sets.push("promoted_task_id = ?");
        values.push(rusqlite::types::Value::Integer(tid));
    }
    if let Some(pid) = input.promoted_project_id {
        let valid: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![pid, uid],
            |r| r.get(0),
        )?;
        if !valid {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
        sets.push("promoted_project_id = ?");
        values.push(rusqlite::types::Value::Integer(pid));
    }

    if !sets.is_empty() {
        let sql = format!("UPDATE inbox SET {} WHERE id = ?{}", sets.join(", "), values.len() + 1);
        values.push(rusqlite::types::Value::Integer(id));
        tx.prepare(&sql)?.execute(rusqlite::params_from_iter(values.iter()))?;
    }

    let item: InboxItem = tx.query_row(&format!("{INBOX_SELECT} WHERE id = ?1"), [id], inbox_from_row)?;
    tx.commit()?;
    drop(db);
    let _ = app.emit("inbox-updated", ());
    Ok(item)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_inbox_item(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let deleted = tx.execute(
        "DELETE FROM inbox WHERE id = ?1 AND user_id = ?2",
        params![id, uid],
    )?;
    tx.commit()?;
    drop(db);
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Inbox item {id} not found.")));
    }
    let _ = app.emit("inbox-updated", ());
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_inbox_count(state: State<'_, AppState>) -> Result<i64, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM inbox WHERE user_id = ?1 AND completed_at IS NULL",
        [uid],
        |r| r.get(0),
    )?;
    Ok(count)
}