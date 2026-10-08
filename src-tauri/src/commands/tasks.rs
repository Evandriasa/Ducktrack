use rusqlite::{params, Connection, Row};
use tauri::{AppHandle, Emitter, State};

use super::super::database::{indexer, now, AppState};
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{Task, TaskFilter, TaskInput, TaskUpdate, TaskWithProject};

const VALID_STATUSES: &[&str] = &["Todo", "In Progress", "Blocked", "Completed", "Cancelled"];
const VALID_PRIORITIES: &[&str] = &["Low", "Medium", "High", "Critical"];
const VALID_TYPES: &[&str] = &[
    "Task",
    "Bug",
    "Feature",
    "Improvement",
    "Maintenance",
    "Investigation",
    "Documentation",
];

pub fn task_from_row(row: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        parent_task_id: row.get(2)?,
        key: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        status: row.get(6)?,
        priority: row.get(7)?,
        task_type: row.get(8)?,
        due_date: row.get(9)?,
        assignee: row.get(10)?,
        estimated_minutes: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        completed_at: row.get(14)?,
    })
}

pub fn task_with_project_from_row(row: &Row) -> rusqlite::Result<TaskWithProject> {
    let task = Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        parent_task_id: row.get(2)?,
        key: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        status: row.get(6)?,
        priority: row.get(7)?,
        task_type: row.get(8)?,
        due_date: row.get(9)?,
        assignee: row.get(10)?,
        estimated_minutes: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        completed_at: row.get(14)?,
    };
    Ok(TaskWithProject {
        task,
        project_key: row.get(15)?,
        project_name: row.get(16)?,
    })
}

const TASK_COLS: &str = "t.id, t.project_id, t.parent_task_id, t.key, t.title, t.description, \
     t.status, t.priority, t.type, t.due_date, t.assignee, t.estimated_minutes, t.created_at, t.updated_at, t.completed_at";

fn write_history(
    tx: &Connection,
    task_id: i64,
    action: &str,
    field: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    user_id: i64,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO task_history (task_id, action, field, old_value, new_value, created_at, user_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![task_id, action, field, old_value, new_value, now(), user_id],
    )?;
    Ok(())
}

