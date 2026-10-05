use rusqlite::{params, Row};
use tauri::{AppHandle, Emitter, State};

use super::super::database::{now, AppState};
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{Comment, CommentInput};

fn comment_from_row(row: &Row) -> rusqlite::Result<Comment> {
    Ok(Comment {
        id: row.get(0)?,
        task_id: row.get(1)?,
        user_id: row.get(2)?,
        user_name: row.get(3)?,
        content: row.get(4)?,
        created_at: row.get(5)?,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_comment(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CommentInput,
) -> Result<Comment, AppError> {
    ensure_non_empty(&input.content, "Comment")?;
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let owned: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
        params![input.task_id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Task {} not found.", input.task_id)));
    }
    let ts = now();
    db.execute(
        "INSERT INTO comments (task_id, user_id, content, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![input.task_id, uid, input.content.trim(), ts],
    )?;
    let id = db.last_insert_rowid();
    let comment = db.query_row(
        "SELECT c.id, c.task_id, c.user_id, u.name AS user_name, c.content, c.created_at
         FROM comments c LEFT JOIN users u ON u.id = c.user_id WHERE c.id = ?1",
        [id],
        comment_from_row,
    )?;
    let _ = app.emit("comment-added", id);
    Ok(comment)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_comments(state: State<'_, AppState>, task_id: i64) -> Result<Vec<Comment>, AppError> {
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
    let mut stmt = db.prepare(
        "SELECT c.id, c.task_id, c.user_id, u.name AS user_name, c.content, c.created_at
         FROM comments c LEFT JOIN users u ON u.id = c.user_id
         WHERE c.task_id = ?1 ORDER BY c.created_at ASC, c.id ASC",
    )?;
    let rows = stmt.query_map([task_id], comment_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_comment(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let owned: bool = db.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM comments c
            JOIN tasks t ON t.id = c.task_id
            JOIN projects p ON p.id = t.project_id
            WHERE c.id = ?1 AND p.user_id = ?2
        )",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Comment {id} not found.")));
    }
    let deleted = db.execute("DELETE FROM comments WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Comment {id} not found.")));
    }
    let _ = app.emit("comment-deleted", id);
    Ok(())
}