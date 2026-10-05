pub mod commands;
pub mod database;
pub mod error;
pub mod models;

use tauri::Manager;

use error::AppError;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("could not resolve app data directory");
            let workspace = database::connection::Workspace::ensure(&app_data_dir)?;
            let db = database::connection::open(&workspace.db_path)?;

            app.manage(database::AppState {
                db: std::sync::Mutex::new(db),
                workspace,
                current_user_id: std::sync::Mutex::new(None),
            });

            if std::env::var("DT_SEED_DEMO").as_deref() == Ok("1") {
                seed_demo(app.handle())?;
            }

            let state = app.state::<database::AppState>();
            let _ = commands::auto_backup(state, None);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Projects
            commands::create_project,
            commands::get_project,
            commands::update_project,
            commands::delete_project,
            commands::list_projects,
            commands::get_project_tree,
            commands::get_project_stats,
            // Tasks
            commands::create_task,
            commands::get_task,
            commands::update_task,
            commands::delete_task,
            commands::list_tasks,
            commands::complete_task,
            commands::list_subtasks,
            // History
            commands::get_task_history,
            commands::get_project_history,
            commands::get_global_history,
            // Work logs
            commands::create_work_log,
            commands::update_work_log,
            commands::delete_work_log,
            commands::list_work_logs,
            // Documents
            commands::create_document,
            commands::get_document,
            commands::update_document,
            commands::delete_document,
            commands::list_documents,
            // Attachments
            commands::add_attachment,
            commands::add_attachment_bytes,
            commands::add_image_data,
            commands::read_attachment_data,
            commands::remove_attachment,
            commands::list_attachments,
            // Search
            commands::global_search,
            // Backup
            commands::create_backup,
            commands::list_backups,
            commands::restore_backup,
            commands::export_workspace,
            commands::import_workspace,
            // Dashboard
            commands::get_dashboard_stats,
            commands::get_workspace_info,
            commands::reset_workspace,
            // Users
            commands::list_users,
            commands::create_user,
            commands::update_user,
            commands::delete_user,
            commands::login_user,
            commands::logout_user,
            commands::debug_log,
            // Tags
            commands::list_tags,
            commands::create_tag,
            commands::update_tag,
            commands::delete_tag,
            commands::add_task_tag,
            commands::remove_task_tag,
            commands::list_task_tags,
            commands::add_project_tag,
            commands::remove_project_tag,
            commands::list_project_tags,
            // Comments
            commands::create_comment,
            commands::list_comments,
            commands::delete_comment,
            // Relationships
            commands::add_relationship,
            commands::list_relationships,
            commands::remove_relationship,
            // Inbox
            commands::create_inbox_item,
            commands::list_inbox,
            commands::update_inbox_item,
            commands::delete_inbox_item,
            commands::get_inbox_count,
            // Timers
            commands::get_active_timer,
            commands::start_timer,
            commands::stop_timer,
            // Journal & reports
            commands::get_daily_summary,
            commands::get_summary_report,
            commands::get_blocked_tasks,
            // Portfolio
            commands::export_portfolio,
            // 5-Why
            commands::list_five_whys,
            commands::get_five_why,
            commands::create_five_why,
            commands::update_five_why,
            commands::delete_five_why,
            // Excel import/export
            commands::export_excel,
            commands::import_excel,
            // Settings
            commands::get_setting,
            commands::set_setting,
            // Updates
            commands::check_for_updates,
            commands::open_github_page,
            // Overdue
            commands::get_overdue_tasks,
            // Task lookup (mentions)
            commands::get_task_by_key,
            commands::auto_backup,
            commands::prune_backups,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DuckTrack");
}

