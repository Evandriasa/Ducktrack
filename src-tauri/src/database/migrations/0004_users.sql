-- 0004_users.sql — accounts, per-user ownership

ALTER TABLE users ADD COLUMN password TEXT NOT NULL DEFAULT '';
ALTER TABLE users ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';

ALTER TABLE projects ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE work_logs ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE search_index ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;

CREATE INDEX IF NOT EXISTS idx_projects_user ON projects(user_id);
CREATE INDEX IF NOT EXISTS idx_worklogs_user ON work_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_search_user ON search_index(user_id);