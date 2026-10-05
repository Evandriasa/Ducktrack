use rusqlite::{params, Connection};

use super::now;

pub fn upsert(
    conn: &Connection,
    category: &str,
    item_id: i64,
    user_id: i64,
    title: &str,
    body: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO search_index (category, item_id, user_id, title, body, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(category, item_id) DO UPDATE SET
            user_id = excluded.user_id,
            title = excluded.title,
            body = excluded.body",
        params![category, item_id, user_id, title, body, now()],
    )?;
    Ok(())
}

pub fn remove(conn: &Connection, category: &str, item_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM search_index WHERE category = ?1 AND item_id = ?2",
        params![category, item_id],
    )?;
    Ok(())
}