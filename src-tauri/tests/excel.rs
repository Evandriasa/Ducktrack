use std::sync::Mutex;

use calamine::Reader;
use ducktrack_lib::database::{connection, AppState};
use tauri::Manager;

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "ducktrack-test-{tag}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&base).unwrap();
    base
}

fn seed(db: &rusqlite::Connection) {
    let ts = ducktrack_lib::database::now();
    db.execute(
        "INSERT INTO projects (key, name, description, status, priority, user_id, created_at, updated_at)
         VALUES ('EXC', 'Excel Project', 'Imported round-trip fixture', 'Active', 'High', 1, ?1, ?1)",
        [&ts],
    )
    .unwrap();
    let pid: i64 = db.last_insert_rowid();
    db.execute(
        "INSERT INTO tasks (project_id, key, title, type, status, priority, assignee, due_date, estimated_minutes, created_at, updated_at)
         VALUES (?1, 'EXC-001', 'Verify export', 'Task', 'Todo', 'Medium', 'Alice', '2026-01-15', 120, ?2, ?2)",
        params![pid, ts],
    )
    .unwrap();
    db.execute(
        "INSERT INTO work_logs (project_id, task_id, description, duration_minutes, user_id, created_at)
         VALUES (?1, NULL, 'Testing excel export', 45, 1, ?2)",
        params![pid, ts],
    )
    .unwrap();
}

#[test]
fn excel_export_and_import_round_trip() {
    let dir = temp_dir("excel");
    let workspace = connection::Workspace::ensure(&dir).unwrap();
    let db = connection::open(&workspace.db_path).unwrap();
    seed(&db);

    let app = tauri::test::mock_builder()
        .manage(AppState {
            db: Mutex::new(db),
            workspace,
            current_user_id: Mutex::new(Some(1)),
        })
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();

    let state = app.state::<AppState>();

    // Export all sheets.
    let path = ducktrack_lib::commands::export_excel(state.clone()).expect("export_excel should succeed");
    assert!(std::path::Path::new(&path).exists(), "xlsx should be created at {path}");

    // Verify the workbook has all expected sheets with correct headers.
    let mut sheets = calamine::open_workbook_auto(&path).expect("calamine should open the workbook");
    let names = sheets.sheet_names().to_vec();
    for expected in ["Overview", "Projects", "Tasks", "Work Logs", "Documents", "Inbox", "5-Why"] {
        assert!(names.contains(&expected.to_string()), "missing sheet {expected}: {names:?}");
    }
    let pj = sheets.worksheet_range("Projects").expect("Projects sheet exists");
    let mut rows = pj.rows();
    let header: Vec<String> = rows
        .next()
        .unwrap()
        .iter()
        .map(|d| ducktrack_lib::commands::excel_cell_string(Some(d)))
        .collect();
    assert_eq!(&header, &["Key", "Name", "Status", "Priority", "Description", "Created", "Updated"]);
    let data: Vec<Vec<String>> = rows.map(|r| r.iter().map(|d| ducktrack_lib::commands::excel_cell_string(Some(d))).collect()).collect();
    assert_eq!(data.len(), 1, "should export the seeded project");
    assert_eq!(&data[0][0], "EXC");
    assert_eq!(&data[0][1], "Excel Project");

    // Import back into a *fresh* workspace -> both create counts should be 1.
    drop(state);
    let dir2 = temp_dir("excel-fresh");
    let workspace2 = connection::Workspace::ensure(&dir2).unwrap();
    let db2 = connection::open(&workspace2.db_path).unwrap();
    let app2 = tauri::test::mock_builder()
        .manage(AppState {
            db: Mutex::new(db2),
            workspace: workspace2,
            current_user_id: Mutex::new(Some(1)),
        })
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let state2 = app2.state::<AppState>();
    let result = ducktrack_lib::commands::import_excel(state2.clone(), path.clone()).expect("import should succeed");
    // Fresh DB: project EXC and task EXC-001 get created, exported work_log has no task so tasks_created = 1.
    assert_eq!(result.projects_created, 1, "fresh DB must create the project");
    assert_eq!(result.tasks_created, 1, "fresh DB must create the task");

    // Re-import on the same DB -> updates, not creates.
    let result2 = ducktrack_lib::commands::import_excel(state2.clone(), path).expect("second import should succeed");
    assert_eq!(result2.projects_updated, 1, "existing project should be updated on re-import");
    assert_eq!(result2.tasks_updated, 1, "existing task should be updated on re-import");
    assert_eq!(result2.projects_created + result2.tasks_created, 0);

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&dir2);
}

use rusqlite::params;

#[test]
fn excel_cell_string_type_is_stable() {
    // Public helper used by tests reads a cell the same way the importer does.
    let _ = ducktrack_lib::commands::excel_cell_string(Some(&calamine::Data::Int(7)));
}