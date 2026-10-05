# DuckTrack — Engineering Workspace

## 1. Vision

DuckTrack is a local-first desktop project management and engineering workspace inspired by tools such as ClickUp and GitHub Projects/Issues.

The goal is not to build another generic todo list. DuckTrack should provide one clean place to:

- Capture todos and engineering tasks.
- Organize work into projects and nested project structures.
- Record what was actually done.
- Automatically maintain a permanent change history.
- Store project documentation and supporting files.
- Search across the entire workspace.
- Present completed work professionally as a portfolio.
- Work fully offline without requiring a server.

The application should feel like an engineering tool: structured, fast, dense but readable, keyboard-friendly, and reliable.

---

## 2. Core Principles

### Local first

All important data lives locally. The application must work without internet access or a database server.

### Data ownership

The user's workspace should be easy to back up, export, move, and restore.

### Automatic history

Meaningful changes should be recorded automatically by the backend. The frontend must not be responsible for maintaining the audit trail.

### Stable identifiers

Projects and tasks receive stable human-readable keys that do not change after creation.

### Clean separation

The frontend handles presentation and user interaction.

The Rust backend handles:

- Validation.
- Database access.
- Business logic.
- Transactions.
- History generation.
- File operations.
- Events.

### Build a vertical slice first

The first milestone should prove the architecture with a small but fully working workflow rather than attempting the entire application at once.

---

# 3. Technology

## Desktop

- Tauri
- Rust

## Frontend

- HTML
- JavaScript (ES modules)
- Plain CSS with a custom dark theme

React/Vue/etc. are intentionally avoided. The frontend is deliberately framework-free.

## Database

- SQLite
- rusqlite
- SQLite FTS5 for global search

## Target

Linux (the current development and test platform).

Windows and macOS builds may follow later.

---

# 4. High-Level Architecture

```text
┌─────────────────────────────────────────────┐
│                 DuckTrack                   │
│                                             │
│  HTML + JavaScript + Tailwind + Lucide      │
│                    │                        │
│              Tauri Commands                 │
│                    │                        │
│              Rust Backend                   │
│                    │                        │
│          ┌─────────┴─────────┐              │
│          │                   │              │
│       SQLite             File System        │
│          │                   │              │
│     ducktrack.db       attachments/         │
└─────────────────────────────────────────────┘
```

The frontend never directly modifies SQLite.

All database mutations go through Rust commands.

---

# 5. Workspace Structure

The application should use a portable workspace structure:

```text
DuckTrack/
├── ducktrack.db
├── attachments/
├── backups/
└── exports/
```

The executable itself can be installed separately.

A future portable .ducktrack archive may contain the database, attachments, and metadata.

---

# 6. Main Navigation

## V1

- Dashboard
- My Tasks
- Projects
- Work Log
- Change Log
- Documentation
- Portfolio
- Settings

## Future

- Calendar
- Roadmap
- Reports
- Automation
- Integrations

---

# 7. Dashboard

The Dashboard answers:

**What is happening right now?**

It should show:

- Tasks currently in progress.
- Blocked tasks.
- High/critical priority tasks.
- Recently completed work.
- Recent activity.
- Project progress.
- Recent work logs.
- Quick actions.

Example:

```text
┌────────────────────────────────────────────────────┐
│ Dashboard                                          │
├────────────────────────────────────────────────────┤
│                                                    │
│  In Progress    Blocked    Completed    Projects   │
│      4             1          37           6        │
│                                                    │
├────────────────────────────────────────────────────┤
│ Active Tasks                                       │
│                                                    │
│ AGV-042  Fix topology mismatch          In Progress│
│ AGV-043  Add history filtering           In Progress│
│ PILOT-017 Repair glue pump                    Blocked│
│                                                    │
├────────────────────────────────────────────────────┤
│ Recent Activity                                    │
│                                                    │
│ AGV-043 completed                                  │
│ AGV-042 status changed                             │
│ PILOT-017 work logged                              │
└────────────────────────────────────────────────────┘
```

---

# 8. Projects

Projects organize the workspace hierarchically.

Projects can contain projects to arbitrary depth.

Example:

```text
AGV Systems
├── AGV Middleware
│   ├── Operations Hub
│   ├── AGV V4
│   └── Infrastructure
├── AGV Maintenance
└── AGV Infrastructure
```

