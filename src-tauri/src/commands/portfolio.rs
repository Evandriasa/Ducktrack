use std::fs;

use tauri::State;

use super::super::database::{now_ms, AppState};
use super::super::error::AppError;

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioExport {
    pub markdown_path: String,
    pub html_path: String,
}

struct PtTask {
    key: String,
    title: String,
    status: String,
    priority: String,
    assignee: Option<String>,
    due: Option<String>,
}

struct PtProject {
    key: String,
    name: String,
    description: String,
    status: String,
    priority: String,
    tasks: Vec<PtTask>,
    open_tasks: i64,
    completed_tasks: i64,
    minutes: i64,
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[tauri::command(rename_all = "snake_case")]
pub fn export_portfolio(state: State<'_, AppState>) -> Result<PortfolioExport, AppError> {
    let uid = state.active_user()?;
    let db = state.db.lock().map_err(|_| AppError::Other("db lock poisoned".into()))?;

    let name: String = db.query_row("SELECT name FROM users WHERE id = ?1", [uid], |r| r.get(0))?;
    let ts = now_ms().replace([':', 'T', 'Z'], "-");

    let mut projects: Vec<PtProject> = Vec::new();
    let mut total_minutes = 0i64;

    {
        let mut proj_stmt = db.prepare(
            "SELECT id, key, name, description, status, priority FROM projects WHERE user_id = ?1 ORDER BY name COLLATE NOCASE",
        )?;
        let project_rows: Vec<(i64, String, String, String, String, String)> = proj_stmt
            .query_map([uid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        for (pid, pkey, pname, pdesc, pstatus, ppriority) in project_rows {
            let (open_tasks, completed_tasks): (i64, i64) = db.query_row(
                "SELECT SUM(CASE WHEN status NOT IN ('Completed','Cancelled') THEN 1 ELSE 0 END),
                        SUM(CASE WHEN status = 'Completed' THEN 1 ELSE 0 END)
                 FROM tasks WHERE project_id = ?1",
                [pid],
                |r| Ok((r.get::<_, Option<i64>>(0)?.unwrap_or(0), r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
            )?;
            let minutes: i64 = db.query_row(
                "SELECT COALESCE(SUM(duration_minutes), 0) FROM work_logs WHERE project_id = ?1",
                [pid],
                |r| r.get(0),
            )?;
            total_minutes += minutes;

            let mut task_stmt = db.prepare(
                "SELECT key, title, status, priority, assignee, due_date FROM tasks WHERE project_id = ?1 ORDER BY created_at",
            )?;
            let tasks: Vec<PtTask> = task_stmt
                .query_map([pid], |r| {
                    Ok(PtTask {
                        key: r.get(0)?,
                        title: r.get(1)?,
                        status: r.get(2)?,
                        priority: r.get(3)?,
                        assignee: r.get(4)?,
                        due: r.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            projects.push(PtProject {
                key: pkey,
                name: pname,
                description: pdesc,
                status: pstatus,
                priority: ppriority,
                tasks,
                open_tasks,
                completed_tasks,
                minutes,
            });
        }
    }

    let total_tasks: i64 = db.query_row(
        "SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE p.user_id = ?1",
        [uid],
        |r| r.get(0),
    )?;
    let completed_total: i64 = db.query_row(
        "SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE t.status = 'Completed' AND p.user_id = ?1",
        [uid],
        |r| r.get(0),
    )?;

    drop(db);

    let target_dir = state.workspace.exports.join(format!("ducktrack-portfolio-{ts}"));
    fs::create_dir_all(&target_dir)?;
    let md_path = target_dir.join("portfolio.md");
    let html_path = target_dir.join("portfolio.html");

    let md = build_markdown(&name, &projects, total_tasks, completed_total, total_minutes);
    let html = build_html(&name, &projects, total_tasks, completed_total, total_minutes);
    fs::write(&md_path, md)?;
    fs::write(&html_path, html)?;

    Ok(PortfolioExport {
        markdown_path: md_path.to_string_lossy().into_owned(),
        html_path: html_path.to_string_lossy().into_owned(),
    })
}

fn build_markdown(
    name: &str,
    projects: &[PtProject],
    total_tasks: i64,
    completed_total: i64,
    total_minutes: i64,
) -> String {
    let total_hours = total_minutes as f64 / 60.0;
    let mut out = String::new();
    out.push_str(&format!("# Work Portfolio — {name}\n\n"));
    out.push_str(&format!(
        "- **Projects:** {}\n- **Tasks:** {total_tasks} ({completed_total} completed, {:.0}%)\n- **Hours logged:** {total_hours:.1}h\n\n",
        projects.len(),
        if total_tasks == 0 { 0.0 } else { completed_total as f64 * 100.0 / total_tasks as f64 }
    ));

    for p in projects {
        out.push_str(&format!(
            "\n## {} ({})\n", p.name, p.key
        ));
        if !p.description.trim().is_empty() {
            out.push_str(&format!("{}\n", p.description.trim()));
        }
        out.push_str(&format!(
            "\n_{} open / {} completed · {}h logged · status: {}_\n\n",
            p.open_tasks,
            p.completed_tasks,
            p.minutes as f64 / 60.0,
            p.status
        ));
        if p.tasks.is_empty() {
            out.push_str("_No tasks._\n");
        } else {
            out.push_str("| Key | Title | Status | Priority | Assignee | Due |\n");
            out.push_str("| --- | --- | --- | --- | --- | --- |\n");
            for t in &p.tasks {
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    t.key,
                    t.title.replace('|', "\\|"),
                    t.status,
                    t.priority,
                    t.assignee.clone().unwrap_or_default(),
                    t.due.clone().unwrap_or_default()
                ));
            }
        }
    }
    out
}

fn build_html(
    name: &str,
    projects: &[PtProject],
    total_tasks: i64,
    completed_total: i64,
    total_minutes: i64,
) -> String {
    let total_hours = total_minutes as f64 / 60.0;
    let pct = if total_tasks == 0 {
        0.0
    } else {
        completed_total as f64 * 100.0 / total_tasks as f64
    };
    let name_esc = esc_html(name);

    let mut cards = String::new();
    for p in projects {
        let mut rows = String::new();
        for t in &p.tasks {
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                esc_html(&t.key),
                esc_html(&t.title),
                esc_html(&t.status),
                esc_html(&t.priority),
                esc_html(t.assignee.as_deref().unwrap_or("")),
                esc_html(t.due.as_deref().unwrap_or(""))
            ));
        }
        if rows.is_empty() {
            rows = "<tr><td colspan=\"6\"><em>No tasks.</em></td></tr>".to_string();
        }
        cards.push_str(&format!(
            "<section class=\"card\"><h2>{}</h2><p class=\"meta\">Key: {} · Status: {} · Priority: {}</p>\
             <p>{}</p><p class=\"meta\">{} open / {} completed · {:.1}h logged</p>\
             <table><thead><tr><th>Key</th><th>Title</th><th>Status</th><th>Priority</th><th>Assignee</th><th>Due</th></tr></thead>\
             <tbody>{}</tbody></table></section>",
            esc_html(&p.name),
            esc_html(&p.key),
            esc_html(&p.status),
            esc_html(&p.priority),
            esc_html(&p.description),
            p.open_tasks,
            p.completed_tasks,
            p.minutes as f64 / 60.0,
            rows
        ));
    }

    format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <title>Work Portfolio — {name_esc}</title><style>\
         body{{font-family:-apple-system,'Segoe UI',Roboto,sans-serif;max-width:980px;margin:2rem auto;padding:0 1rem;color:#1c2733;background:#fff}}\
         h1{{border-bottom:3px solid #2f6fd0;padding-bottom:.4rem}}h2{{margin-top:1.8rem}}\
         .card{{border:1px solid #e2e8f0;border-radius:.5rem;padding:1rem 1.2rem;margin:1rem 0}}\
         .meta{{color:#5a6b7b;font-size:.9rem}}table{{border-collapse:collapse;width:100%;font-size:.92rem}}\
         th,td{{border:1px solid #e2e8f0;padding:.4rem .55rem;text-align:left}}th{{background:#f1f6fc}}\
         </style></head><body>\
         <h1>Work Portfolio — {name_esc}</h1>\
         <p class=\"meta\">Projects: {project_count} · Tasks: {total_tasks} ({completed_total} completed, {pct:.0}%) · Hours logged: {total_hours:.1}h</p>\
         {cards}</body></html>",
        project_count = projects.len()
    )
}