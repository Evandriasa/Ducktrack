-- 0005_roadmap: inbox, work timers, app settings, task estimates

CREATE TABLE IF NOT EXISTS inbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL DEFAULT 1,
    text TEXT NOT NULL,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    promoted_task_id INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    promoted_project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_inbox_user ON inbox(user_id);

CREATE TABLE IF NOT EXISTS timers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_timers_user ON timers(user_id);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

ALTER TABLE tasks ADD COLUMN estimated_minutes INTEGER;