-- 0001_init.sql — core tables

CREATE TABLE IF NOT EXISTS users (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id    INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    key          TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'Active',
    priority     TEXT NOT NULL DEFAULT 'Medium',
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS tasks (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id     INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    parent_task_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
    key            TEXT NOT NULL UNIQUE,
    title          TEXT NOT NULL,
    description    TEXT NOT NULL DEFAULT '',
    status         TEXT NOT NULL DEFAULT 'Todo',
    priority       TEXT NOT NULL DEFAULT 'Medium',
    type           TEXT NOT NULL DEFAULT 'Task',
    due_date       TEXT,
    assignee       TEXT,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    completed_at   TEXT
);

CREATE INDEX IF NOT EXISTS idx_tasks_project   ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_parent    ON tasks(parent_task_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status    ON tasks(status);

CREATE TABLE IF NOT EXISTS task_history (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id    INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    action     TEXT NOT NULL,
    field      TEXT,
    old_value  TEXT,
    new_value  TEXT,
    created_at TEXT NOT NULL,
    user_id    INTEGER REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_history_task    ON task_history(task_id);
CREATE INDEX IF NOT EXISTS idx_history_created ON task_history(created_at);

CREATE TABLE IF NOT EXISTS work_logs (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id       INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    task_id          INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    description      TEXT NOT NULL,
    duration_minutes INTEGER,
    created_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_worklogs_task    ON work_logs(task_id);
CREATE INDEX IF NOT EXISTS idx_worklogs_project ON work_logs(project_id);
CREATE INDEX IF NOT EXISTS idx_worklogs_created ON work_logs(created_at);

INSERT OR IGNORE INTO users (id, name, created_at)
VALUES (1, 'Local User', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));