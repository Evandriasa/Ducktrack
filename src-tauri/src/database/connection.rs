use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::migrations;

pub struct Workspace {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub attachments: PathBuf,
    pub backups: PathBuf,
    pub exports: PathBuf,
}

impl Workspace {
    pub fn ensure(app_data_dir: &Path) -> Result<Self, rusqlite::Error> {
        let root = app_data_dir.join("workspace");
        fs::create_dir_all(&root).map_err(|e| rusqlite::Error::ToSqlConversionFailure(
            Box::new(e),
        ))?;

        let attachments = root.join("attachments");
        let backups = root.join("backups");
        let exports = root.join("exports");

        for dir in [&attachments, &backups, &exports] {
            fs::create_dir_all(dir).map_err(|e| rusqlite::Error::ToSqlConversionFailure(
                Box::new(e),
            ))?;
        }

        Ok(Self {
            db_path: root.join("ducktrack.db"),
            attachments,
            backups,
            exports,
            root,
        })
    }
}

pub fn open(db_path: &Path) -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", "5000")?;
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    migrations::run(&conn)?;
    Ok(conn)
}