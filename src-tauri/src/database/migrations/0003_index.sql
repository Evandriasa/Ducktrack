-- 0003_index.sql — global search index (FTS5)

CREATE TABLE IF NOT EXISTS search_index (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    category   TEXT NOT NULL,
    item_id    INTEGER NOT NULL,
    title      TEXT NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_search_item ON search_index(category, item_id);

CREATE VIRTUAL TABLE IF NOT EXISTS fts_workspace USING fts5(
    category,
    title,
    body,
    content = 'search_index',
    content_rowid = 'id',
    tokenize = 'unicode61'
);

CREATE TRIGGER IF NOT EXISTS trg_search_insert AFTER INSERT ON search_index BEGIN
    INSERT INTO fts_workspace(rowid, category, title, body)
    VALUES (new.id, new.category, new.title, new.body);
END;

CREATE TRIGGER IF NOT EXISTS trg_search_delete AFTER DELETE ON search_index BEGIN
    INSERT INTO fts_workspace(fts_workspace, rowid, category, title, body)
    VALUES ('delete', old.id, old.category, old.title, old.body);
END;

CREATE TRIGGER IF NOT EXISTS trg_search_update AFTER UPDATE ON search_index BEGIN
    INSERT INTO fts_workspace(fts_workspace, rowid, category, title, body)
    VALUES ('delete', old.id, old.category, old.title, old.body);
    INSERT INTO fts_workspace(rowid, category, title, body)
    VALUES (new.id, new.category, new.title, new.body);
END;