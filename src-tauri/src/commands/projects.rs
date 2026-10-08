use rusqlite::{params, Connection, Row};

use super::super::database::{indexer, now, AppState};
use super::super::error::{ensure_key_valid, AppError};
use super::super::models::{Project, ProjectInput, ProjectNode, ProjectStats, ProjectUpdate};
use tauri::{AppHandle, Emitter, State};

fn project_from_row(row: &Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        key: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        status: row.get(5)?,
        priority: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        completed_at: row.get(9)?,
    })
}

fn map_error(err: rusqlite::Error) -> AppError {
    match &err {
        rusqlite::Error::SqliteFailure(e, Some(msg)) if e.code == rusqlite::ErrorCode::ConstraintViolation && msg.contains("projects.key") => {
            AppError::Validation("Project key already exists.".into())
        }
        _ => AppError::Db(err),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_project(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ProjectInput,
) -> Result<Project, AppError> {
    ensure_key_valid(&input.key)?;
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Name must not be empty.".into()));
    }

    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let ts = now();

    let parent = match input.parent_id {
        Some(id) => {
            let owned: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
                params![id, uid],
                |r| r.get(0),
            )?;
            if !owned {
                return Err(AppError::NotFound(format!("Parent project {id} not found.")));
            }
            Some(id)
        }
        None => None,
    };

    let key = input.key.trim().to_uppercase();
    let priority = input.priority.unwrap_or_else(|| "Medium".into());
    let status = input.status.unwrap_or_else(|| "Active".into());
    let description = input.description.unwrap_or_default();

    let result = tx.execute(
        "INSERT INTO projects (parent_id, key, name, description, status, priority, user_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![parent, key, input.name.trim(), description, status, priority, uid, ts],
    );

    if let Err(err) = result {
        return Err(map_error(err));
    }

    let id = tx.last_insert_rowid();
    indexer::upsert(
        &tx,
        "project",
        id,
        uid,
        &format!("{key} — {}", input.name.trim()),
        &description,
    )?;
    tx.commit()?;
    drop(db);

    app.emit("project-created", id)?;
    get_project(state, id)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_project(state: State<'_, AppState>, id: i64) -> Result<Project, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    db.query_row(
        "SELECT id, parent_id, key, name, description, status, priority, created_at, updated_at, completed_at
         FROM projects WHERE id = ?1 AND user_id = ?2",
        params![id, uid],
        project_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Project {id} not found.")),
        other => AppError::Db(other),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: ProjectUpdate,
) -> Result<Project, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    if !tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
        params![id, uid],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(AppError::NotFound(format!("Project {id} not found.")));
    }

    if let Some(pid) = input.parent_id {
        let mut candidates: Vec<i64> = vec![id];
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let mut stmt = tx.prepare("SELECT id FROM projects WHERE parent_id = ?1 AND user_id = ?2")?;
            for child in stmt.query_map(params![cur, uid], |r| r.get::<_, i64>(0))? {
                let child_id = child?;
                candidates.push(child_id);
                stack.push(child_id);
            }
        }
        if candidates.contains(&pid) {
            return Err(AppError::Validation("Cannot set a project as its own descendant.".into()));
        }
    }

    let ts = now();
    let mut sets: Vec<String> = vec!["updated_at = ?".to_string()];
    let mut values: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(ts.clone())];

    if let Some(name) = &input.name {
        if name.trim().is_empty() {
            return Err(AppError::Validation("Name must not be empty.".into()));
        }
        sets.push("name = ?".to_string());
        values.push(rusqlite::types::Value::Text(name.trim().to_string()));
    }
    if let Some(desc) = &input.description {
        sets.push("description = ?".to_string());
        values.push(rusqlite::types::Value::Text(desc.clone()));
    }
    if let Some(priority) = &input.priority {
        sets.push("priority = ?".to_string());
        values.push(rusqlite::types::Value::Text(priority.clone()));
    }
    if let Some(status) = &input.status {
        sets.push("status = ?".to_string());
        sets.push("completed_at = ?".to_string());
        let completed_at = if status == "Completed" || status == "Archived" {
            ts.clone()
        } else {
            "".into()
        };
        values.push(rusqlite::types::Value::Text(status.clone()));
        values.push(rusqlite::types::Value::Text(completed_at));
    }
    if let Some(pid) = input.parent_id {
        sets.push("parent_id = ?".to_string());
        values.push(rusqlite::types::Value::Integer(pid));
    }

    let sql = format!(
        "UPDATE projects SET {} WHERE id = ?{}",
        sets.join(", "),
        values.len() + 1
    );
    values.push(rusqlite::types::Value::Integer(id));

    let mut stmt = tx.prepare(&sql)?;
    stmt.execute(rusqlite::params_from_iter(values.iter()))?;
    drop(stmt);

    let project: Project = tx.query_row(
        "SELECT id, parent_id, key, name, description, status, priority, created_at, updated_at, completed_at
         FROM projects WHERE id = ?1",
        [id],
        project_from_row,
    )?;
    indexer::upsert(&tx, "project", id, uid, &format!("{} — {}", project.key, project.name), &project.description)?;
    tx.commit()?;
    drop(db);

    app.emit("project-updated", id)?;
    Ok(project)
}

