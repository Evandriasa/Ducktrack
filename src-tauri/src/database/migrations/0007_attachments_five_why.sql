-- Allow evidence files to be attached directly to a 5-Why analysis.
-- Attachments already carry task_id / project_id; this adds the third owner.
ALTER TABLE attachments ADD COLUMN five_why_id INTEGER REFERENCES five_whys(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_attachments_five_why ON attachments(five_why_id);
CREATE INDEX IF NOT EXISTS idx_attachments_task ON attachments(task_id);
CREATE INDEX IF NOT EXISTS idx_attachments_project ON attachments(project_id);
