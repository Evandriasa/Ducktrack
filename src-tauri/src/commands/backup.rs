use std::fs;
use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use super::super::database::{now, now_ms, AppState};
use super::super::error::AppError;

use rusqlite::OptionalExtension;

fn sanitize_path_for_sql(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

/// Creates a timestamped backup and prunes old ones, honoring the optional
/// retention count argument. Returns the created backup path.
fn backup_and_prune(
    state: &State<'_, AppState>,
    retention: Option<i64>,
) -> Result<String, AppError> {
    let path = create_backup_inner(state)?;
    if let Some(keep) = retention {
        if keep > 0 {
            prune_backups_inner(state, keep)?;
        }
    }
    Ok(path)
}

fn create_backup_inner(state: &State<'_, AppState>) -> Result<String, AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let timestamp = now_ms().replace([':', 'T', 'Z'], "-");
    let target = state.workspace.backups.join(format!("ducktrack-{timestamp}.db"));
    let target_sql = sanitize_path_for_sql(&target);
    db.execute_batch(&format!("VACUUM INTO '{target_sql}'"))?;
    drop(db);
    Ok(target.to_string_lossy().into_owned())
}

fn prune_backups_inner(state: &State<'_, AppState>, keep: i64) -> Result<(), AppError> {
    let keep = keep.max(0);
    let mut names = scan_backups(&state.workspace.backups);
    names.sort();
    names.reverse();
    if names.len() <= keep as usize {
        return Ok(());
    }
    let excess = names.split_off(keep as usize);
    for name in excess {
        let _ = fs::remove_file(state.workspace.backups.join(&name));
    }
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn auto_backup(
    state: State<'_, AppState>,
    retention: Option<i64>,
) -> Result<String, AppError> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let last = get_setting_str(&state, "lastAutoBackup");
    if last.as_deref() == Some(today.as_str()) {
        return Ok(String::new());
    }
    let keep = match retention {
        Some(keep) => keep,
        None => get_setting_i64(&state, "backupRetentionCount").unwrap_or(30),
    };
    let path = backup_and_prune(&state, Some(keep))?;
    set_setting_str(&state, "lastAutoBackup", &today);
    Ok(path)
}

fn get_setting_str(state: &State<'_, AppState>, key: &str) -> Option<String> {
    let db = state.db.lock().ok()?;
    db.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()
        .ok()
        .flatten()
}

fn set_setting_str(state: &State<'_, AppState>, key: &str, value: &str) {
    if let Ok(db) = state.db.lock() {
        let _ = db.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            rusqlite::params![key, value, now()],
        );
    }
}

fn get_setting_i64(state: &State<'_, AppState>, key: &str) -> Option<i64> {
    let db = state.db.lock().ok()?;
    db.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .and_then(|v| v.parse::<i64>().ok())
}

#[tauri::command(rename_all = "snake_case")]
pub fn prune_backups(state: State<'_, AppState>, keep: i64) -> Result<(), AppError> {
    prune_backups_inner(&state, keep)
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_backup(state: State<'_, AppState>) -> Result<String, AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let timestamp = now_ms().replace([':', 'T', 'Z'], "-");
    let target = state.workspace.backups.join(format!("ducktrack-{timestamp}.db"));
    let target_sql = sanitize_path_for_sql(&target);
    db.execute_batch(&format!("VACUUM INTO '{target_sql}'"))?;
    drop(db);
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_backups(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let backups = &state.workspace.backups;
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(backups) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("ducktrack-") && name.ends_with(".db") {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names.reverse();
    Ok(names)
}

fn scan_backups(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("ducktrack-") && name.ends_with(".db") {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names.reverse();
    names
}

#[tauri::command(rename_all = "snake_case")]
pub fn restore_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    name: Option<String>,
) -> Result<(), AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let source = match name {
        Some(n) => {
            // Only ever resolve a plain filename inside the backups dir.
            let base = Path::new(&n).file_name().and_then(|b| b.to_str()).unwrap_or("");
            if base != n.trim() || !n.starts_with("ducktrack-") || !n.ends_with(".db") {
                return Err(AppError::Validation("Invalid backup name.".into()));
            }
            state.workspace.backups.join(&n)
        }
        None => {
            let names = scan_backups(&state.workspace.backups);
            if names.is_empty() {
                return Err(AppError::NotFound("No backups available.".into()));
            }
            state.workspace.backups.join(&names[0])
        }
    };

    if !source.exists() {
        return Err(AppError::NotFound(format!(
            "Backup not found: {}",
            source.display()
        )));
    }

    drop(db);

    let db_path = &state.workspace.db_path;
    // Copy to a temp file first; only swap it in once the copy succeeded, so a
    // failed restore can never wipe the live database.
    let tmp = db_path.with_extension("db.restoring");
    fs::copy(&source, &tmp)?;
    if db_path.exists() {
        fs::remove_file(db_path)?;
    }
    for suffix in ["-wal", "-shm"] {
        let sidecar = db_path.with_extension(format!("db{suffix}"));
        if sidecar.exists() {
            fs::remove_file(sidecar)?;
        }
    }
    fs::rename(&tmp, db_path)?;

    let conn = super::super::database::connection::open(db_path)?;
    *state
        .db
        .lock()
        .map_err(|_| AppError::Other("db lock poisoned".into()))? = conn;

    let _ = app.emit("workspace-restored", ());
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn export_workspace(state: State<'_, AppState>) -> Result<String, AppError> {
    let timestamp = now_ms().replace([':', 'T', 'Z'], "-");
    let target_dir = state.workspace.exports.join(format!("ducktrack-export-{timestamp}"));
    fs::create_dir_all(&target_dir)?;

    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let target_db_sql = sanitize_path_for_sql(&target_dir.join("ducktrack.db"));
    db.execute_batch(&format!("VACUUM INTO '{target_db_sql}'"))?;
    drop(db);

    let export_attachments = target_dir.join("attachments");
    fs::create_dir_all(&export_attachments)?;
    copy_dir(&state.workspace.attachments, &export_attachments)?;

    Ok(target_dir.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn import_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    source_dir: String,
) -> Result<(), AppError> {
    let source_dir = Path::new(&source_dir).to_path_buf();
    let source_db = source_dir.join("ducktrack.db");
    if !source_db.exists() {
        return Err(AppError::NotFound(format!(
            "No ducktrack.db found in {}",
            source_dir.display()
        )));
    }

    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    drop(db);

    let db_path = &state.workspace.db_path;
    if db_path.exists() {
        fs::remove_file(db_path)?;
    }
    for suffix in ["-wal", "-shm"] {
        let sidecar = db_path.with_extension(format!("db{suffix}"));
        if sidecar.exists() {
            fs::remove_file(sidecar)?;
        }
    }
    fs::copy(&source_db, db_path)?;

    let source_attachments = source_dir.join("attachments");
    if source_attachments.exists() {
        for entry in fs::read_dir(&source_attachments)?.flatten() {
            let dest = state.workspace.attachments.join(entry.file_name());
            if entry.path().is_dir() {
                copy_dir(&entry.path(), &dest)?;
            } else if !dest.exists() {
                fs::copy(entry.path(), dest)?;
            }
        }
    }

    let conn = super::super::database::connection::open(db_path)?;
    *state
        .db
        .lock()
        .map_err(|_| AppError::Other("db lock poisoned".into()))? = conn;

    let _ = app.emit("workspace-restored", ());
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), AppError> {
    if !from.exists() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let dest = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else if !dest.exists() {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}