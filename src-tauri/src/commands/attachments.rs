use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use super::super::database::{indexer, now, AppState};
use super::super::error::{ensure_non_empty, AppError};

static MIME_TO_EXT: &[(&str, &str)] = &[
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/gif", "gif"),
    ("image/webp", "webp"),
    ("image/svg+xml", "svg"),
    ("image/bmp", "bmp"),
    ("image/x-icon", "ico"),
];

fn ext_for_mime(mime: &str) -> &str {
    MIME_TO_EXT
        .iter()
        .find(|(m, _)| *m == mime)
        .map(|(_, e)| *e)
        .unwrap_or("png")
}

fn mime_for_filename(filename: &str) -> Option<String> {
    let ext = filename.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase())?;
    MIME_TO_EXT
        .iter()
        .find(|(_, e)| *e == ext)
        .map(|(m, _)| (*m).to_string())
}

fn filename_for(label: &str, mime: Option<&str>, roll: i64) -> String {
    let ext = mime.map(ext_for_mime).unwrap_or("png");
    let id = now().replace([':', 'T', 'Z', '.', '-'], "_");
    let roll = roll.max(0);
    format!("{roll:05}_{id}_{label}.{ext}")
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: i64,
    pub task_id: Option<i64>,
    pub project_id: Option<i64>,
    pub filename: String,
    pub path: String,
    pub size: i64,
    pub mime: Option<String>,
    pub created_at: String,
}