## Project fields

- id
- parent_id
- key
- name
- description
- status
- priority
- created_at
- updated_at
- completed_at

## Project keys

Examples:

- AGV
- PILOT
- ROBOT
- TEST

These keys are used to generate task identifiers.

---

# 9. Tasks

Tasks are the primary actionable unit.

Example:

- AGV-001
- AGV-042
- PILOT-017
- ROBOT-011

## Task fields

- id
- project_id
- parent_task_id
- key
- title
- description
- status
- priority
- type
- due_date
- created_at
- updated_at
- completed_at
- assignee

## Statuses

- Todo
- In Progress
- Blocked
- Completed
- Cancelled

Running should not be used as a normal historical task state. That represents live machine/runtime state and does not belong in the task history model.

## Priorities

- Low
- Medium
- High
- Critical

## Types

- Task
- Bug
- Feature
- Improvement
- Maintenance
- Investigation
- Documentation

## Task capabilities

Tasks should support:

- Subtasks.
- Tags.
- Comments.
- Attachments.
- Relationships.
- Work logs.
- Automatic history.
- Due dates.
- Project association.

---

# 10. Task History

Every meaningful task mutation creates an append-only history record.

## Table: task_history

- id
- task_id
- action
- field
- old_value
- new_value
- created_at
- user_id

Examples:

- Task created
- Status changed: Todo → In Progress
- Priority changed: Medium → High
- Title changed
- Description changed
- Task completed
- Task reopened
- Attachment added
- Comment added

History is generated by Rust.

The frontend cannot be trusted to create audit records.

## Transaction requirement

A task mutation and its history record must happen inside the same SQLite transaction.

```sql
BEGIN

UPDATE tasks
INSERT INTO task_history ...

COMMIT
```

If either operation fails, the entire transaction is rolled back.

This guarantees that the history cannot silently diverge from the task state.

---

# 11. Change Log

The Change Log is the human-readable presentation of automatic history.

Example:

```text
09 SEP 2026

AGV-038
Added backend date/time filtering

Feature completed

SQL filtering now supports date and time ranges.
```

Filters:

- Date.
- Project.
- Task.
- Change type.
- Search.

The Change Log is historical and should not be treated as a live machine/task-state viewer.

---

# 12. Work Log

Work Logs answer:

**What did I actually do?**

This is intentionally separate from task status.

A task can remain In Progress while multiple work logs are recorded.

A work log can belong to:

- A project.
- A task.
- Both.
- Neither.

## Fields

- id
- project_id
- task_id
- description
- duration_minutes
- created_at
- user_id

## Example

```text
Investigated topology mismatch.

Found incorrect connection between station 14
and station 18.

Corrected topology and started retesting.
```

V1 uses manually entered duration.

Automatic timers can be added later.

---

# 13. Documentation

Documentation is project-linked knowledge.

Example:

```text
AGV Middleware
├── Architecture
├── History Event Model
├── WebSocket API
├── Troubleshooting
└── Deployment
```

## Fields

- id
- project_id
- title
- content
- created_at
- updated_at

Markdown is a good initial storage format.

Future capabilities:

- Rich editor.
- Document version history.
- Table of contents.
- Cross-links.
- Embedded images.
- Project/task references.

---

# 14. Attachments

Files should live outside SQLite.

SQLite stores metadata and file paths.

Initial supported categories:

- Images.
- PDF.
- Text.
- CSV.
- Common Office documents where practical.

Example:

```text
DuckTrack/
├── ducktrack.db
├── attachments/
│   ├── ...
├── backups/
└── exports/
```

---

# 15. Search

Global search should be accessible through:

- Ctrl + K

Search should cover:

- Projects.
- Tasks.
- Work Logs.
- Documentation.
- Change History.
- Comments.

SQLite FTS5 should be used where appropriate.

Example:

```text
Search DuckTrack

> topology

Tasks
AGV-042
AGV-031

Documents
AGV Troubleshooting

Work Logs
08 Sep — topology mismatch

Changes
Topology corrected
```

---

# 16. Tags

Example tags:

- AGV
- Node-RED
- MSSQL
- OPC-UA
- Robotics
- Automation
- Maintenance
- Frontend
- Backend
- Testing