fn seed_demo(app: &tauri::AppHandle) -> Result<(), AppError> {
    use rusqlite::OptionalExtension;

    let state = app.state::<database::AppState>();
    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let has_projects: bool = db
        .query_row("SELECT EXISTS(SELECT 1 FROM projects)", [], |r| r.get(0))
        .optional()?
        .unwrap_or(false);
    if has_projects {
        return Ok(());
    }

    let tx = db.transaction()?;
    let ts = database::now();

    let insert_project = |tx: &rusqlite::Transaction<'_>, parent: Option<i64>, key: &str, name: &str, description: &str| -> Result<i64, rusqlite::Error> {
        tx.execute(
            "INSERT INTO projects (parent_id, key, name, description, status, priority, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'Active', 'Medium', ?5, ?5)",
            rusqlite::params![parent, key, name, description, ts],
        )?;
        Ok(tx.last_insert_rowid())
    };

    let agv_middleware = insert_project(&tx, None, "AGV", "AGV Systems", "AGV fleet monitoring, middleware and operational tooling")?;
    let agv_mw_child = insert_project(&tx, Some(agv_middleware), "MWS", "AGV Middleware", "Operations Hub, AGV V4 and infrastructure")?;
    let pilot = insert_project(&tx, None, "PILOT", "Pilot Line", "Pilot production line recovery and automation")?;
    let robot = insert_project(&tx, None, "ROBOT", "Cartesian Robot", "Speaker contour glue application")?;
    insert_project(&tx, None, "TEST", "Plant Testing", "Acoustic measurements, labels and test procedures")?;

    let insert_task = |tx: &rusqlite::Transaction<'_>, project: i64, key: &str, title: &str, status: &str, priority: &str, task_type: &str, description: &str, minutes_ago: i64| -> Result<(), rusqlite::Error> {
        let created = chrono::Utc::now() - chrono::Duration::minutes(minutes_ago);
        let created = created.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let completed_at = if status == "Completed" { Some(database::now()) } else { None };
        tx.execute(
            "INSERT INTO tasks (project_id, key, title, description, status, priority, type, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9)",
            rusqlite::params![project, key, title, description, status, priority, task_type, created, completed_at],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO task_history (task_id, action, field, old_value, new_value, created_at, user_id)
             VALUES (?1, 'created', 'status', NULL, ?2, ?3, 1)",
            rusqlite::params![id, status, created],
        )?;
        if status == "Completed" {
            tx.execute(
                "INSERT INTO task_history (task_id, action, field, old_value, new_value, created_at, user_id)
                 VALUES (?1, 'completed', 'status', 'In Progress', 'Completed', ?2, 1)",
                rusqlite::params![id, database::now()],
            )?;
        }
        Ok(())
    };

    insert_task(&tx, agv_mw_child, "AGV-001", "Fix topology mismatch", "Completed", "High", "Bug", "Incorrect connection between station 14 and station 18 in the middleware topology.", 5000)?;
    insert_task(&tx, agv_mw_child, "AGV-002", "Add backend date/time filtering", "In Progress", "High", "Feature", "SQL filtering for historical events now supports date and time ranges.", 100)?;
    insert_task(&tx, agv_mw_child, "AGV-003", "Test WebSocket connection", "Todo", "Medium", "Investigation", "Verify WebSocket connectivity between Operations Hub and AGV V4.", 30)?;
    insert_task(&tx, pilot, "PILOT-001", "Repair glue pump pressure system", "In Progress", "High", "Maintenance", "Disassembled glue pump assembly and inspected pressure regulation.", 60)?;
    insert_task(&tx, pilot, "PILOT-002", "Record station 14 reference values", "Blocked", "Medium", "Investigation", "Waiting for dry-run validation before capturing reference values.", 0)?;
    insert_task(&tx, robot, "ROBOT-011", "Teach speaker contour glue path", "Todo", "Medium", "Task", "Teach glue path along the speaker contour.", 200)?;

    insert_task_worklog(&tx, agv_mw_child, "AGV-002", 1, "Investigated topology mismatch. Found incorrect connection between station 14 and station 18.", 135, 90)?;
    insert_task_worklog(&tx, pilot, "PILOT-001", 0, "Disassembled the glue pump assembly and inspected the pressure regulator.", 100, 100)?;

    // index seed rows
    let mut projects_seed = tx.prepare("SELECT id, key, name, description FROM projects")?;
    let proj_rows: Vec<(i64, String, String, String)> = projects_seed
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(projects_seed);
    for (id, key, name, desc) in proj_rows {
        database::indexer::upsert(&tx, "project", id, 1, &format!("{key} — {name}"), &desc)?;
    }

    let mut tasks_seed = tx.prepare("SELECT id, key, title, description FROM tasks")?;
    let task_rows: Vec<(i64, String, String, String)> = tasks_seed
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(tasks_seed);
    for (id, key, title, desc) in task_rows {
        database::indexer::upsert(&tx, "task", id, 1, &format!("{key} — {title}"), &desc)?;
    }

    let docs = [
        (agv_mw_child, "AGV Middleware Architecture", "System overview: WebSockets, Node-RED and MSSQL integration."),
        (agv_mw_child, "AGV History Event Model", "Historical states, logging rules and filtering."),
        (pilot, "Pilot Line Recovery", "Known issues, repairs and station documentation."),
        (robot, "Cartesian Robot Setup", "Dry-run procedure and glue path teaching."),
        (0i64, "", ""),
    ];
    for (project, title, content) in docs.iter().filter(|(_, t, _)| !t.is_empty()) {
        tx.execute(
            "INSERT INTO documents (project_id, title, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            rusqlite::params![project, *title, *content, ts],
        )?;
        let id = tx.last_insert_rowid();
        database::indexer::upsert(&tx, "document", id, 1, &format!("{project} — {title}"), content)?;
    }

    let worklog_rows: Vec<(i64, Option<i64>, String)> = {
        let mut stmt = tx.prepare(
            "SELECT w.id, w.task_id, w.description FROM work_logs w",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        rows
    };
    for (id, task_id, desc) in worklog_rows {
        let label = match task_id {
            Some(tid) => {
                let key: String = tx.query_row("SELECT key FROM tasks WHERE id = ?1", [tid], |r| r.get(0))?;
                format!("Work log on {key}")
            }
            None => "Work log".to_string(),
        };
        database::indexer::upsert(&tx, "worklog", id, 1, &label, &desc)?;
    }

    tx.commit()?;
    Ok(())
}

fn insert_task_worklog(
    tx: &rusqlite::Transaction<'_>,
    project_id: i64,
    task_key: &str,
    _idx: i64,
    description: &str,
    duration: i64,
    minutes_ago: i64,
) -> Result<(), rusqlite::Error> {
    let task_id: i64 = match tx.query_row(
        "SELECT id FROM tasks WHERE key = ?1",
        [task_key],
        |r| r.get(0),
    ) {
        Ok(id) => id,
        Err(_) => return Ok(()),
    };
    let created = chrono::Utc::now() - chrono::Duration::minutes(minutes_ago);
    let created = created.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    tx.execute(
        "INSERT INTO work_logs (project_id, task_id, description, duration_minutes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![project_id, task_id, description, duration, created],
    )?;
    Ok(())
}