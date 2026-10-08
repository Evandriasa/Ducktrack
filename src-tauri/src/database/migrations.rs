use rusqlite::Connection;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("migrations/0001_init.sql")),
    ("0002_extended", include_str!("migrations/0002_extended.sql")),
    ("0003_index", include_str!("migrations/0003_index.sql")),
    ("0004_users", include_str!("migrations/0004_users.sql")),
    ("0005_roadmap", include_str!("migrations/0005_roadmap.sql")),
    ("0006_five_whys", include_str!("migrations/0006_five_whys.sql")),
    ("0007_attachments_five_why", include_str!("migrations/0007_attachments_five_why.sql")),
    ("0008_tags_user", include_str!("migrations/0008_tags_user.sql")),
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

        // 0008 makes tags per-user: tags.user_id exists and the name
        // uniqueness is scoped by (user_id, name).
        let col: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('tags') WHERE name = 'user_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col, 1, "tags.user_id column should exist");

        conn.execute(
            "INSERT INTO users (name, created_at) VALUES ('a', '2026-01-01T00:00:00Z'), ('b', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO tags (name, user_id) VALUES ('x', 1), ('x', 2), ('y', 1)", [])
            .unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM tags WHERE user_id = 2 AND name = 'x'", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1,
            "two users may each have a tag named x"
        );
        let dup = conn.execute("INSERT INTO tags (name, user_id) VALUES ('y', 1)", []);
        assert!(dup.is_err(), "same user + same name must still be unique");
    }

    #[test]
    fn migration_0008_backfills_existing_tags() {
        let conn = connect();
        // Apply only up to 0007, seed data, then apply 0008.
        for (version, sql) in MIGRATIONS {
            if version == &"0008_tags_user" {
                break;
            }
            conn.execute_batch(sql).unwrap();
        }
        conn.execute(
            "INSERT INTO users (id, name, created_at) VALUES (2, 'b', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO projects (user_id, key, name, created_at, updated_at) VALUES (1, 'P1', 'P1', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'), (2, 'P2', 'P2', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')", [])
            .unwrap();
        conn.execute("INSERT INTO tasks (project_id, key, title, created_at, updated_at) VALUES (1, 'P1-001', 't1', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'), (2, 'P2-001', 't2', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')", [])
            .unwrap();
        conn.execute("INSERT INTO tags (id, name) VALUES (1, 'mine'), (2, 'theirs')", [])
            .unwrap();
        conn.execute("INSERT INTO task_tags (task_id, tag_id) VALUES (1, 1), (2, 2)", [])
            .unwrap();
        conn.execute_batch(include_str!("migrations/0008_tags_user.sql")).unwrap();

        let (uid1, uid2): (i64, i64) = (
            conn.query_row("SELECT user_id FROM tags WHERE id = 1", [], |r| r.get(0))
                .unwrap(),
            conn.query_row("SELECT user_id FROM tags WHERE id = 2", [], |r| r.get(0))
                .unwrap(),
        );
        assert_eq!(uid1, 1, "tag used by user 1's task stays with user 1");
        assert_eq!(uid2, 2, "tag used by user 2's task stays with user 2");
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