## Tables

### tags

- id
- name

### task_tags

- task_id
- tag_id

### project_tags

- project_id
- tag_id

---

# 17. Relationships

Tasks should eventually support relationships such as:

- blocks
- blocked_by
- relates_to
- duplicates
- depends_on

Potential table:

### relationships

- id
- source_task_id
- target_task_id
- type
- created_at

This does not need to be fully implemented in the first vertical slice.

---

# 18. Users

DuckTrack supports multiple local user accounts with password login.

The application opens with a sign-in screen. On first run the user creates an account, or sets a password for the pre-seeded "Local User". Additional users can be added, renamed and deleted in Settings → Users.

Each user gets an isolated workspace: projects, tasks, work logs, documents, attachments and search are scoped to the signed-in user. History records the acting user.

### users

- id
- name
- password (salted SHA-256 hash, stored as `salt$hash`)
- created_at
- updated_at

### Backend commands

- list_users
- create_user
- update_user
- delete_user
- login_user
- logout_user

Passwords are never stored in plain text. The acting user id is kept in memory for the session and cleared on sign-out.

### Per-user isolation

- projects.user_id
- work_logs.user_id
- search_index.user_id

All read, write and delete commands are scoped to the current user. A user cannot see, modify or delete another user's data.

---

# 19. Suggested Database Schema

Initial core tables:

- users
- projects
- tasks
- task_history
- work_logs
- documents
- attachments
- comments
- tags
- task_tags
- project_tags
- relationships
- settings

The first database migration should focus on:

- users
- projects
- tasks
- task_history
- work_logs

Additional tables can be introduced through later migrations.

---

# 20. Backend Commands

## Projects

- create_project
- get_project
- update_project
- delete_project
- list_projects
- get_project_tree
- get_project_stats

## Tasks

- create_task
- get_task
- update_task
- delete_task
- list_tasks
- complete_task
- list_subtasks

## History

- get_task_history
- get_project_history
- get_global_history

## Work Logs

- create_work_log
- update_work_log
- delete_work_log
- list_work_logs

## Documents

- create_document
- get_document
- update_document
- delete_document
- list_documents

## Attachments

- add_attachment
- remove_attachment
- list_attachments

## Search

- global_search

## Backup

- create_backup
- restore_backup
- list_backups
- export_workspace
- import_workspace

## Dashboard

- get_dashboard_stats
- get_workspace_info
- reset_workspace

## Users

- list_users
- create_user
- update_user
- delete_user
- login_user
- logout_user

---

# 21. Events

V1 should use Tauri events instead of WebSockets.

Example:

```text
Task completed
       ↓
Rust updates SQLite
       ↓
Rust writes task_history
       ↓
Rust emits event
       ↓
Frontend refreshes relevant views
       ↓
Dashboard/project progress updates
```

A WebSocket layer is unnecessary for the local V1 application.

---

# 22. Backup and Export

DuckTrack should make data ownership simple.

Required capabilities:

- Manual backup.
- Restore backup.
- Workspace export.
- Workspace import.
- Automatic daily/weekly backups (planned).
- Configurable retention (planned).

Potential future archive:

```text
project.ducktrack

containing:

ducktrack.db
attachments/
metadata.json
```

---

# 23. Portfolio

The Portfolio is presentation-oriented.

It should turn real DuckTrack data into professional proof of work.

Potential statistics:

- Completed tasks
- Features / improvements
- Bug fixes
- Projects
- Work logs
- Documentation
- Changes

Future export formats:

- PDF.
- HTML.
- Markdown.

The portfolio should use actual project history rather than manually maintained claims.

---

# 24. UI Direction

The visual style should be:

- Dark.
- Engineering-oriented.
- Clean.
- Dense but readable.
- Minimal noise.
- Strong hierarchy.
- Clear status indicators.
- Consistent cards.
- Keyboard-friendly.

Basic layout:

```text
┌──────────────┬────────────────────────────────────┐
│ Sidebar      │ Header / Search                    │
│              ├────────────────────────────────────┤
│ Dashboard    │                                    │
│ Tasks        │ Main Content                       │
│ Projects     │                                    │
│ Work Log     │                                    │
│ Change Log   │                                    │
│ Docs         │                                    │
│ Portfolio    │                                    │
│ Settings     │                                    │
└──────────────┴────────────────────────────────────┘
```

