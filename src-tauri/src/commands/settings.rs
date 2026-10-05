use rusqlite::OptionalExtension;
use tauri::State;

use super::super::database::AppState;
use super::super::error::{ensure_non_empty, AppError};

#[tauri::command(rename_all = "snake_case")]
pub fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, AppError> {
    ensure_non_empty(&key, "Key")?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let value = db
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?;
    Ok(value)
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    ensure_non_empty(&key, "Key")?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    db.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![key, value, super::super::database::now()],
    )?;
    Ok(())
}