fn collect_subtree_ids(conn: &Connection, root: i64) -> rusqlite::Result<Vec<i64>> {
    let mut all = vec![root];
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let mut stmt = conn.prepare("SELECT id FROM projects WHERE parent_id = ?1")?;
        let children: Vec<i64> = stmt
            .query_map([id], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for child in children {
            all.push(child);
            stack.push(child);
        }
    }
    Ok(all)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_project(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound(format!("Project {id} not found.")));
    }

    let subtree = collect_subtree_ids(&tx, id)?;
    let placeholders = vec!["?"; subtree.len()].join(",");
    {   
        let mut del_tasks = tx.prepare("DELETE FROM search_index WHERE category = 'task' AND item_id = ?1")?;
        let mut del_projects = tx.prepare("DELETE FROM search_index WHERE category = 'project' AND item_id = ?1")?;
        for pid in &subtree {
            let mut stmt = tx.prepare("SELECT id FROM tasks WHERE project_id = ?1")?;
            let task_ids: Vec<i64> = stmt
                .query_map([pid], |r| r.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            for tid in task_ids {
                let _ = del_tasks.execute([tid])?;
            }
            let _ = del_projects.execute([*pid])?;
        }
    }
    tx.execute(
        &format!(
            "DELETE FROM search_index WHERE category = 'attachment' AND item_id IN (
                 SELECT id FROM attachments WHERE project_id IN ({placeholders}) OR task_id IN (
                     SELECT id FROM tasks WHERE project_id IN ({placeholders})
                 )
             )"
        ),
        rusqlite::params_from_iter(subtree.iter().chain(subtree.iter())),
    )?;
    tx.execute(
        &format!(
            "DELETE FROM search_index WHERE category = 'document' AND item_id IN (
                 SELECT id FROM documents WHERE project_id IN ({placeholders})
             )"
        ),
        rusqlite::params_from_iter(subtree.iter()),
    )?;

    tx.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    tx.commit()?;
    drop(db);

    let _ = app.emit("project-deleted", id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(
        "SELECT id, parent_id, key, name, description, status, priority, created_at, updated_at, completed_at
         FROM projects WHERE user_id = ?1 ORDER BY key",
    )?;
    let rows = stmt.query_map([uid], project_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_project_tree(state: State<'_, AppState>) -> Result<Vec<ProjectNode>, AppError> {
    let projects = list_projects(state)?;

    let nodes: std::collections::HashMap<i64, ProjectNode> = projects
        .into_iter()
        .map(|p| {
            let id = p.id;
            (id, ProjectNode { project: p, children: Vec::new() })
        })
        .collect();

    let mut by_parent: std::collections::HashMap<Option<i64>, Vec<i64>> = std::collections::HashMap::new();
    for node in nodes.values() {
        by_parent.entry(node.project.parent_id).or_default().push(node.project.id);
    }
    for children in by_parent.values_mut() {
        children.sort_unstable();
    }

    fn build(parent: Option<i64>, by_parent: &std::collections::HashMap<Option<i64>, Vec<i64>>, nodes: &std::collections::HashMap<i64, ProjectNode>, depth: u32, out: &mut Vec<ProjectNode>) {
        if depth > 100 {
            return;
        }
        let mut children = Vec::new();
        if let Some(ids) = by_parent.get(&parent) {
            for &cid in ids {
                if let Some(node) = nodes.get(&cid) {
                    let mut child = node.clone();
                    build(Some(cid), by_parent, nodes, depth + 1, &mut child.children);
                    children.push(child);
                }
            }
        }
        children.sort_by(|a, b| a.project.key.cmp(&b.project.key));
        out.extend(children);
    }

    let mut roots: Vec<ProjectNode> = Vec::new();
    build(None, &by_parent, &nodes, 0, &mut roots);
    if roots.is_empty() && !nodes.is_empty() {
        let mut leftover: Vec<ProjectNode> = nodes.into_values().collect();
        leftover.sort_by(|a, b| a.project.key.cmp(&b.project.key));
        return Ok(leftover);
    }
    Ok(roots)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_project_stats(state: State<'_, AppState>, id: i64) -> Result<ProjectStats, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stats = ProjectStats::default();

    let owned: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Project {id} not found.")));
    }

    let one = |sql: &str| -> Result<i64, AppError> {
        db.query_row(sql, params![id], |r| r.get(0)).map_err(AppError::from)
    };

    stats.total = one("SELECT COUNT(*) FROM tasks WHERE project_id = ?1")?;
    stats.completed = one("SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'Completed'")?;
    stats.in_progress = one("SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'In Progress'")?;
    stats.blocked = one("SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status = 'Blocked'")?;
    stats.open = one("SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND status IN ('Todo', 'In Progress', 'Blocked')")?;
    stats.progress = if stats.total > 0 {
        (stats.completed as f64 / stats.total as f64) * 100.0
    } else {
        0.0
    };

    Ok(stats)
}