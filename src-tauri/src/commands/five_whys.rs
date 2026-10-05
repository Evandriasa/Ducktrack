use rusqlite::{params, Row};
use tauri::{AppHandle, Emitter, State};

use super::super::database::{now, AppState};
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{FiveWhy, FiveWhyInput, FiveWhyUpdate};

const VALID_STATUSES: &[&str] = &["Open", "Investigating", "Actioned", "Closed"];

const FIVEWHY_COLS: &str = "fw.id, fw.user_id, fw.project_id, fw.task_id, \
     p.key, p.name, t.key, t.title, \
     fw.problem, fw.why_1, fw.why_2, fw.why_3, fw.why_4, fw.why_5, \
     fw.root_cause, fw.corrective_action, fw.owner, fw.due_date, fw.status, \
     fw.created_at, fw.updated_at";

fn fivewhy_from_row(row: &Row) -> rusqlite::Result<FiveWhy> {
    Ok(FiveWhy {
        id: row.get(0)?,
        user_id: row.get(1)?,
        project_id: row.get(2)?,
        task_id: row.get(3)?,
        project_key: row.get(4)?,
        project_name: row.get(5)?,
        task_key: row.get(6)?,
        task_title: row.get(7)?,
        problem: row.get(8)?,
        why_1: row.get(9)?,
        why_2: row.get(10)?,
        why_3: row.get(11)?,
        why_4: row.get(12)?,
        why_5: row.get(13)?,
        root_cause: row.get(14)?,
        corrective_action: row.get(15)?,
        owner: row.get(16)?,
        due_date: row.get(17)?,
        status: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
    })
}

fn validate_status(status: &str) -> Result<(), AppError> {
    if !VALID_STATUSES.contains(&status) {
        return Err(AppError::Validation(format!("Unknown status: {status}")));
    }
    Ok(())
}

fn sanitize<'a>(input: &'a Option<String>, field: &str) -> Result<Option<String>, AppError> {
    match input {
        Some(v) => {
            let v = v.trim();
            if v.is_empty() {
                if field == "problem" {
                    return Err(AppError::Validation("Problem must not be empty.".into()));
                }
                Ok(None)
            } else {
                Ok(Some(v.to_string()))
            }
        }
        None => Ok(None),
    }
}

fn project_owned(db: &rusqlite::Connection, project_id: i64, uid: i64) -> bool {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
        params![project_id, uid],
        |r| r.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

fn task_owned(db: &rusqlite::Connection, task_id: i64, uid: i64) -> bool {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
        params![task_id, uid],
        |r| r.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_five_whys(state: State<'_, AppState>) -> Result<Vec<FiveWhy>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(&format!(
        "SELECT {FIVEWHY_COLS}
         FROM five_whys fw
         LEFT JOIN projects p ON p.id = fw.project_id
         LEFT JOIN tasks t ON t.id = fw.task_id
         WHERE fw.user_id = ?1
         ORDER BY fw.updated_at DESC"
    ))?;
    let rows = stmt.query_map([uid], fivewhy_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_five_why(state: State<'_, AppState>, id: i64) -> Result<FiveWhy, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    db.query_row(
        &format!(
            "SELECT {FIVEWHY_COLS}
             FROM five_whys fw
             LEFT JOIN projects p ON p.id = fw.project_id
             LEFT JOIN tasks t ON t.id = fw.task_id
             WHERE fw.id = ?1 AND fw.user_id = ?2"
        ),
        params![id, uid],
        fivewhy_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Analysis {id} not found.")),
        other => AppError::Db(other),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_five_why(
    app: AppHandle,
    state: State<'_, AppState>,
    input: FiveWhyInput,
) -> Result<FiveWhy, AppError> {
    ensure_non_empty(&input.problem, "Problem")?;
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    if let Some(pid) = input.project_id {
        if !project_owned(&db, pid, uid) {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
    }
    if let Some(tid) = input.task_id {
        if !task_owned(&db, tid, uid) {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
    }

    let status = input.status.unwrap_or_else(|| "Open".into());
    validate_status(&status)?;

    let problem = input.problem.trim().to_string();
    let ts = now();

    db.execute(
        "INSERT INTO five_whys
         (user_id, project_id, task_id, problem, why_1, why_2, why_3, why_4, why_5,
          root_cause, corrective_action, owner, due_date, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            uid,
            input.project_id,
            input.task_id,
            problem,
            sanitize(&input.why_1, "why_1")?,
            sanitize(&input.why_2, "why_2")?,
            sanitize(&input.why_3, "why_3")?,
            sanitize(&input.why_4, "why_4")?,
            sanitize(&input.why_5, "why_5")?,
            sanitize(&input.root_cause, "root_cause")?,
            sanitize(&input.corrective_action, "corrective_action")?,
            sanitize(&input.owner, "owner")?,
            sanitize(&input.due_date, "due_date")?,
            status,
            ts,
        ],
    )?;
    let id = db.last_insert_rowid();
    drop(db);
    let five_why = get_five_why(state, id)?;
    let _ = app.emit("five-why-created", serde_json::to_value(&five_why).unwrap_or_default());
    Ok(five_why)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_five_why(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: FiveWhyUpdate,
) -> Result<FiveWhy, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM five_whys WHERE id = ?1 AND user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Analysis {id} not found.")));
    }

    if let Some(pid) = input.project_id {
        if !project_owned(&tx, pid, uid) {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
    }
    if let Some(tid) = input.task_id {
        if !task_owned(&tx, tid, uid) {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
    }

    let status = match &input.status {
        Some(s) => {
            validate_status(s)?;
            let s = s.trim().to_string();
            Some(s)
        }
        None => None,
    };

    let current: (String, Option<i64>, Option<i64>, String) = tx.query_row(
        "SELECT status, project_id, task_id, problem FROM five_whys WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;

    let new_status = status.as_deref().unwrap_or(&current.0).to_string();
    let problem = match sanitize(&input.problem, "problem")? {
        Some(p) => p,
        None => current.3,
    };
    tx.execute(
        "UPDATE five_whys SET
           problem = ?1, project_id = ?2, task_id = ?3,
           why_1 = ?4, why_2 = ?5, why_3 = ?6, why_4 = ?7, why_5 = ?8,
           root_cause = ?9, corrective_action = ?10, owner = ?11, due_date = ?12,
           status = ?13, updated_at = ?14
         WHERE id = ?15",
        params![
            problem,
            input.project_id.or(current.1),
            input.task_id.or(current.2),
            sanitize(&input.why_1, "why_1")?,
            sanitize(&input.why_2, "why_2")?,
            sanitize(&input.why_3, "why_3")?,
            sanitize(&input.why_4, "why_4")?,
            sanitize(&input.why_5, "why_5")?,
            sanitize(&input.root_cause, "root_cause")?,
            sanitize(&input.corrective_action, "corrective_action")?,
            sanitize(&input.owner, "owner")?,
            sanitize(&input.due_date, "due_date")?,
            new_status,
            now(),
            id,
        ],
    )?;
    tx.commit()?;
    drop(db);
    let five_why = get_five_why(state, id)?;
    let _ = app.emit("five-why-updated", serde_json::to_value(&five_why).unwrap_or_default());
    Ok(five_why)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_five_why(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let deleted = db.execute(
        "DELETE FROM five_whys WHERE id = ?1 AND user_id = ?2",
        params![id, uid],
    )?;
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Analysis {id} not found.")));
    }
    let _ = app.emit("five-why-deleted", id);
    Ok(())
}