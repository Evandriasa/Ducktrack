pub mod connection;
pub mod indexer;
pub mod migrations;

use std::sync::Mutex;

use rusqlite::Connection;

pub use connection::Workspace;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub workspace: Workspace,
    pub current_user_id: Mutex<Option<i64>>,
}

impl AppState {
    pub fn active_user(&self) -> Result<i64, crate::error::AppError> {
        self.current_user_id
            .lock()
            .map_err(|_| crate::error::AppError::Other("db lock poisoned".into()))?
            .ok_or_else(|| crate::error::AppError::Other("Not logged in.".into()))
    }
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn now_ms() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}