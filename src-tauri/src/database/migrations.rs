use rusqlite::Connection;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("migrations/0001_init.sql")),
    ("0002_extended", include_str!("migrations/0002_extended.sql")),
    ("0003_index", include_str!("migrations/0003_index.sql")),
    ("0004_users", include_str!("migrations/0004_users.sql")),
    ("0005_roadmap", include_str!("migrations/0005_roadmap.sql")),
    ("0006_five_whys", include_str!("migrations/0006_five_whys.sql")),
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