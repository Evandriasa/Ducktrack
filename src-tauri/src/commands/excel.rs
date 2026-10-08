use std::fs;
use std::io::BufReader;

use calamine::{Data, Reader};
use rusqlite::params;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError};
use tauri::State;

use super::super::database::{indexer, now, now_ms, AppState};
use super::super::error::{ensure_key_valid, AppError};

const SHEET_OVERVIEW: &str = "Overview";
const SHEET_PROJECTS: &str = "Projects";
const SHEET_TASKS: &str = "Tasks";
const SHEET_WORKLOGS: &str = "Work Logs";
const SHEET_DOCS: &str = "Documents";
const SHEET_INBOX: &str = "Inbox";
const SHEET_FIVEWHYS: &str = "5-Why";

const PROJ_COLS: [&str; 7] = ["Key", "Name", "Status", "Priority", "Description", "Created", "Updated"];
const TASK_COLS: [&str; 11] = [
    "Key", "Project Key", "Title", "Type", "Status", "Priority", "Assignee", "Due",
    "Estimated (h)", "Created", "Updated",
];
const WL_COLS: [&str; 5] = ["Date", "Project", "Task", "Description", "Duration (h)"];
const DOC_COLS: [&str; 4] = ["Project", "Title", "Updated", "Content"];
const INBOX_COLS: [&str; 3] = ["Text", "Created", "Completed"];
const FW_COLS: [&str; 13] = [
    "ID", "Problem", "Project", "Task", "Why 1", "Why 2", "Why 3", "Why 4", "Why 5",
    "Root Cause", "Corrective Action", "Owner", "Status",
];

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExcelImportResult {
    pub path: String,
    pub projects_created: i64,
    pub projects_updated: i64,
    pub tasks_created: i64,
    pub tasks_updated: i64,
    pub skipped_missing_project: i64,
}

fn xerr(e: impl std::fmt::Display) -> AppError {
    AppError::Other(format!("Excel error: {e}"))
}

fn ok<T>(r: Result<T, XlsxError>) -> Result<(), AppError> {
    r.map(|_| ()).map_err(xerr)
}

fn header_format() -> Format {
    Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x1F4E78))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
}

fn text_format() -> Format {
    Format::new().set_border(FormatBorder::Thin)
}

fn title_format() -> Format {
    Format::new().set_bold().set_font_size(16).set_font_color(Color::RGB(0x1F4E78))
}

fn stats_format() -> Format {
    Format::new().set_bold().set_font_color(Color::RGB(0x1F4E78))
}

fn write_headers(ws: &mut Worksheet, cols: &[&str]) -> Result<(), AppError> {
    let fmt = header_format();
    for (i, c) in cols.iter().enumerate() {
        ok(ws.write_string_with_format(0, i as u16, *c, &fmt))?;
    }
    Ok(())
}

fn set_widths(ws: &mut Worksheet, widths: &[f64]) -> Result<(), AppError> {
    for (i, w) in widths.iter().enumerate() {
        ok(ws.set_column_width(i as u16, *w))?;
    }
    Ok(())
}

fn finalize(ws: &mut Worksheet, ncols: usize, rows: u32) -> Result<(), AppError> {
    ok(ws.set_freeze_panes(1, 0))?;
    if rows > 1 {
        ok(ws.autofilter(0, 0, rows - 1, ncols as u16 - 1))?;
    }
    ws.set_default_row_height(18);
    Ok(())
}

fn write_text_row(ws: &mut Worksheet, row: u32, vals: &[String]) -> Result<(), AppError> {
    let fmt = text_format();
    for (c, v) in vals.iter().enumerate() {
        ok(ws.write_string_with_format(row, c as u16, v, &fmt))?;
    }
    Ok(())
}

fn fmt_hours(minutes: Option<i64>) -> String {
    match minutes {
        Some(m) => format!("{:.1}", m as f64 / 60.0),
        None => String::new(),
    }
}