fn next_task_key(tx: &Connection, project_id: i64) -> Result<String, AppError> {
    let project_key: String = tx.query_row(
        "SELECT key FROM projects WHERE id = ?1",
        [project_id],
        |r| r.get(0),
    )?;
    let prefix = format!("{project_key}-");
    let max: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(CAST(SUBSTR(key, ?1) AS INTEGER)), 0) FROM tasks WHERE key LIKE ?2",
            params![prefix.len() as i64 + 1, format!("{}%", prefix)],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(format!("{prefix}{:03}", max + 1))
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_task(
    app: AppHandle,
    state: State<'_, AppState>,
    input: TaskInput,
) -> Result<Task, AppError> {
    ensure_non_empty(&input.title, "Title")?;

    let uid = state.active_user()?;
    let project_owned: bool = state
        .db
        .lock()
        .map_err(|_| AppError::Other("db lock poisoned".into()))?
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![input.project_id, uid],
            |r| r.get(0),
        )?;
    if !project_owned {
        return Err(AppError::NotFound(format!(
            "Project {} not found.",
            input.project_id
        )));
    }

    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let ts = now();

    if let Some(parent) = input.parent_task_id {
        let parent_ok: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
            params![parent, uid],
            |r| r.get(0),
        )?;
        if !parent_ok {
            return Err(AppError::NotFound(format!("Parent task {parent} not found.")));
        }
    }

    let key = next_task_key(&tx, input.project_id)?;
    let status = input.status.unwrap_or_else(|| "Todo".into());
    let priority = input.priority.unwrap_or_else(|| "Medium".into());
    let task_type = input.task_type.unwrap_or_else(|| "Task".into());

    if !VALID_STATUSES.contains(&status.as_str()) {
        return Err(AppError::Validation(format!("Unknown status: {status}")));
    }
    if !VALID_PRIORITIES.contains(&priority.as_str()) {
        return Err(AppError::Validation(format!("Unknown priority: {priority}")));
    }
    if !VALID_TYPES.contains(&task_type.as_str()) {
        return Err(AppError::Validation(format!("Unknown task type: {task_type}")));
    }

    let description = input.description.unwrap_or_default();
    let completed_at = if status == "Completed" { Some(ts.clone()) } else { None };

    tx.execute(
        "INSERT INTO tasks (project_id, parent_task_id, key, title, description, status, priority, type, due_date, assignee, estimated_minutes, created_at, updated_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, ?13)",
        params![
            input.project_id,
            input.parent_task_id,
            key,
            input.title.trim(),
            description,
            status,
            priority,
            task_type,
            input.due_date,
            input.assignee,
            input.estimated_minutes,
            ts,
            completed_at,
        ],
    )?;

    let id = tx.last_insert_rowid();
    write_history(&tx, id, "created", Some("status"), None, Some(&status), uid)?;
    if status == "Completed" {
        write_history(&tx, id, "completed", Some("status"), Some("Todo"), Some("Completed"), uid)?;
    }

    indexer::upsert(
        &tx,
        "task",
        id,
        uid,
        &format!("{key} — {}", input.title.trim()),
        &description,
    )?;
    tx.commit()?;
    drop(db);

    let task = get_task(state, id)?;
    let _ = app.emit("task-created", serde_json::to_value(&task).unwrap_or_default());
    Ok(task)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_task(state: State<'_, AppState>, id: i64) -> Result<Task, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    db.query_row(
        &format!(
            "SELECT {TASK_COLS} FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2"
        ),
        params![id, uid],
        task_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Task {id} not found.")),
        other => AppError::Db(other),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn complete_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<Task, AppError> {
    update_task(app, state, id, TaskUpdate {
        status: Some("Completed".into()),
        ..Default::default()
    })
}

fn update_in_tx(
    tx: &Connection,
    task_id: i64,
    input: &TaskUpdate,
    user_id: i64,
) -> Result<Task, AppError> {
    let existing: Task = tx.query_row(
        &format!("SELECT {TASK_COLS} FROM tasks t WHERE t.id = ?1"),
        [task_id],
        task_from_row,
    )?;

    let mut sets: Vec<&str> = vec!["updated_at = ?"];
    let mut values: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(now())];
    let completed_ts = now();

    macro_rules! field {
        ($new:expr, $col:ident, $name:expr) => {
            if let Some(value) = &$new {
                if &existing.$col != value {
                    write_history(
                        &tx,
                        task_id,
                        "changed",
                        Some($name),
                        Some(existing.$col.as_str()),
                        Some(value.as_str()),
                        user_id,
                    )?;
                }
                sets.push(concat!(stringify!($col), " = ?"));
                values.push(rusqlite::types::Value::Text(value.clone()));
            }
        };
    }

    field!(input.title, title, "title");
    field!(input.description, description, "description");
    field!(input.priority, priority, "priority");
    // task_type → DB column "type" (stringify!($col) = "task_type", so push the correct SQL name)
    if let Some(value) = &input.task_type {
        if existing.task_type != *value {
            write_history(
                &tx,
                task_id,
                "changed",
                Some("type"),
                Some(existing.task_type.as_str()),
                Some(value.as_str()),
                user_id,
            )?;
        }
        sets.push("type = ?");
        values.push(rusqlite::types::Value::Text(value.clone()));
    }
    // due_date: an empty string clears the field (stored as NULL)
    if let Some(value) = &input.due_date {
        let old = existing.due_date.clone().unwrap_or_default();
        if old != *value {
            write_history(
                &tx,
                task_id,
                "changed",
                Some("due_date"),
                Some(&old),
                Some(value),
                user_id,
            )?;
        }
        if value.is_empty() {
            sets.push("due_date = NULL");
        } else {
            sets.push("due_date = ?");
            values.push(rusqlite::types::Value::Text(value.clone()));
        }
    }

    // assignee: an empty string clears the field (stored as NULL)
    if let Some(value) = &input.assignee {
        let old = existing.assignee.clone().unwrap_or_default();
        if old != *value {
            write_history(&tx, task_id, "changed", Some("assignee"), Some(&old), Some(value), user_id)?;
        }
        if value.is_empty() {
            sets.push("assignee = NULL");
        } else {
            sets.push("assignee = ?");
            values.push(rusqlite::types::Value::Text(value.clone()));
        }
    }

    if let Some(new_status) = &input.status {
        if !VALID_STATUSES.contains(&new_status.as_str()) {
            return Err(AppError::Validation(format!("Unknown status: {new_status}")));
        }
        if existing.status != *new_status {
            match new_status.as_str() {
                "Completed" => {
                    write_history(&tx, task_id, "completed", Some("status"), Some(existing.status.as_str()), Some(new_status.as_str()), user_id)?;
                }
                "Cancelled" => {
                    write_history(&tx, task_id, "cancelled", Some("status"), Some(existing.status.as_str()), Some(new_status.as_str()), user_id)?;
                }
                _ if existing.status == "Completed" || existing.status == "Cancelled" => {
                    write_history(&tx, task_id, "reopened", Some("status"), Some(existing.status.as_str()), Some(new_status.as_str()), user_id)?;
                }
                _ => {
                    write_history(&tx, task_id, "changed", Some("status"), Some(existing.status.as_str()), Some(new_status.as_str()), user_id)?;
                }
            }
        }
        sets.push("status = ?");
        values.push(rusqlite::types::Value::Text(new_status.clone()));
        if new_status == "Completed" {
            sets.push("completed_at = ?");
            values.push(rusqlite::types::Value::Text(completed_ts.clone()));
        } else {
            sets.push("completed_at = NULL");
        }
    }

    if let Some(value) = &input.parent_task_id {
        if *value == task_id {
            return Err(AppError::Validation("A task cannot be its own parent.".into()));
        }
        let parent_owned: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
            params![value, user_id],
            |r| r.get(0),
        )?;
        if !parent_owned {
            return Err(AppError::Validation(format!("Parent task {value} not found.")));
        }
        let mut cursor = *value;
        let mut guard = 0;
        loop {
            let up: Option<i64> = tx.query_row(
                "SELECT parent_task_id FROM tasks WHERE id = ?1 AND parent_task_id IS NOT NULL",
                [cursor],
                |r| r.get(0),
            ).ok();
            match up {
                None => break,
                Some(ancestor) if ancestor == task_id => {
                    return Err(AppError::Validation("Cannot set a task as an ancestor of itself.".into()));
                }
                Some(ancestor) => {
                    guard += 1;
                    if guard > 1000 {
                        return Err(AppError::Validation("Parent chain too deep.".into()));
                    }
                    cursor = ancestor;
                }
            }
        }
        let old = existing.parent_task_id;
        if old != Some(*value) {
            write_history(
                &tx,
                task_id,
                "changed",
                Some("parent_task"),
                Some(&old.unwrap_or(-1).to_string()),
                Some(&value.to_string()),
                user_id,
            )?;
        }
        sets.push("parent_task_id = ?");
        values.push(rusqlite::types::Value::Integer(*value));
    }

    if let Some(value) = input.estimated_minutes {
        if existing.estimated_minutes != Some(value) {
            let old = existing.estimated_minutes.map(|v| v.to_string()).unwrap_or_default();
            write_history(
                &tx,
                task_id,
                "changed",
                Some("estimate"),
                Some(&old),
                Some(&value.to_string()),
                user_id,
            )?;
        }
        // 0 (empty estimate field) clears the value back to NULL
        if value == 0 {
            sets.push("estimated_minutes = NULL");
        } else {
            sets.push("estimated_minutes = ?");
            values.push(rusqlite::types::Value::Integer(value));
        }
    }

    if let Some(new_pid) = input.project_id {
        if new_pid != existing.project_id {
            let owned: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
                params![new_pid, user_id],
                |r| r.get(0),
            )?;
            if !owned {
                return Err(AppError::NotFound(format!("Project {new_pid} not found.")));
            }
            let old_key = existing.key.clone();
            let new_key = next_task_key(tx, new_pid)?;
            write_history(
                &tx,
                task_id,
                "changed",
                Some("project"),
                Some(&existing.project_id.to_string()),
                Some(&new_pid.to_string()),
                user_id,
            )?;
            write_history(
                &tx,
                task_id,
                "changed",
                Some("key"),
                Some(&old_key),
                Some(&new_key),
                user_id,
            )?;
            sets.push("key = ?");
            values.push(rusqlite::types::Value::Text(new_key));
            sets.push("project_id = ?");
            values.push(rusqlite::types::Value::Integer(new_pid));
        }
    }

    let sql = format!(
        "UPDATE tasks SET {} WHERE id = ?{}",
        sets.join(", "),
        values.len() + 1
    );
    values.push(rusqlite::types::Value::Integer(task_id));
    tx.prepare(&sql)?.execute(rusqlite::params_from_iter(values.iter()))?;

    let updated: Task = tx.query_row(
        &format!("SELECT {TASK_COLS} FROM tasks t WHERE t.id = ?1"),
        [task_id],
        task_from_row,
    )?;

    indexer::upsert(&tx, "task", task_id, user_id, &format!("{} — {}", updated.key, updated.title), &updated.description)?;

    Ok(updated)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: TaskUpdate,
) -> Result<Task, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Task {id} not found.")));
    }

    let updated = update_in_tx(&tx, id, &input, uid)?;
    tx.commit()?;
    drop(db);

    let _ = app.emit("task-updated", serde_json::to_value(&updated).unwrap_or_default());
    Ok(updated)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_task(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let owned: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.id = ?1 AND p.user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Task {id} not found.")));
    }
    indexer::remove(&tx, "task", id)?;

    let mut all = vec![id];
    let mut cursor = vec![id];
    let mut seen = std::collections::HashSet::new();
    seen.insert(id);
    while let Some(tid) = cursor.pop() {
        let children: Vec<i64> = tx
            .prepare(
                "SELECT t.id FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.parent_task_id = ?1 AND p.user_id = ?2",
            )?
            .query_map(params![tid, uid], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for child in children {
            if seen.insert(child) {
                all.push(child);
                cursor.push(child);
            }
        }
    }
    for tid in &all {
        indexer::remove(&tx, "task", *tid)?;
    }
    let placeholders = vec!["?"; all.len()].join(",");
    tx.execute(
        &format!(
            "DELETE FROM search_index WHERE category = 'attachment' AND item_id IN (
                 SELECT id FROM attachments WHERE task_id IN ({placeholders})
             )"
        ),
        rusqlite::params_from_iter(all.iter()),
    )?;

    let deleted = tx.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
    tx.commit()?;
    drop(db);
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Task {id} not found.")));
    }
    let _ = app.emit("task-deleted", id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_tasks(
    state: State<'_, AppState>,
    filter: Option<TaskFilter>,
) -> Result<Vec<TaskWithProject>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let filter = filter.unwrap_or_default();

    let mut sql = format!(
        "SELECT {TASK_COLS}, p.key, p.name FROM tasks t
         JOIN projects p ON p.id = t.project_id WHERE p.user_id = ?"
    );
    let mut values: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Integer(uid)];

    if let Some(tag_id) = filter.tag_id {
        sql.push_str(" AND t.id IN (SELECT task_id FROM task_tags WHERE tag_id = ?)");
        values.push(rusqlite::types::Value::Integer(tag_id));
    }
    if let Some(pid) = filter.project_id {
        sql.push_str(" AND t.project_id = ?");
        values.push(rusqlite::types::Value::Integer(pid));
    }
    if let Some(status) = &filter.status {
        sql.push_str(" AND t.status = ?");
        values.push(rusqlite::types::Value::Text(status.clone()));
    }
    if let Some(priority) = &filter.priority {
        sql.push_str(" AND t.priority = ?");
        values.push(rusqlite::types::Value::Text(priority.clone()));
    }
    if let Some(assignee) = &filter.assignee {
        sql.push_str(" AND t.assignee = ?");
        values.push(rusqlite::types::Value::Text(assignee.clone()));
    }
    if !filter.include_subtasks.unwrap_or(false) {
        sql.push_str(" AND t.parent_task_id IS NULL");
    }

    sql.push_str(" ORDER BY t.created_at DESC");

    let mut stmt = db.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(values.iter()), task_with_project_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_subtasks(
    state: State<'_, AppState>,
    parent_id: i64,
) -> Result<Vec<TaskWithProject>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(
        &format!(
            "SELECT {TASK_COLS}, p.key, p.name FROM tasks t
             JOIN projects p ON p.id = t.project_id
             WHERE t.parent_task_id = ?1 AND p.user_id = ?2 ORDER BY t.created_at"
        ),
    )?;
    let rows = stmt.query_map(params![parent_id, uid], task_with_project_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_task_by_key(
    state: State<'_, AppState>,
    key: String,
) -> Result<TaskWithProject, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    db.query_row(
        &format!(
            "SELECT {TASK_COLS}, p.key, p.name FROM tasks t
             JOIN projects p ON p.id = t.project_id
             WHERE t.key = ?1 AND p.user_id = ?2"
        ),
        params![key.trim().to_uppercase(), uid],
        task_with_project_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Task {} not found.", key))
        }
        other => AppError::Db(other),
    })
}