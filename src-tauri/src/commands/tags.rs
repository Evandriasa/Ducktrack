use rusqlite::{params, Row};
use tauri::{AppHandle, Emitter, State};

use super::super::database::AppState;
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{Tag, TagInput, TagUpdate, TagWithCount};

fn tag_from_row(row: &Row) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
    })
}

fn tag_with_count_from_row(row: &Row) -> rusqlite::Result<TagWithCount> {
    Ok(TagWithCount {
        id: row.get(0)?,
        name: row.get(1)?,
        task_count: row.get(2)?,
        project_count: row.get(3)?,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<TagWithCount>, AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(
        "SELECT t.id, t.name,
                (SELECT COUNT(*) FROM task_tags tt WHERE tt.tag_id = t.id),
                (SELECT COUNT(*) FROM project_tags pt WHERE pt.tag_id = t.id)
         FROM tags t ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], tag_with_count_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_tag(state: State<'_, AppState>, input: TagInput) -> Result<Tag, AppError> {
    ensure_non_empty(&input.name, "Name")?;
    let name = input.name.trim();
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1 COLLATE NOCASE)",
        [name],
        |r| r.get(0),
    )?;
    if exists {
        return Err(AppError::Validation("A tag with that name already exists.".into()));
    }
    db.execute("INSERT INTO tags (name) VALUES (?1)", [name])?;
    let id = db.last_insert_rowid();
    db.query_row("SELECT id, name FROM tags WHERE id = ?1", [id], tag_from_row)
        .map_err(AppError::from)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_tag(state: State<'_, AppState>, id: i64, input: TagUpdate) -> Result<Tag, AppError> {
    ensure_non_empty(input.name.as_deref().unwrap_or(""), "Name")?;
    let name = input.name.as_deref().unwrap_or("").trim().to_string();
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let taken: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM tags WHERE name = ?1 COLLATE NOCASE AND id <> ?2)",
        params![name, id],
        |r| r.get(0),
    )?;
    if taken {
        return Err(AppError::Validation("A tag with that name already exists.".into()));
    }
    let updated = db.execute("UPDATE tags SET name = ?1 WHERE id = ?2", params![name, id])?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("Tag {id} not found.")));
    }
    db.query_row("SELECT id, name FROM tags WHERE id = ?1", [id], tag_from_row)
        .map_err(AppError::from)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let deleted = db.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Tag {id} not found.")));
    }
    let _ = app.emit("tag-deleted", id);
    Ok(())
}

fn task_owned(db: &rusqlite::Connection, task_id: i64, uid: i64) -> bool {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
        params![task_id, uid],
        |r| r.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

fn tag_exists(db: &rusqlite::Connection, tag_id: i64) -> bool {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
        [tag_id],
        |r| r.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_task_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: i64,
    tag_id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    if !task_owned(&db, task_id, uid) {
        return Err(AppError::NotFound(format!("Task {task_id} not found.")));
    }
    if !tag_exists(&db, tag_id) {
        return Err(AppError::NotFound(format!("Tag {tag_id} not found.")));
    }
    db.execute(
        "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
        params![task_id, tag_id],
    )?;
    let _ = app.emit("task-tag-added", (task_id, tag_id));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_task_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: i64,
    tag_id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    if !task_owned(&db, task_id, uid) {
        return Err(AppError::NotFound(format!("Task {task_id} not found.")));
    }
    db.execute(
        "DELETE FROM task_tags WHERE task_id = ?1 AND tag_id = ?2",
        params![task_id, tag_id],
    )?;
    let _ = app.emit("task-tag-removed", (task_id, tag_id));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_task_tags(state: State<'_, AppState>, task_id: i64) -> Result<Vec<Tag>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    if !task_owned(&db, task_id, uid) {
        return Err(AppError::NotFound(format!("Task {task_id} not found.")));
    }
    let mut stmt = db.prepare(
        "SELECT t.id, t.name FROM tags t JOIN task_tags tt ON tt.tag_id = t.id WHERE tt.task_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([task_id], tag_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_project_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: i64,
    tag_id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let owned: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
        params![project_id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Project {project_id} not found.")));
    }
    if !tag_exists(&db, tag_id) {
        return Err(AppError::NotFound(format!("Tag {tag_id} not found.")));
    }
    db.execute(
        "INSERT OR IGNORE INTO project_tags (project_id, tag_id) VALUES (?1, ?2)",
        params![project_id, tag_id],
    )?;
    let _ = app.emit("project-tag-added", (project_id, tag_id));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_project_tag(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: i64,
    tag_id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let owned: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
        params![project_id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Project {project_id} not found.")));
    }
    db.execute(
        "DELETE FROM project_tags WHERE project_id = ?1 AND tag_id = ?2",
        params![project_id, tag_id],
    )?;
    let _ = app.emit("project-tag-removed", (project_id, tag_id));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_project_tags(state: State<'_, AppState>, project_id: i64) -> Result<Vec<Tag>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let owned: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
        params![project_id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Project {project_id} not found.")));
    }
    let mut stmt = db.prepare(
        "SELECT t.id, t.name FROM tags t JOIN project_tags pt ON pt.tag_id = t.id WHERE pt.project_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([project_id], tag_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}