---

# 25. Project Structure

Current source tree:

```text
DuckTrack/
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── error.rs
│   │   ├── models/
│   │   │   └── mod.rs
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── projects.rs
│   │   │   ├── tasks.rs
│   │   │   ├── history.rs
│   │   │   ├── worklogs.rs
│   │   │   ├── documents.rs
│   │   │   ├── attachments.rs
│   │   │   ├── search.rs
│   │   │   ├── backup.rs
│   │   │   ├── dashboard.rs
│   │   │   └── users.rs
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs
│   │   │   ├── indexer.rs
│   │   │   ├── migrations.rs
│   │   │   └── migrations/
│   │   │       ├── 0001_init.sql
│   │   │       ├── 0002_extended.sql
│   │   │       ├── 0003_index.sql
│   │   │       └── 0004_users.sql
│   │   └── tauri.conf.json
│   │
│   └── tests/
│       └── contract_test.rs
│
├── src/
│   ├── index.html
│   ├── css/
│   │   └── app.css
│   └── js/
│       ├── app.js
│       ├── api.js
│       ├── ui.js
│       ├── dashboard.js
│       ├── tasks.js
│       ├── projects.js
│       ├── changelog.js
│       ├── worklog.js
│       ├── documents.js
│       ├── portfolio.js
│       └── settings.js
│
└── README.md
```

---

# 26. V1 Scope

## Implemented

- Tauri desktop application.
- Rust backend.
- SQLite database.
- Database migrations.
- Dashboard.
- Projects.
- Nested projects.
- Tasks.
- Subtasks.
- Statuses.
- Priorities.
- Automatic task history.
- Change Log.
- Work Log with edit/delete.
- Global search.
- Dark UI.
- Manual backup / restore.
- Workspace export / import.
- Attachments.
- Document store with edit/delete.
- Portfolio view.
- Multi-user local accounts with login.
- Workspace reset.

## Schema exists, UI/commands not yet built

- Comments.
- Tags / task_tags / project_tags.
- Task relationships.

## Future

- Markdown rendering for documents.
- Automatic daily/weekly backups + retention.
- Portfolio export (PDF/HTML/Markdown).
- Calendar, roadmap, reports.
- Keyboard shortcut breadth and polish.

## Explicitly not V1

- Cloud synchronization.
- Networked multi-user collaboration (local accounts exist; server syncing does not).
- Chat.
- AI assistant.
- Complex permissions.
- Mobile application.
- Calendar integration.
- GitHub integration.
- Automated time tracking.
- Large integration layer.

---

# 27. Development Phases

## Phase 0 — Prototype

Already conceptually completed.

The UI concept has been validated with an HTML prototype.

## Phase 1 — Working Desktop Skeleton

Build:

- Tauri.
- Rust.
- SQLite.
- Database initialization.
- Migration system.
- Basic frontend.
- Dashboard.

Success condition:

Running the application should:

- Open DuckTrack.
- Create ducktrack.db if it does not exist.
- Initialize the schema.
- Display the Dashboard.

## Phase 2 — Projects

Implement:

- Project creation.
- Project editing.
- Project deletion.
- Nested projects.
- Project tree.
- Project progress.

## Phase 3 — Tasks

Implement:

- Task creation.
- Task editing.
- Task deletion.
- Task identifiers.
- Subtasks.
- Status.
- Priority.
- Type.
- Due dates.

## Phase 4 — History

Implement:

- Automatic history.
- Transaction-safe mutations.
- Task history.
- Project history.
- Global Change Log.
- History filtering.

## Phase 5 — Work Logs

Implement:

- Create work log.
- Edit work log.
- Delete work log.
- Link work log to task/project.
- Duration.

## Phase 6 — Search

Implement:

- Global search.
- FTS5 where useful.
- Keyboard shortcut.
- Search results grouped by type.

## Phase 7 — Documentation and Attachments

Implement:

- Markdown documents.
- Project-linked docs.
- Attachments.
- File metadata.
- Attachment previews where practical.

## Phase 8 — Portfolio

Implement:

- Completed work overview.
- Statistics.
- Project summaries.
- Professional presentation.
- Export options.

## Phase 9 — Backup and Export

Implement:

- Manual backup.
- Automatic backup.
- Restore.
- Workspace export/import.
- Retention settings.

## Phase 10 — Polish

Implement:

- Keyboard shortcuts.
- Animations.
- Loading states.
- Empty states.
- Error handling.
- Performance improvements.
- Accessibility.
- Packaging.
- Windows installer.
- Optional update system.

---

# 28. V1 Success Criteria

DuckTrack V1 is successful when the following workflow works end-to-end:

1. Create a project.
2. Create a nested project.
3. Create a task.
4. Create a subtask.
5. Change the task status.
6. Complete the task.
7. See every meaningful change in history.
8. Record work against the task.
9. Search for the task/work.
10. See project progress.
11. Close and reopen DuckTrack without losing data.
12. Create a backup.
13. Restore the workspace.
14. Work entirely offline.

---

# 29. First Working Demo

The first architecture-proving demo should contain:

## Projects

```text
├── AGV Middleware
└── Pilot Line
```

## Tasks

- AGV-001 Fix topology
- AGV-002 Add history filtering
- PILOT-001 Repair glue pump

## Recent Activity

- AGV-001 created
- AGV-001 status changed
- AGV-002 completed

Then create:

- AGV-003 Test WebSocket connection

Change:

```text
Todo
  ↓
In Progress
  ↓
Completed
```

The Change Log must automatically show:

- AGV-003 created
- Status: Todo → In Progress
- Status: In Progress → Completed

If this workflow works, the fundamental DuckTrack architecture is proven.

---

# 30. Immediate Build Order

The fastest useful implementation path is:

1. Create the Tauri project.
2. Initialize SQLite.
3. Add migrations.
4. Create core tables.
5. Port the current Dashboard prototype.
6. Implement project creation.
7. Implement task creation.
8. Implement task status changes.
9. Automatically write history inside the same transaction.
10. Display the Change Log.
11. Package the first Windows build.

This deliberately creates a small end-to-end product before expanding the feature set.

---

# 31. Future Integrations

Potential integrations after V1:

- Node-RED
- Git
- GitHub
- GitLab
- MSSQL
- OPC-UA
- Windows notifications
- File-system watchers

Node-RED could eventually push engineering activity into DuckTrack.

Example:

```text
Node-RED
   ↓
DuckTrack command/API
   ↓
Work Log / Change Log / Task
```

---

# 32. Future Cloud Sync

Cloud synchronization is deliberately postponed.

A possible future architecture:

```text
Desktop A
    ↓
 SQLite
    ↓
Sync Server
    ↑
 SQLite
    ↑
Desktop B
```

The local-first architecture should remain valid even if synchronization is added later.

---

# 33. Design Rule for Future Features

Before adding a feature, ask:

1. Does it improve project/task organization?
2. Does it help record actual work?
3. Does it improve traceability/history?
4. Does it improve documentation?
5. Does it improve professional presentation?
6. Does it preserve local ownership of the data?
7. Does it keep the application simple enough to maintain?

If the answer is mostly no, the feature probably does not belong in DuckTrack.

---

# 34. Definition of Done

A feature is considered done only when:

- The UI works.
- Backend validation exists.
- Database mutations are correct.
- Errors are handled.
- History is generated where applicable.
- Data survives application restart.
- The feature does not introduce unnecessary coupling.
- The feature is documented sufficiently to maintain it.

---

# 35. Long-Term Goal

DuckTrack should become a personal engineering operating system:

```text
Ideas
  ↓
Projects
  ↓
Tasks
  ↓
Work
  ↓
History
  ↓
Documentation
  ↓
Portfolio
```

The key idea is that the user should not have to maintain the same information in several places.

Work performed in DuckTrack should naturally build the project's history, and that history should naturally become documentation and proof of completed work.

---

# 36. Implementation Status

## Complete and verified