fn attachment_from_row(row: &Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get(0)?,
        task_id: row.get(1)?,
        project_id: row.get(2)?,
        filename: row.get(3)?,
        path: row.get(4)?,
        size: row.get(5)?,
        mime: row.get(6)?,
        created_at: row.get(7)?,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImageInput {
    pub data_url: String,
    pub task_id: Option<i64>,
    pub project_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AttachmentCtx {
    pub id: i64,
    pub task_id: Option<i64>,
    pub project_id: Option<i64>,
    pub user_id: i64,
}

/// Store a data-URL image (clipboard paste / screenshot) as an attachment.
#[tauri::command(rename_all = "snake_case")]
pub fn add_image_data(
    app: AppHandle,
    state: State<'_, AppState>,
    input: ImageInput,
) -> Result<Attachment, AppError> {
    let (mime, bytes) = super::base64::decode_data_url(&input.data_url)
        .map_err(|e| AppError::Validation(format!("Invalid image data: {e}")))?;
    if bytes.is_empty() {
        return Err(AppError::Validation("Image data is empty.".into()));
    }

    let uid = state.active_user()?;
    let ts = now();
    let id = now().replace([':', 'T', 'Z', '.', '-'], "_");
    let filename = format!("screenshot-{id}.{}", ext_for_mime(mime.as_deref().unwrap_or("image/png")));
    let dest = state.workspace.attachments.join(&filename);

    fs::write(&dest, &bytes)?;
    let size = fs::metadata(&dest)?.len() as i64;

    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    if let Some(tid) = input.task_id {
        let owned: bool = tx.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id
                WHERE t.id = ?1 AND p.user_id = ?2
            )",
            params![tid, uid],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
    }
    if let Some(pid) = input.project_id {
        let owned: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![pid, uid],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
    }

    let ts_short = ts.replace([':', 'T', 'Z', '.', '-'], "_");
    tx.execute(
        "INSERT INTO attachments (task_id, project_id, filename, path, size, mime, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![input.task_id, input.project_id, filename, dest.to_string_lossy(), size, mime, ts],
    )?;
    let rowid = tx.last_insert_rowid();
    if let Some(tid) = input.task_id {
        let task_key: String = tx.query_row("SELECT key FROM tasks WHERE id = ?1", [tid], |r| r.get(0))?;
        let _ = indexer::upsert(&tx, "attachment", rowid, uid, &format!("Attachment on {task_key}"), &filename)?;
    }

    let attachment: Attachment = tx.query_row(
        "SELECT id, task_id, project_id, filename, path, size, mime, created_at
         FROM attachments WHERE id = ?1",
        [rowid],
        attachment_from_row,
    )?;
    tx.commit()?;
    drop(db);

    let _ = app.emit("attachment-added", rowid);
    Ok(attachment)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AttachmentBytesInput {
    pub data: String,
    pub filename: String,
    pub task_id: Option<i64>,
    pub project_id: Option<i64>,
    pub mime: Option<String>,
}

/// Store a file's bytes (base64, from the webview) as an attachment.
#[tauri::command(rename_all = "snake_case")]
pub fn add_attachment_bytes(
    app: AppHandle,
    state: State<'_, AppState>,
    input: AttachmentBytesInput,
) -> Result<Attachment, AppError> {
    ensure_non_empty(&input.data, "Data")?;
    ensure_non_empty(&input.filename, "Filename")?;
    let bytes = super::base64::decode(&input.data)
        .map_err(|e| AppError::Validation(format!("Invalid file data: {e}")))?;
    if bytes.is_empty() {
        return Err(AppError::Validation("File data is empty.".into()));
    }

    let uid = state.active_user()?;
    let ts = now();
    let id = now().replace([':', 'T', 'Z', '.', '-'], "_");
    let filename = input.filename.trim();
    let filename = if filename.is_empty() {
        format!("attachment-{id}")
    } else {
        filename.to_string()
    };
    let dest = state.workspace.attachments.join(&format!("{id}__{filename}"));

    fs::write(&dest, &bytes)?;
    let size = bytes.len() as i64;

    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    if let Some(tid) = input.task_id {
        let owned: bool = tx.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id
                WHERE t.id = ?1 AND p.user_id = ?2
            )",
            params![tid, uid],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
    }
    if let Some(pid) = input.project_id {
        let owned: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![pid, uid],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
    }

    tx.execute(
        "INSERT INTO attachments (task_id, project_id, filename, path, size, mime, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![input.task_id, input.project_id, filename, dest.to_string_lossy(), size, input.mime, ts],
    )?;
    let rowid = tx.last_insert_rowid();
    if let Some(tid) = input.task_id {
        let task_key: String = tx.query_row("SELECT key FROM tasks WHERE id = ?1", [tid], |r| r.get(0))?;
        let _ = indexer::upsert(&tx, "attachment", rowid, uid, &format!("Attachment on {task_key}"), &filename)?;
    }

    let attachment: Attachment = tx.query_row(
        "SELECT id, task_id, project_id, filename, path, size, mime, created_at
         FROM attachments WHERE id = ?1",
        [rowid],
        attachment_from_row,
    )?;
    tx.commit()?;
    drop(db);

    let _ = app.emit("attachment-added", rowid);
    Ok(attachment)
}

/// Read an attachment back out as a data URL for inline previews.
#[tauri::command(rename_all = "snake_case")]
pub fn read_attachment_data(
    state: State<'_, AppState>,
    id: i64,
) -> Result<AttachmentRef, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let (filename, path, size, mime): (String, String, i64, Option<String>) = db.query_row(
        "SELECT a.filename, a.path, a.size, a.mime
         FROM attachments a
         WHERE a.id = ?1 AND (
             EXISTS(SELECT 1 FROM projects p WHERE p.id = a.project_id AND p.user_id = ?2)
             OR EXISTS(
                 SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id
                 WHERE t.id = a.task_id AND p.user_id = ?2
             )
         )",
        params![id, uid],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;

    let bytes = fs::read(&path)?;
    let mime = mime
        .filter(|m| !m.is_empty())
        .or_else(|| mime_for_filename(&filename))
        .unwrap_or_else(|| "image/png".to_string());
    let data_url = super::base64::encode_data_url(Some(&mime), &bytes);

    drop(db);
    Ok(AttachmentRef { id, filename, size, mime: Some(mime), data_url })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentRef {
    pub id: i64,
    pub filename: String,
    pub size: i64,
    pub mime: Option<String>,
    pub data_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AttachmentInput {
    pub src_path: String,
    pub task_id: Option<i64>,
    pub project_id: Option<i64>,
    pub mime: Option<String>,
}

/// Copy a file on disk into the attachments folder and register it.
#[tauri::command(rename_all = "snake_case")]
pub fn add_attachment(
    app: AppHandle,
    state: State<'_, AppState>,
    input: AttachmentInput,
) -> Result<Attachment, AppError> {
    ensure_non_empty(&input.src_path, "Source path")?;
    let src = Path::new(&input.src_path);
    if !src.exists() {
        return Err(AppError::NotFound(format!(
            "Source file not found: {}",
            src.display()
        )));
    }

    let uid = state.active_user()?;
    let ts = now();
    let id = now().replace([':', 'T', 'Z', '.', '-'], "_");
    let filename = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("attachment-{id}"));
    let dest = state.workspace.attachments.join(&format!("{id}__{filename}"));

    fs::copy(&src, &dest)?;
    let size = fs::metadata(&dest)?.len() as i64;

    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;

    if let Some(tid) = input.task_id {
        let owned: bool = tx.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM tasks t JOIN projects p ON p.id = t.project_id
                WHERE t.id = ?1 AND p.user_id = ?2
            )",
            params![tid, uid],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(AppError::NotFound(format!("Task {tid} not found.")));
        }
    }
    if let Some(pid) = input.project_id {
        let owned: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![pid, uid],
            |r| r.get(0),
        )?;
        if !owned {
            return Err(AppError::NotFound(format!("Project {pid} not found.")));
        }
    }

    tx.execute(
        "INSERT INTO attachments (task_id, project_id, filename, path, size, mime, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![input.task_id, input.project_id, filename, dest.to_string_lossy(), size, input.mime, ts],
    )?;
    let rowid = tx.last_insert_rowid();
    if let Some(tid) = input.task_id {
        let task_key: String = tx.query_row("SELECT key FROM tasks WHERE id = ?1", [tid], |r| r.get(0))?;
        let _ = indexer::upsert(&tx, "attachment", rowid, uid, &format!("Attachment on {task_key}"), &filename)?;
    }

    let attachment: Attachment = tx.query_row(
        "SELECT id, task_id, project_id, filename, path, size, mime, created_at
         FROM attachments WHERE id = ?1",
        [rowid],
        attachment_from_row,
    )?;
    tx.commit()?;
    drop(db);

    let _ = app.emit("attachment-added", rowid);
    Ok(attachment)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_attachments(
    state: State<'_, AppState>,
    task_id: Option<i64>,
    project_id: Option<i64>,
) -> Result<Vec<Attachment>, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let rows: Vec<Attachment> = if let Some(tid) = task_id {
        db.prepare(
            "SELECT a.id, a.task_id, a.project_id, a.filename, a.path, a.size, a.mime, a.created_at
             FROM attachments a
             JOIN tasks t ON t.id = a.task_id
             LEFT JOIN projects p ON p.id = t.project_id
             WHERE a.task_id = ?1 AND p.user_id = ?2
             ORDER BY a.created_at DESC",
        )?
        .query_map(params![tid, uid], attachment_from_row)?
        .collect::<Result<_, _>>()?
    } else if let Some(pid) = project_id {
        db.prepare(
            "SELECT a.id, a.task_id, a.project_id, a.filename, a.path, a.size, a.mime, a.created_at
             FROM attachments a
             LEFT JOIN projects p ON p.id = a.project_id
             WHERE a.project_id = ?1 AND p.user_id = ?2
             ORDER BY a.created_at DESC",
        )?
        .query_map(params![pid, uid], attachment_from_row)?
        .collect::<Result<_, _>>()?
    } else {
        db.prepare(
            "SELECT a.id, a.task_id, a.project_id, a.filename, a.path, a.size, a.mime, a.created_at
             FROM attachments a
             LEFT JOIN tasks t ON t.id = a.task_id
             LEFT JOIN projects p ON p.id = COALESCE(t.project_id, a.project_id)
             WHERE p.user_id = ?1
             ORDER BY a.created_at DESC",
        )?
        .query_map([uid], attachment_from_row)?
        .collect::<Result<_, _>>()?
    };
    Ok(rows)
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_attachment(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), AppError> {
    let uid = state.active_user()?;
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let owned: bool = tx.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM attachments a
            LEFT JOIN tasks t ON t.id = a.task_id
            LEFT JOIN projects p ON p.id = COALESCE(t.project_id, a.project_id)
            WHERE a.id = ?1 AND p.user_id = ?2
        )",
        params![id, uid],
        |r| r.get(0),
    )?;
    if !owned {
        return Err(AppError::NotFound(format!("Attachment {id} not found.")));
    }
    let path: String = tx.query_row(
        "SELECT a.path FROM attachments a WHERE a.id = ?1",
        [id],
        |r| r.get(0),
    )?;
    tx.execute("DELETE FROM attachments WHERE id = ?1", [id])?;
    tx.commit()?;
    let _ = fs::remove_file(&path);
    let _ = app.emit("attachment-removed", id);
    Ok(())
}
