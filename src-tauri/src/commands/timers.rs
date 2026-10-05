use rusqlite::{params, OptionalExtension, Row};
use tauri::{AppHandle, Emitter, State};
use chrono::DateTime;

use super::super::database::{indexer, now, AppState};
use super::super::error::AppError;
use super::super::models::{ActiveTimer, TimerStartInput, TimerStopped, WorkLog};

fn active_timer_from_row(row: &Row) -> rusqlite::Result<ActiveTimer> {
    let started: String = row.get(3)?;
    let elapsed = elapsed_minutes(&started);
    Ok(ActiveTimer {
        id: row.get(0)?,
        task_id: row.get(1)?,
        task_key: row.get(2)?,
        task_title: row.get(4)?,
        started_at: started,
        note: row.get(5)?,
        elapsed_minutes: elapsed,
    })
}

fn elapsed_minutes(started_at: &str) -> i64 {
    match DateTime::parse_from_rfc3339(started_at) {
        Ok(start) => {
            let start_utc = start.with_timezone(&chrono::Utc);
            let mins = (chrono::Utc::now() - start_utc).num_minutes();
            if mins < 0 { 0 } else { mins }
        }
        Err(_) => 0,
    }
}

fn get_active_inner(db: &rusqlite::Connection, uid: i64) -> Result<Option<ActiveTimer>, AppError> {
    let mut stmt = db.prepare(
        "SELECT tm.id, tm.task_id, t.key, tm.started_at, t.title, tm.note
         FROM timers tm JOIN tasks t ON t.id = tm.task_id
         WHERE tm.user_id = ?1 ORDER BY tm.id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map([uid], active_timer_from_row)?;
    match rows.next() {
        Some(Ok(timer)) => Ok(Some(timer)),
        Some(Err(e)) => Err(AppError::Db(e)),
        None => Ok(None),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_active_timer(state: State<'_, AppState>) -> Result<Option<ActiveTimer>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    get_active_inner(&db, uid)
}

#[tauri::command(rename_all = "snake_case")]
pub fn start_timer(
    app: AppHandle,
    state: State<'_, AppState>,
    input: TimerStartInput,
) -> Result<ActiveTimer, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let owned: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
        params![input.task_id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Task {} not found.", input.task_id)));
    }

    let active: Option<i64> = tx
        .query_row("SELECT id FROM timers WHERE user_id = ?1 LIMIT 1", [uid], |r| r.get(0))
        .optional()?;
    if active.is_some() {
        return Err(AppError::Validation("A timer is already running. Stop it first.".into()));
    }

    let ts = now();
    let note = input.note.unwrap_or_default().trim().to_string();
    tx.execute(
        "INSERT INTO timers (user_id, task_id, started_at, note) VALUES (?1, ?2, ?3, ?4)",
        params![uid, input.task_id, ts, note],
    )?;

    let timer: ActiveTimer = tx.query_row(
        "SELECT tm.id, tm.task_id, t.key, tm.started_at, t.title, tm.note
         FROM timers tm JOIN tasks t ON t.id = tm.task_id WHERE tm.id = ?1",
        [tx.last_insert_rowid()],
        active_timer_from_row,
    )?;
    tx.commit()?;
    drop(db);
    let _ = app.emit("timer-changed", serde_json::to_value(&timer).unwrap_or_default());
    Ok(timer)
}

#[tauri::command(rename_all = "snake_case")]
pub fn stop_timer(
    app: AppHandle,
    state: State<'_, AppState>,
    description: Option<String>,
) -> Result<TimerStopped, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let timer = get_active_inner(&tx, uid)?;
    let timer = timer.ok_or_else(|| AppError::NotFound("No active timer.".into()))?;

    let elapsed = elapsed_minutes(&timer.started_at).max(1);
    let description = description
        .filter(|d| !d.trim().is_empty())
        .map(|d| d.trim().to_string())
        .unwrap_or_else(|| format!("Time logged on {}", timer.task_key));

    let created = now();
    tx.execute(
        "INSERT INTO work_logs (project_id, task_id, description, duration_minutes, created_at, user_id)
         VALUES (
             (SELECT project_id FROM tasks WHERE id = ?1), ?1, ?2, ?3, ?4, ?5)",
        params![timer.task_id, description, elapsed, created, uid],
    )?;
    let work_log_id = tx.last_insert_rowid();
    indexer::upsert(&tx, "worklog", work_log_id, uid, &format!("Time logged on {}", timer.task_key), &description)?;

    tx.execute("DELETE FROM timers WHERE id = ?1", [timer.id])?;
    tx.commit()?;
    drop(db);

    let work_log = WorkLog {
        id: work_log_id,
        project_id: None,
        task_id: Some(timer.task_id),
        description,
        duration_minutes: Some(elapsed),
        created_at: created,
    };
    let _ = app.emit("timer-changed", ());
    let _ = app.emit("worklog-created", work_log_id);
    Ok(TimerStopped {
        work_log,
        task_key: Some(timer.task_key.clone()),
        elapsed_minutes: elapsed,
    })
}