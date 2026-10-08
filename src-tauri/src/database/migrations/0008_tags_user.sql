-- Tags become per-user so that each profile only sees and manages its own tags.
-- Rebuild the table: the old global UNIQUE(name) constraint is replaced by a
-- per-user unique index. Existing tags are attributed to the user who actually
-- used them (via the task/project association), falling back to the default user.
CREATE TABLE tags_new (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    name    TEXT NOT NULL,
    user_id INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX idx_tags_user_name ON tags_new(user_id, name COLLATE NOCASE);

INSERT INTO tags_new (id, name, user_id)
SELECT t.id, t.name,
       COALESCE(
         (SELECT p.user_id
            FROM task_tags tt
            JOIN tasks tk   ON tk.id = tt.task_id
            JOIN projects p ON p.id   = tk.project_id
           WHERE tt.tag_id = t.id ORDER BY tt.tag_id LIMIT 1),
         (SELECT p.user_id
            FROM project_tags pt
            JOIN projects p ON p.id = pt.project_id
           WHERE pt.tag_id = t.id ORDER BY pt.tag_id LIMIT 1),
         1)
  FROM tags t;

DROP TABLE tags;

ALTER TABLE tags_new RENAME TO tags;

CREATE INDEX IF NOT EXISTS idx_tags_user ON tags(user_id);