pub fn excel_cell_string(d: Option<&Data>) -> String {
    match d {
        Some(Data::String(s)) => s.clone(),
        Some(Data::Float(f)) => {
            let s = f.to_string();
            if s.ends_with(".0") { s[..s.len() - 2].to_string() } else { s }
        }
        Some(Data::Int(i)) => i.to_string(),
        Some(Data::Bool(b)) => if *b { "1".to_string() } else { "0".to_string() },
        Some(Data::DateTime(dt)) => dt.to_string(),
        Some(Data::DateTimeIso(s)) => s.clone(),
        Some(Data::DurationIso(s)) => s.clone(),
        _ => String::new(),
    }
}

fn non_empty(d: Option<&Data>) -> Option<String> {
    let s = excel_cell_string(d).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

#[tauri::command(rename_all = "snake_case")]
pub fn export_excel(state: State<'_, AppState>) -> Result<String, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let user: String = db.query_row("SELECT name FROM users WHERE id = ?1", [uid], |r| r.get(0))?;

    let mut wb = Workbook::new();

    // ---- Overview --------------------------------------------------------
    let ov = wb.add_worksheet();
    ok(ov.set_name(SHEET_OVERVIEW))?;
    let title = title_format();
    ok(ov.write_string_with_format(0, 0, "DuckTrack — Workspace Export", &title))?;
    let sfmt = stats_format();
    ok(ov.write_string_with_format(2, 0, format!("Generated for: {user}").as_str(), &sfmt))?;
    ok(ov.write_string_with_format(3, 0, format!("Generated at: {}", now_ms()).as_str(), &sfmt))?;

    let p_count: i64 = db.query_row("SELECT COUNT(*) FROM projects WHERE user_id = ?1", [uid], |r| r.get(0))?;
    let t_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE p.user_id = ?1",
        [uid],
        |r| r.get(0),
    )?;
    let open_t: i64 = db.query_row(
        "SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id
         WHERE p.user_id = ?1 AND t.status NOT IN ('Completed','Cancelled')",
        [uid],
        |r| r.get(0),
    )?;
    let wl_min: i64 = db.query_row(
        "SELECT COALESCE(SUM(w.duration_minutes),0) FROM work_logs w WHERE w.user_id = ?1",
        [uid],
        |r| r.get(0),
    )?;
    let doc_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM documents d LEFT JOIN projects p ON p.id = d.project_id
         WHERE d.project_id IS NULL OR p.user_id = ?1",
        [uid],
        |r| r.get(0),
    )?;
    let inbox_count: i64 = db.query_row("SELECT COUNT(*) FROM inbox WHERE user_id = ?1", [uid], |r| r.get(0))?;
    let fw_count: i64 = db.query_row("SELECT COUNT(*) FROM five_whys WHERE user_id = ?1", [uid], |r| r.get(0))?;

    let stats: [(&str, String); 7] = [
        ("Projects", p_count.to_string()),
        ("Tasks", t_count.to_string()),
        ("Open tasks", open_t.to_string()),
        ("Work logged", format!("{:.1} h", wl_min as f64 / 60.0)),
        ("Documents", doc_count.to_string()),
        ("Inbox items", inbox_count.to_string()),
        ("5-Why analyses", fw_count.to_string()),
    ];
    for (i, (label, val)) in stats.iter().enumerate() {
        ok(ov.write_string_with_format(6 + i as u32, 0, *label, &sfmt))?;
        ok(ov.write_string_with_format(6 + i as u32, 1, val.as_str(), &text_format()))?;
    }
    ok(ov.set_column_width(0, 26))?;
    ok(ov.set_column_width(1, 16))?;

    // ---- Projects --------------------------------------------------------
    let mut wp = wb.add_worksheet();
    ok(wp.set_name(SHEET_PROJECTS))?;
    write_headers(&mut wp, &PROJ_COLS)?;
    let proj_rows = {
        let mut stmt = db.prepare(
            "SELECT key, name, status, priority, description, created_at, updated_at
             FROM projects WHERE user_id = ?1 ORDER BY name COLLATE NOCASE",
        )?;
        let mut row_i = 1u32;
        let mut it = stmt.query_map([uid], |r| {
            Ok((
                r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?;
        while let Some(row) = it.next() {
            let (key, name, status, priority, desc, created, updated) = row?;
            write_text_row(&mut wp, row_i, &[key, name, status, priority, desc, created, updated])?;
            row_i += 1;
        }
        row_i
    };
    set_widths(&mut wp, &[12.0, 24.0, 12.0, 12.0, 40.0, 22.0, 22.0])?;
    finalize(&mut wp, PROJ_COLS.len(), proj_rows)?;

    // ---- Tasks -----------------------------------------------------------
    let mut wt = wb.add_worksheet();
    ok(wt.set_name(SHEET_TASKS))?;
    write_headers(&mut wt, &TASK_COLS)?;
    let task_rows = {
        let mut stmt = db.prepare(
            "SELECT t.key, p.key, t.title, t.type, t.status, t.priority, t.assignee, t.due_date,
                    t.estimated_minutes, t.created_at, t.updated_at
             FROM tasks t JOIN projects p ON p.id = t.project_id
             WHERE p.user_id = ?1 ORDER BY p.key, t.created_at",
        )?;
        let mut row_i = 1u32;
        let mut it = stmt.query_map([uid], |r| {
            Ok((
                r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?, r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<i64>>(8)?, r.get::<_, String>(9)?, r.get::<_, String>(10)?,
            ))
        })?;
        while let Some(row) = it.next() {
            let (key, pkey, title, ttype, status, priority, assignee, due, est, created, updated) = row?;
            let vals: [String; 11] = [
                key, pkey, title, ttype, status, priority,
                assignee.unwrap_or_default(), due.unwrap_or_default(),
                fmt_hours(est), created, updated,
            ];
            write_text_row(&mut wt, row_i, &vals)?;
            row_i += 1;
        }
        row_i
    };
    set_widths(&mut wt, &[14.0, 12.0, 36.0, 12.0, 12.0, 12.0, 14.0, 12.0, 12.0, 22.0, 22.0])?;
    finalize(&mut wt, TASK_COLS.len(), task_rows)?;

    // ---- Work logs -------------------------------------------------------
    let mut wl = wb.add_worksheet();
    ok(wl.set_name(SHEET_WORKLOGS))?;
    write_headers(&mut wl, &WL_COLS)?;
    let wl_rows = {
        let mut stmt = db.prepare(
            "SELECT w.created_at, p.key, t.key, w.description, w.duration_minutes
             FROM work_logs w
             LEFT JOIN projects p ON p.id = w.project_id
             LEFT JOIN tasks t ON t.id = w.task_id
             WHERE w.user_id = ?1 ORDER BY w.created_at",
        )?;
        let mut row_i = 1u32;
        let mut it = stmt.query_map([uid], |r| {
            Ok((
                r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?, r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        while let Some(row) = it.next() {
            let (created, pkey, tkey, desc, minutes) = row?;
            let vals: [String; 5] = [
                created, pkey.unwrap_or_default(), tkey.unwrap_or_default(), desc,
                fmt_hours(Some(minutes)),
            ];
            write_text_row(&mut wl, row_i, &vals)?;
            row_i += 1;
        }
        row_i
    };
    set_widths(&mut wl, &[22.0, 12.0, 14.0, 40.0, 12.0])?;
    finalize(&mut wl, WL_COLS.len(), wl_rows)?;

    // ---- Documents -------------------------------------------------------
    let mut wd = wb.add_worksheet();
    ok(wd.set_name(SHEET_DOCS))?;
    write_headers(&mut wd, &DOC_COLS)?;
    let doc_rows = {
        let mut stmt = db.prepare(
            "SELECT p.key, d.title, d.updated_at, d.content
             FROM documents d LEFT JOIN projects p ON p.id = d.project_id
             WHERE d.project_id IS NULL OR p.user_id = ?1
             ORDER BY d.updated_at",
        )?;
        let mut row_i = 1u32;
        let mut it = stmt.query_map([uid], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?,
                r.get::<_, String>(2)?, r.get::<_, String>(3)?,
            ))
        })?;
        while let Some(row) = it.next() {
            let (pkey, title, updated, content) = row?;
            let preview: String = content.trim().replace('\n', " ").chars().take(500).collect();
            let vals: [String; 4] = [pkey.unwrap_or_default(), title, updated, preview];
            write_text_row(&mut wd, row_i, &vals)?;
            row_i += 1;
        }
        row_i
    };
    set_widths(&mut wd, &[12.0, 30.0, 22.0, 60.0])?;
    finalize(&mut wd, DOC_COLS.len(), doc_rows)?;

    // ---- Inbox -----------------------------------------------------------
    let mut wi = wb.add_worksheet();
    ok(wi.set_name(SHEET_INBOX))?;
    write_headers(&mut wi, &INBOX_COLS)?;
    let inbox_rows = {
        let mut stmt = db.prepare(
            "SELECT text, created_at, completed_at FROM inbox WHERE user_id = ?1 ORDER BY created_at",
        )?;
        let mut row_i = 1u32;
        let mut it = stmt.query_map([uid], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
        })?;
        while let Some(row) = it.next() {
            let (text, created, completed) = row?;
            let vals: [String; 3] = [text, created, completed.unwrap_or_default()];
            write_text_row(&mut wi, row_i, &vals)?;
            row_i += 1;
        }
        row_i
    };
    set_widths(&mut wi, &[70.0, 22.0, 22.0])?;
    finalize(&mut wi, INBOX_COLS.len(), inbox_rows)?;

    // ---- 5-Why -----------------------------------------------------------
    let mut wf = wb.add_worksheet();
    ok(wf.set_name(SHEET_FIVEWHYS))?;
    write_headers(&mut wf, &FW_COLS)?;
    let fw_rows = {
        let mut stmt = db.prepare(
            "SELECT fw.id, fw.problem, p.key, t.key,
                    fw.why_1, fw.why_2, fw.why_3, fw.why_4, fw.why_5,
                    fw.root_cause, fw.corrective_action, fw.owner, fw.status
             FROM five_whys fw
             LEFT JOIN projects p ON p.id = fw.project_id
             LEFT JOIN tasks t ON t.id = fw.task_id
             WHERE fw.user_id = ?1 ORDER BY fw.updated_at",
        )?;
        let mut row_i = 1u32;
        let mut it = stmt.query_map([uid], |r| {
            Ok((
                r.get::<_, i64>(0)?, r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?, r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?, r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?, r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?, r.get::<_, Option<String>>(9)?,
                r.get::<_, Option<String>>(10)?, r.get::<_, Option<String>>(11)?,
                r.get::<_, String>(12)?,
            ))
        })?;
        while let Some(row) = it.next() {
            let (id, problem, pkey, tkey, w1, w2, w3, w4, w5, rc, ca, owner, status) = row?;
            let vals: [String; 13] = [
                id.to_string(), problem, pkey.unwrap_or_default(), tkey.unwrap_or_default(),
                w1.unwrap_or_default(), w2.unwrap_or_default(), w3.unwrap_or_default(),
                w4.unwrap_or_default(), w5.unwrap_or_default(), rc.unwrap_or_default(),
                ca.unwrap_or_default(), owner.unwrap_or_default(), status,
            ];
            write_text_row(&mut wf, row_i, &vals)?;
            row_i += 1;
        }
        row_i
    };
    set_widths(&mut wf, &[8.0, 40.0, 12.0, 14.0, 26.0, 26.0, 26.0, 26.0, 26.0, 34.0, 34.0, 14.0, 14.0])?;
    finalize(&mut wf, FW_COLS.len(), fw_rows)?;

    drop(db);

    let ts = now_ms().replace([':', 'T', 'Z'], "-");
    let target_dir = state.workspace.exports.join(format!("ducktrack-excel-{ts}"));
    fs::create_dir_all(&target_dir)?;
    let path = target_dir.join("ducktrack-workspace.xlsx");
    wb.save(&path).map_err(xerr)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn import_excel(state: State<'_, AppState>, path: String) -> Result<ExcelImportResult, AppError> {
    let uid = state.active_user()?;
    if !fs::metadata(&path).map_err(|e| AppError::Io(e))?.is_file() {
        return Err(AppError::NotFound(format!("File not found: {path}")));
    }

    let mut sheets: calamine::Sheets<BufReader<fs::File>> =
        calamine::open_workbook_auto(&path).map_err(|e| xerr(e.to_string()))?;
    let names: Vec<String> = sheets.sheet_names().to_vec();
    if !names.iter().any(|n| n == SHEET_PROJECTS) {
        return Err(AppError::Validation(format!("Workbook has no '{SHEET_PROJECTS}' sheet.")));
    }

    let mut projects_created = 0i64;
    let mut projects_updated = 0i64;
    let mut tasks_created = 0i64;
    let mut tasks_updated = 0i64;
    let mut skipped = 0i64;

    let mut db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;
    let tx = db.transaction()?;
    let ts = now();

    // ---- Projects sheet ----
    if let Ok(range) = sheets.worksheet_range(SHEET_PROJECTS) {
        for row in range.rows().skip(1) {
            let key = match non_empty(row.get(0)) {
                Some(k) => k.trim().to_uppercase(),
                None => continue,
            };
            if key.is_empty() { continue; }
            ensure_key_valid(&key)?;
            let name = non_empty(row.get(1)).unwrap_or_else(|| key.clone());
            let status = non_empty(row.get(2)).unwrap_or_else(|| "Active".into());
            let priority = non_empty(row.get(3)).unwrap_or_else(|| "Medium".into());
            let description = non_empty(row.get(4)).unwrap_or_default();

            let upd = tx.execute(
                "UPDATE projects SET name = ?1, status = ?2, priority = ?3,
                        description = ?4, updated_at = ?5
                 WHERE key = ?6 AND user_id = ?7",
                params![name, status, priority, description, ts, key, uid],
            )?;
            if upd > 0 {
                projects_updated += 1;
            } else {
                tx.execute(
                    "INSERT INTO projects (key, name, description, status, priority, user_id, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                    params![key, name, description, status, priority, uid, ts],
                )?;
                let pid = tx.last_insert_rowid();
                indexer::upsert(&tx, "project", pid, uid, &format!("{key} — {name}"), &description)?;
                projects_created += 1;
            }
        }
    }

    // ---- Tasks sheet ----
    if let Ok(range) = sheets.worksheet_range(SHEET_TASKS) {
        let mut body = range.rows().skip(1);
        while let Some(row) = body.next() {
            let key = match non_empty(row.get(0)) { Some(k) => k.trim().to_uppercase(), None => continue };
            let pkey = match non_empty(row.get(1)) { Some(k) => k.trim().to_uppercase(), None => continue };
            if key.is_empty() || pkey.is_empty() { continue; }
            let title = match non_empty(row.get(2)) { Some(t) => t, None => continue };
            let pid: Option<i64> = tx.query_row(
                "SELECT id FROM projects WHERE key = ?1 AND user_id = ?2",
                params![pkey, uid],
                |r| r.get(0),
            ).ok();
            let Some(pid) = pid else { skipped += 1; continue; };

            let ttype = non_empty(row.get(3)).unwrap_or_else(|| "Task".into());
            let status = non_empty(row.get(4)).unwrap_or_else(|| "Todo".into());
            let priority = non_empty(row.get(5)).unwrap_or_else(|| "Medium".into());
            let assignee = non_empty(row.get(6));
            let due = non_empty(row.get(7));
            let est = non_empty(row.get(8)).map(|h| (h.parse::<f64>().unwrap_or(0.0) * 60.0) as i64);

            let existing: i64 = tx.query_row(
                "SELECT COUNT(*) FROM tasks WHERE key = ?1 AND project_id = ?2",
                params![key, pid],
                |r| r.get(0),
            )?;
            if existing > 0 {
                let upd = tx.execute(
                    "UPDATE tasks SET project_id = ?1, title = ?2, type = ?3, status = ?4,
                            priority = ?5, assignee = ?6, due_date = ?7, estimated_minutes = ?8,
                            updated_at = ?9
                     WHERE key = ?10 AND project_id = ?11",
                    params![pid, title, ttype, status, priority, assignee, due, est, ts, key, pid],
                )?;
                if upd > 0 { tasks_updated += 1; }
            } else {
                tx.execute(
                    "INSERT INTO tasks (project_id, key, title, type, status, priority,
                            assignee, due_date, estimated_minutes, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                    params![pid, key, title, ttype, status, priority, assignee, due, est, ts],
                )?;
                let tid = tx.last_insert_rowid();
                indexer::upsert(&tx, "task", tid, uid, &format!("{key} — {title}"), &title)?;
                tasks_created += 1;
            }
        }
    }

    tx.commit()?;
    drop(db);

    Ok(ExcelImportResult {
        path,
        projects_created,
        projects_updated,
        tasks_created,
        tasks_updated,
        skipped_missing_project: skipped,
    })
}