use rusqlite::params;
use tauri::{AppHandle, Emitter, State};

use super::super::database::{now, AppState};
use super::super::error::AppError;
use super::super::models::{RelationshipDetail, RelationshipInput};

const REL_TYPES: [&str; 5] = ["blocks", "blocked_by", "relates_to", "duplicates", "depends_on"];

#[derive(Clone)]
struct RelRow {
    id: i64,
    rel_type: String,
    direction: String,
    task_id: i64,
    task_key: String,
    task_title: String,
    created_at: String,
}

fn rel_detail(row: &RelRow) -> RelationshipDetail {
    RelationshipDetail {
        id: row.id,
        rel_type: row.rel_type.clone(),
        direction: row.direction.clone(),
        task_id: row.task_id,
        task_key: row.task_key.clone(),
        task_title: row.task_title.clone(),
        created_at: row.created_at.clone(),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_relationship(
    app: AppHandle,
    state: State<'_, AppState>,
    input: RelationshipInput,
) -> Result<(), AppError> {
    if !REL_TYPES.contains(&input.rel_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Unknown relationship type: {}",
            input.rel_type
        )));
    }
    if input.source_task_id == input.target_task_id {
        return Err(AppError::Validation("A task cannot relate to itself.".into()));
    }

    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    for tid in [input.source_task_id, input.target_task_id] {
        let owned: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
            params![tid, uid],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
    }

    let duplicate: bool = db.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM relationships
            WHERE (source_task_id = ?1 AND target_task_id = ?2 AND type = ?3)
               OR (source_task_id = ?2 AND target_task_id = ?1 AND type = ?3)
        )",
        params![input.source_task_id, input.target_task_id, input.rel_type],
        |r| r.get(0),
    )?;
    if duplicate {
        return Err(AppError::Validation("That relationship already exists.".into()));
    }

    db.execute(
        "INSERT INTO relationships (source_task_id, target_task_id, type, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![input.source_task_id, input.target_task_id, input.rel_type, now()],
    )?;
    let _ = app.emit("relationship-added", input.source_task_id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_relationships(
    state: State<'_, AppState>,
    task_id: i64,
) -> Result<Vec<RelationshipDetail>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let owned: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
        params![task_id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Task {task_id} not found.")));
    }

    let mut rows: Vec<RelRow> = Vec::new();
    {
        let mut stmt = db.prepare(
            "SELECT r.id, r.type, 'outgoing', t.id, t.key, t.title, r.created_at
             FROM relationships r
             JOIN tasks t ON t.id = r.target_task_id
             WHERE r.source_task_id = ?1
             ORDER BY r.created_at ASC",
        )?;
        for row in stmt.query_map([task_id], |row| {
            Ok(RelRow {
                id: row.get(0)?,
                rel_type: row.get(1)?,
                direction: row.get(2)?,
                task_id: row.get(3)?,
                task_key: row.get(4)?,
                task_title: row.get(5)?,
                created_at: row.get(6)?,
            })
        })? {
            rows.push(row?);
        }
    }
    {
        let mut stmt = db.prepare(
            "SELECT r.id, r.type, 'incoming', t.id, t.key, t.title, r.created_at
             FROM relationships r
             JOIN tasks t ON t.id = r.source_task_id
             WHERE r.target_task_id = ?1
             ORDER BY r.created_at ASC",
        )?;
        for row in stmt.query_map([task_id], |row| {
            Ok(RelRow {
                id: row.get(0)?,
                rel_type: row.get(1)?,
                direction: row.get(2)?,
                task_id: row.get(3)?,
                task_key: row.get(4)?,
                task_title: row.get(5)?,
                created_at: row.get(6)?,
            })
        })? {
            rows.push(row?);
        }
    }

    Ok(rows.into_iter().map(|r| rel_detail(&r)).collect())
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_relationship(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let owned: bool = db.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM relationships r
            LEFT JOIN tasks ts ON ts.id = r.source_task_id
            LEFT JOIN tasks tt ON tt.id = r.target_task_id
            LEFT JOIN projects ps ON ps.id = ts.project_id
            LEFT JOIN projects pt ON pt.id = tt.project_id
            WHERE r.id = ?1 AND (COALESCE(ps.user_id, pt.user_id) = ?2)
        )",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Relationship {id} not found.")));
    }
    let deleted = db.execute("DELETE FROM relationships WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Relationship {id} not found.")));
    }
    let _ = app.emit("relationship-removed", id);
    Ok(())
}