- **Dashboard** — stats, active work, recent activity, project progress, time-aware greeting.
- **Projects** — create, edit, delete, nest, open, progress, parent selection, cycle prevention.
- **Tasks** — create, edit, delete, subtasks, status, priority, type, due date, assignee, quick work log.
- **Change Log** — automatic, transaction-safe history with filters.
- **Work Log** — create, edit, delete, link to project/task, duration.
- **Documentation** — create, edit, delete, project-linked, list.
- **Search** — Ctrl+K, FTS5 with LIKE fallback, grouped results (tasks, projects, work logs, documents, attachments), navigation to items.
- **Attachments** — file copy into workspace, metadata, remove.
- **Backup / restore / export / import** — manual.
- **Portfolio** — statistics and summaries from real data.
- **Users** — local accounts, sign-in screen, per-user isolation, add/edit/delete in Settings, sign-out.
- **Tags** — global tag store, assign to tasks and projects, filter My Tasks by tag, create inline.
- **Comments** — per-task comments with user attribution, add/delete.
- **Task relationships** — `blocks`, `blocked_by`, `relates_to`, `duplicates`, `depends_on` links between tasks with a picker.
- **Markdown** — documents render with a dependency-free Markdown renderer (code, tables, headings, lists, links).
- **Inbox** — quick capture column, promote-to-task, complete, reopen, delete, per-user counts.
- **Work timers** — one active timer per user, start/stop with elapsed tracking; stopping auto-creates a linked work log.
- **Journal** — per-day summary (history + work logs), period report table, blocked tasks with blocker keys.
- **Estimates** — per-task estimated minutes shown in task lists and daily dashboard stats.
- **Overdue flagging** — dashboard strip + red badges for past-due open tasks; native notification on launch.
- **Portfolio export** — generates Markdown + HTML from real history.
- **Settings** — users, appearance (light/dark theme), tag manager, workspace backup/restore/auto-backup with retention, export/import, reset.
- **Help** — in-app reference page with keyboard shortcuts.
- **UX polish** — dark/light themes, custom titlebar (min/max/close/drag), command palette (`Ctrl P`), `@@KEY` task mentions, app-styled confirm dialogs, toasts, empty states, workspace reset, keyboard shortcuts (`Ctrl 1…9`, `Ctrl /`, `Ctrl K`, `Ctrl P`), focus rings, modal animations.
- **Quality gates** — 8 contract tests enforce snake_case payloads and camelCase responses.
- **Data safety** — passwords stored as salted SHA-256; data survives restart; migrations are applied in order at startup; automatic daily backups with optional retention pruning; DB verified empty and re-seeded on demand (`DT_SEED_DEMO=1`).

## Not yet implemented

- Full Kanban drag-and-drop (board view is view-only).
- Calendar, roadmap, advanced reports, integrations, cloud sync.

All of these are tracked as future work in sections above.

# 37. Installers

DuckTrack is packaged for Linux, Windows and macOS with Tauri's bundler. Frontend assets are plain files, so no Node/npm step is involved — a Rust toolchain and `tauri-cli` are all that is needed.

| Platform | Formats | Build on |
| --- | --- | --- |
| Linux | `.deb`, `.rpm`, `.AppImage` | Linux |
| Windows | `.exe` (NSIS), `.msi` (WiX) | Windows |
| macOS | `.app`, `.dmg` | macOS |

`tauri build` only produces installers for the OS it runs on, so each format is built on its native platform via the scripts in `scripts/` or the GitHub Actions workflow `.github/workflows/build-installers.yml`:

```bash
# Linux (produces deb/rpm/AppImage under src-tauri/target/release/bundle/)
./scripts/build-linux.sh
```

```powershell
# Windows — builds NSIS .exe + WiX .msi
.\scripts\build-windows.ps1
```

```powershell
# macOS — builds .app + .dmg
.\scripts\build-macos.ps1
```

Notes:

- The `NO_STRIP=1` export in `build-linux.sh` works around linuxdeploy's bundled `strip` failing on the DT_RELR sections produced by glibc ≥ 2.41.
- Icons are generated from `assets/icon.svg` via `cargo tauri icon`.
- Push a tag (`v*`) to trigger the CI workflow, or run it manually from the Actions tab. On a tag it drafts a GitHub release with the installers attached.
- The `deb` package depends on `libwebkit2gtk-4.1-0`; the macOS minimum is 11.0; the Windows NSIS installer is per-user by default.
- Base config lives in `src-tauri/tauri.conf.json` → `bundle` (targets, icons, metadata, per-OS options).