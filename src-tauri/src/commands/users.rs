use std::io::Read;

use rusqlite::{params, Row};
use sha2::{Digest, Sha256};
use tauri::State;

use super::super::database::{now, AppState};
use super::super::error::{ensure_non_empty, AppError};
use super::super::models::{User, UserInput, UserUpdate};

pub fn user_from_row(row: &Row) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    match std::fs::File::open("/dev/urandom") {
        Ok(mut f) => {
            let _ = f.read_exact(&mut buf);
        }
        Err(_) => {
            let t = now();
            let mut idx: usize = 0;
            for b in buf.iter_mut() {
                idx = (idx + 1) % t.len();
                *b = t.as_bytes()[idx];
            }
        }
    }
    hex_encode(&buf)
}

fn hash_password(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hex_encode(&hasher.finalize())
}

fn make_stored_password(password: &str) -> String {
    let salt = random_hex(16);
    format!("{salt}${}", hash_password(&salt, password))
}

fn verify_password(stored: &str, password: &str) -> bool {
    match stored.split_once('$') {
        Some((salt, expected)) if !salt.is_empty() => {
            hash_password(salt, password) == expected
        }
        _ => false,
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_users(state: State<'_, AppState>) -> Result<Vec<User>, AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let mut stmt = db.prepare(
        "SELECT id, name, created_at, updated_at FROM users ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], user_from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_user(state: State<'_, AppState>, input: UserInput) -> Result<User, AppError> {
    ensure_non_empty(&input.name, "Name")?;
    if input.password.is_empty() {
        return Err(AppError::Validation("Password must not be empty.".into()));
    }
    let name = input.name.trim();

    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE name = ?1 COLLATE NOCASE)",
        [name],
        |r| r.get(0),
    )?;
    if exists {
        return Err(AppError::Validation("A user with that name already exists.".into()));
    }

    let ts = now();
    db.execute(
        "INSERT INTO users (name, password, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![name, make_stored_password(&input.password), ts],
    )?;
    let id = db.last_insert_rowid();
    db.query_row(
        "SELECT id, name, created_at, updated_at FROM users WHERE id = ?1",
        [id],
        user_from_row,
    )
    .map_err(AppError::from)
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_user(
    state: State<'_, AppState>,
    id: i64,
    input: UserUpdate,
) -> Result<User, AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let ts = now();

    if let Some(name) = &input.name {
        if name.trim().is_empty() {
            return Err(AppError::Validation("Name must not be empty.".into()));
        }
        let taken: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE name = ?1 COLLATE NOCASE AND id <> ?2)",
            params![name.trim(), id],
            |r| r.get(0),
        )?;
        if taken {
            return Err(AppError::Validation("A user with that name already exists.".into()));
        }
        db.execute(
            "UPDATE users SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name.trim(), ts, id],
        )?;
    }

    if let Some(password) = &input.password {
        if password.is_empty() {
            return Err(AppError::Validation("Password must not be empty.".into()));
        }
        db.execute(
            "UPDATE users SET password = ?1, updated_at = ?2 WHERE id = ?3",
            params![make_stored_password(password), ts, id],
        )?;
    }

    db.query_row(
        "SELECT id, name, created_at, updated_at FROM users WHERE id = ?1",
        [id],
        user_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("User {id} not found.")),
        other => AppError::Db(other),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_user(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let current = state.active_user()?;
    if id == current {
        return Err(AppError::Validation(
            "You cannot delete the user you are logged in as.".into(),
        ));
    }

    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let count: i64 = db.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
    if count <= 1 {
        return Err(AppError::Validation(
            "Cannot delete the last remaining user.".into(),
        ));
    }

    let owned: i64 = db.query_row(
        "SELECT COUNT(*) FROM projects WHERE user_id = ?1",
        [id],
        |r| r.get(0),
    )?;
    if owned > 0 {
        return Err(AppError::Validation(format!(
            "This user still owns {owned} project(s). Delete those projects first."
        )));
    }

    let tx = db.transaction()?;
    tx.execute("DELETE FROM inbox WHERE user_id = ?1", [id])?;
    tx.execute("DELETE FROM timers WHERE user_id = ?1", [id])?;
    tx.execute("DELETE FROM search_index WHERE user_id = ?1", [id])?;
    tx.execute("UPDATE comments SET user_id = NULL WHERE user_id = ?1", [id])?;
    tx.execute("UPDATE task_history SET user_id = NULL WHERE user_id = ?1", [id])?;
    tx.execute("DELETE FROM users WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn login_user(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<User, AppError> {
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let username = username.trim();

    let _ = debug_log(format!(
        "login_user ENTER username={username:?} password_len={}",
        password.len(),
    ));
    let row = db
        .query_row(
            "SELECT id, name, created_at, updated_at, password FROM users WHERE name = ?1 COLLATE NOCASE",
            [username],
            user_from_row_with_password,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::Validation("Invalid username or password.".into())
            }
            other => AppError::Db(other),
        })?;
    let (user, stored) = row;
    let _ = debug_log(format!(
        "login_user DBRESULT user_id={} stored_pw_len={}",
        user.id,
        stored.len()
    ));

    if stored.is_empty() {
        db.execute(
            "UPDATE users SET password = ?1, updated_at = ?2 WHERE id = ?3",
            params![make_stored_password(&password), now(), user.id],
        )?;
        let _ = debug_log(format!("login_user VERDICT OK (empty stored -> set from supplied pw) user_id={}", user.id));
    } else if !verify_password(&stored, &password) {
        let _ = debug_log(format!("login_user VERDICT FAIL verify_password(stored, supplied) = false user_id={}", user.id));
        return Err(AppError::Validation("Invalid username or password.".into()));
    } else {
        let _ = debug_log(format!("login_user VERDICT OK verify_password PASS user_id={}", user.id));
    }

    *state
        .current_user_id
        .lock()
        .map_err(|_| AppError::Other("db lock poisoned".into()))? = Some(user.id);

    Ok(user)
}

fn user_from_row_with_password(row: &Row) -> rusqlite::Result<(User, String)> {
    Ok((
        User {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        },
        row.get(4)?,
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub fn logout_user(state: State<'_, AppState>) -> Result<(), AppError> {
    *state
        .current_user_id
        .lock()
        .map_err(|_| AppError::Other("db lock poisoned".into()))? = None;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn debug_log(msg: String) -> Result<(), AppError> {
    use std::io::Write;
    let line = format!("[{}] {msg}\n", now());
    let path = "/tmp/ducktrack-debug.log";
    let result = || -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        f.write_all(line.as_bytes())
    }();
    if let Err(e) = result {
        return Err(AppError::Other(format!("debug_log write failed: {e}")));
    }
    Ok(())
}
