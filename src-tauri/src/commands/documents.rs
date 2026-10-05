use rusqlite::{params, Row};
use tauri::{AppHandle, Emitter, State};

use super::super::database::{indexer, now, AppState};
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{Document, DocumentInput, DocumentUpdate, DocumentWithProject};

fn document_from_row(row: &Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn document_with_project_from_row(row: &Row) -> rusqlite::Result<DocumentWithProject> {
    let doc = Document {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    };
    Ok(DocumentWithProject {
        document: doc,
        project_key: row.get(6)?,
        project_name: row.get(7)?,
    })
}

const DOC_SELECT: &str = "SELECT d.id, d.project_id, d.title, d.content, d.created_at, d.updated_at,
        p.key, p.name
     FROM documents d JOIN projects p ON p.id = d.project_id";

#[tauri::command(rename_all = "snake_case")]
pub fn create_document(
    app: AppHandle,
    state: State<'_, AppState>,
    input: DocumentInput,
) -> Result<Document, AppError> {
    ensure_non_empty(&input.title, "Title")?;

    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let ts = now();

    let project: Option<(String, String)> = tx.query_row(
        "SELECT key, name FROM projects WHERE id = ?1 AND user_id = ?2",
        params![input.project_id, uid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();
    let (project_key, _project_name) = match project {
        Some(v) => v,
        None => return Err(AppError::NotFound(format!("Project {} not found.", input.project_id))),
    };

    let content = input.content.unwrap_or_default();
    tx.execute(
        "INSERT INTO documents (project_id, title, content, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![input.project_id, input.title.trim(), content, ts],
    )?;
    let id = tx.last_insert_rowid();
    indexer::upsert(&tx, "document", id, uid, &format!("{} — {}", project_key, input.title.trim()), &content)?;
    tx.commit()?;

    let doc = db.query_row(
        "SELECT id, project_id, title, content, created_at, updated_at FROM documents WHERE id = ?1",
        [id],
        document_from_row,
    )?;
    drop(db);
    let _ = app.emit("document-created", id);
    Ok(doc)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_document(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    input: DocumentUpdate,
) -> Result<Document, AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    let owned: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM documents d JOIN projects p ON p.id = d.project_id WHERE d.id = ?1 AND p.user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Document {id} not found.")));
    }

    let mut sets: Vec<&str> = vec!["updated_at = ?"];
    let mut values: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(now())];

    if let Some(title) = &input.title {
        if title.trim().is_empty() {
            return Err(AppError::Validation("Title must not be empty.".into()));
        }
        sets.push("title = ?");
        values.push(rusqlite::types::Value::Text(title.trim().to_string()));
    }
    if let Some(content) = &input.content {
        sets.push("content = ?");
        values.push(rusqlite::types::Value::Text(content.clone()));
    }

    let sql = format!("UPDATE documents SET {} WHERE id = ?{}", sets.join(", "), values.len() + 1);
    values.push(rusqlite::types::Value::Integer(id));
    tx.prepare(&sql)?.execute(rusqlite::params_from_iter(values.iter()))?;

    let doc: DocumentWithProject = tx.query_row(
        &format!("{DOC_SELECT} WHERE d.id = ?1"),
        [id],
        document_with_project_from_row,
    )?;
    indexer::upsert(&tx, "document", id, uid, &format!("{} — {}", doc.project_key, doc.document.title), &doc.document.content)?;
    tx.commit()?;
    drop(db);
    let _ = app.emit("document-updated", id);
    Ok(doc.document)
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_document(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let owned: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM documents d JOIN projects p ON p.id = d.project_id WHERE d.id = ?1 AND p.user_id = ?2)",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Document {id} not found.")));
    }
    let _ = indexer::remove(&tx, "document", id);
    let deleted = tx.execute("DELETE FROM documents WHERE id = ?1", [id])?;
    tx.commit()?;
    drop(db);
    if deleted == 0 {
        return Err(AppError::NotFound(format!("Document {id} not found.")));
    }
    let _ = app.emit("document-deleted", id);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_documents(
    state: State<'_, AppState>,
    project_id: Option<i64>,
) -> Result<Vec<DocumentWithProject>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let sql = match project_id {
        Some(_) => format!("{DOC_SELECT} WHERE d.project_id = ?1 AND p.user_id = ?2 ORDER BY d.updated_at DESC"),
        None => format!("{DOC_SELECT} WHERE p.user_id = ?1 ORDER BY d.updated_at DESC"),
    };
    let mut stmt = db.prepare(&sql)?;
    let rows = match project_id {
        Some(pid) => stmt.query_map(params![pid, uid], document_with_project_from_row)?,
        None => stmt.query_map([uid], document_with_project_from_row)?,
    };
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_document(state: State<'_, AppState>, id: i64) -> Result<Document, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    db.query_row(
        "SELECT d.id, d.project_id, d.title, d.content, d.created_at, d.updated_at
         FROM documents d JOIN projects p ON p.id = d.project_id
         WHERE d.id = ?1 AND p.user_id = ?2",
        params![id, uid],
        document_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Document {id} not found.")),
        other => AppError::Db(other),
    })
}