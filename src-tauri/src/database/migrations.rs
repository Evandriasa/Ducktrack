use rusqlite::Connection;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("migrations/0001_init.sql")),
    ("0002_extended", include_str!("migrations/0002_extended.sql")),
    ("0003_index", include_str!("migrations/0003_index.sql")),
    ("0004_users", include_str!("migrations/0004_users.sql")),
    ("0005_roadmap", include_str!("migrations/0005_roadmap.sql")),
    ("0006_five_whys", include_str!("migrations/0006_five_whys.sql")),
    ("0007_attachments_five_why", include_str!("migrations/0007_attachments_five_why.sql")),
];

pub fn run(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    for (version, sql) in MIGRATIONS {
        let applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [*version],
            |row| row.get(0),
        )?;

        if !applied {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at)
                 VALUES (?1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                [*version],
            )?;
            tx.commit()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn connect() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn
    }

    #[test]
    fn fresh_database_applies_all_migrations() {
        let conn = connect();
        run(&conn).unwrap();

        // 0007 adds five_why_id to attachments and indexes it.
        let col: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('attachments') WHERE name = 'five_why_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col, 1, "attachments.five_why_id column should exist");

        let fw: i64 = conn
            .query_row("SELECT COUNT(*) FROM five_whys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fw, 0);
    }

    #[test]
    fn rerunning_migrations_is_idempotent() {
        let conn = connect();
        run(&conn).unwrap();
        run(&conn).unwrap();

        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(applied, MIGRATIONS.len() as i64);
    }
}