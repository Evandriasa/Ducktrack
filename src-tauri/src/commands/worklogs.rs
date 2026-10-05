use rusqlite::{params, Row};
use tauri::{AppHandle, Emitter, State};

use super::super::database::{indexer, now, AppState};
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{WorkLog, WorkLogInput, WorkLogUpdate, WorkLogWithRefs};

pub(crate) fn worklog_with_refs_from_row(row: &Row) -> rusqlite::Result<WorkLogWithRefs> {
    let wl = WorkLog {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        description: row.get(3)?,
        duration_minutes: row.get(4)?,
        created_at: row.get(5)?,
    };
    Ok(WorkLogWithRefs {
        work_log: wl,
        project_key: row.get(6)?,
        project_name: row.get(7)?,
        task_key: row.get(8)?,
        task_title: row.get(9)?,
    })
}

const WORKLOG_SELECT: &str = "SELECT w.id, w.project_id, w.task_id, w.description, w.duration_minutes, w.created_at,
        p.key AS pkey, p.name AS pname, t.key AS tkey, t.title AS ttitle
     FROM work_logs w
     LEFT JOIN projects p ON p.id = w.project_id
     LEFT JOIN tasks t ON t.id = w.task_id";

#[tauri::command(rename_all = "snake_case")]
pub fn create_work_log(
    app: AppHandle,
    state: State<'_, AppState>,
    input: WorkLogInput,
) -> Result<WorkLog, AppError> {
    ensure_non_empty(&input.description, "Description")?;

    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let ts = now();

    if let Some(pid) = input.project_id {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![pid, uid],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
    }
    if let Some(tid) = input.task_id {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
            params![tid, uid],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
    }

    tx.execute(
        "INSERT INTO work_logs (project_id, task_id, description, duration_minutes, created_at, user_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![input.project_id, input.task_id, input.description.trim(), input.duration_minutes, ts, uid],
    )?;

    let id = tx.last_insert_rowid();
    match (input.task_id, input.project_id) {
        (Some(tid), _) => {
            let task_key: String = tx.query_row("SELECT key FROM tasks WHERE id = ?1", [tid], |r| r.get(0))?;
            indexer::upsert(&tx, "worklog", id, uid, &format!("Work log on {task_key}"), &input.description)?;
        }
        (None, Some(pid)) => {
            let project_key: String = tx.query_row("SELECT key FROM projects WHERE id = ?1", [pid], |r| r.get(0))?;
            indexer::upsert(&tx, "worklog", id, uid, &format!("Work log in {project_key}"), &input.description)?;
        }
        (None, None) => {
            indexer::upsert(&tx, "worklog", id, uid, "Work log", &input.description)?;
        }
    }
    tx.commit()?;

    let updated = get_work_log_inner(&db, id, uid)?;
    drop(db);
    let _ = app.emit("worklog-created", id);
    Ok(updated.work_log)
}

fn get_work_log_inner(db: &rusqlite::Connection, id: i64, uid: i64) -> Result<WorkLogWithRefs, AppError> {
    db.query_row(
        &format!("{WORKLOG_SELECT} WHERE w.id = ?1 AND COALESCE(p.user_id, w.user_id, ?2) = ?2"),
        params![id, uid],
        worklog_with_refs_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Work log {id} not found.")),
        other => AppError::Db(other),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_work_logs(state: State<'_, AppState>) -> Result<Vec<WorkLogWithRefs>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(&format!("{WORKLOG_SELECT} WHERE COALESCE(p.user_id, w.user_id) = ?1 ORDER BY w.created_at DESC"))?;
    let rows = stmt.query_map([uid], worklog_with_refs_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_work_log(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: WorkLogUpdate,
) -> Result<WorkLog, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let owned: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM work_logs w LEFT JOIN projects p ON p.id = w.project_id WHERE w.id = ?1 AND COALESCE(p.user_id, w.user_id) = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Work log {id} not found.")));
    }

    let mut sets: Vec<&str> = Vec::new();
    let mut values: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(desc) = &input.description {
        if desc.trim().is_empty() {
            return Err(AppError::Validation("Description must not be empty.".into()));
        }
        sets.push("description = ?");
        values.push(rusqlite::types::Value::Text(desc.trim().to_string()));
    }
    if let Some(dur) = input.duration_minutes {
        sets.push("duration_minutes = ?");
        values.push(rusqlite::types::Value::Integer(dur));
    }
    if let Some(pid) = input.project_id {
        let owned_pj: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![pid, uid],
            |r| r.get(0),
        )?;
        if !owned_pj {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
        sets.push("project_id = ?");
        values.push(rusqlite::types::Value::Integer(pid));
    }
    if let Some(tid) = input.task_id {
        let owned_tk: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
            params![tid, uid],
            |r| r.get(0),
        )?;
        if !owned_tk {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
        sets.push("task_id = ?");
        values.push(rusqlite::types::Value::Integer(tid));
    }

    if sets.is_empty() {
        return Ok(get_work_log_inner(&tx, id, uid)?.work_log);
    }

    let sql = format!("UPDATE work_logs SET {} WHERE id = ?{}", sets.join(", "), values.len() + 1);
    values.push(rusqlite::types::Value::Integer(id));
    tx.prepare(&sql)?.execute(rusqlite::params_from_iter(values.iter()))?;

    let updated = get_work_log_inner(&tx, id, uid)?;
    indexer::upsert(&tx, "worklog", id, uid, &format!("Work log #{}", id), &updated.work_log.description)?;
    tx.commit()?;
    drop(db);
    let _ = app.emit("worklog-updated", id);
    Ok(updated.work_log)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_work_log(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let owned: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM work_logs w LEFT JOIN projects p ON p.id = w.project_id WHERE w.id = ?1 AND COALESCE(p.user_id, w.user_id) = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Work log {id} not found.")));
    }
    let _ = indexer::remove(&tx, "worklog", id);
    let deleted = tx.execute("DELETE FROM work_logs WHERE id = ?1", [id])?;
    tx.commit()?;
    drop(db);
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Work log {id} not found.")));
    }
    let _ = app.emit("worklog-deleted", id);
    Ok(())
}