-- 0006_five_whys: root cause analysis records

CREATE TABLE IF NOT EXISTS five_whys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
    task_id INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
    problem TEXT NOT NULL DEFAULT '',
    why_1 TEXT,
    why_2 TEXT,
    why_3 TEXT,
    why_4 TEXT,
    why_5 TEXT,
    root_cause TEXT,
    corrective_action TEXT,
    owner TEXT,
    due_date TEXT,
    status TEXT NOT NULL DEFAULT 'Open',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_five_whys_user ON five_whys(user_id);