export async function render(container) {
  container.innerHTML = `
    <div class="page-max" style="max-width:860px">
      <div class="page-head">
        <div>
          <div class="eyebrow">Reference</div>
          <h1 class="page-title">Help</h1>
          <p class="page-desc">How DuckTrack works, and how to get the most out of it.</p>
        </div>
      </div>

      <div class="card card-pad mb-2">
        <h3 class="small" style="font-weight:600">How DuckTrack works</h3>
        <p class="small mt-2" style="line-height:1.7">
          DuckTrack is a <strong>local-first</strong> engineering workspace. Everything is stored on this computer in a
          single SQLite database plus an <code>attachments/</code> folder — nothing leaves your machine and it works fully offline.
          The hierarchy is simple: <strong>projects</strong> can contain other projects to any depth, <strong>tasks</strong> live inside
          a project, and every meaningful mutation to a task is recorded automatically as a permanent <strong>change log</strong> entry.
        </p>
      </div>

      <div class="card card-pad mb-2">
        <h3 class="small" style="font-weight:600">Pages</h3>
        <div class="mt-2 stack" style="gap:10px">
          <div><span class="key">Dashboard</span> — today's numbers, active work, recent activity and project progress. Quick-capture ideas into your inbox.</div>
          <div><span class="key">Inbox</span> — raw thoughts captured fast, then promoted into real tasks or completed.</div>
          <div><span class="key">My Tasks</span> — all tasks with filters by status, tag and sort order. Overdue tasks are flagged red.</div>
          <div><span class="key">Projects</span> — the nested project tree, a card view, and a Kanban board per status.</div>
          <div><span class="key">Change Log</span> — the automatic, immutable history of task changes. Filters by date, project, task and action.</div>
          <div><span class="key">Journal</span> — what actually happened: daily work logs, task timeline, a period report and blocked tasks.</div>
          <div><span class="key">Work Log</span> — what you actually did. Logs can be linked to a task, a project, both or neither.</div>
          <div><span class="key">Documentation</span> — project-linked knowledge, stored in Markdown and rendered readably.</div>
          <div><span class="key">Portfolio</span> — a professional summary built automatically from real history, exportable as Markdown + HTML.</div>
          <div><span class="key">Settings</span> — users, theme, tags, backups, export/import and reset.</div>
        </div>
      </div>

      <div class="card card-pad mb-2">
        <h3 class="small" style="font-weight:600">Keyboard shortcuts</h3>
        <div class="mt-2 stack" style="gap:8px">
          <div><span class="kbd">Ctrl K</span> global search</div>
          <div><span class="kbd">Ctrl P</span> quick actions / command palette</div>
          <div><span class="kbd">Ctrl 1…9</span> jump directly to a page</div>
          <div><span class="kbd">Ctrl /</span> this help page</div>
          <div><span class="kbd">Esc</span> close any dialog</div>
        </div>
      </div>

      <div class="card card-pad mb-2">
        <h3 class="small" style="font-weight:600">Accounts</h3>
        <p class="small mt-2" style="line-height:1.7">
          DuckTrack supports multiple local users. Each user signs in on launch and gets an <strong>isolated workspace</strong> —
          projects, tasks, work logs, documents and search only show that user's data. Add, rename and delete users in
          Settings → Users. Passwords are stored as salted hashes, never in plain text.
        </p>
      </div>

      <div class="card card-pad mb-2">
        <h3 class="small" style="font-weight:600">Data, backup and export</h3>
        <p class="small mt-2" style="line-height:1.7">
          Your data lives in <code>Settings → Workspace</code>. Create a backup at any time and restore the latest in
          Settings → Backup. Export copies the whole workspace (database + attachments); import restores it. "Reset workspace"
          empties everything — use it carefully.
        </p>
      </div>

      <div class="card card-pad">
        <h3 class="small" style="font-weight:600">Tips</h3>
        <div class="mt-2 stack" style="gap:8px">
          <div>• Use <strong>project keys</strong> like <code>AGV</code> — task identifiers are derived from them (<code>AGV-042</code>).</div>
          <div>• Mention a task anywhere by typing <code>@@KEY</code> (e.g. <code>@@AGV-042</code>) — it becomes a clickable link.</div>
          <div>• Click a task name anywhere to open it: statuses, priorities, quick work logs, tags, comments, relationships, timers and estimates.</div>
          <div>• Log work against a task even while it stays In Progress — status and effort are separate concepts.</div>
          <div>• Start a <strong>timer</strong> from a task; stopping it automatically logs the elapsed time as a work log.</div>
        </div>
      </div>
    </div>`;
}