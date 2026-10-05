use rusqlite::Connection;
use std::path::Path;
use std::time::{Duration, Instant};

fn probe() {
    let db_path = Path::new("/home/jacinto/.local/share/com.ducktrack.workspace/workspace/ducktrack.db");
    let start = Instant::now();
    let db = Connection::open(db_path).expect("open db");
    println!("open ok in {:?}", start.elapsed());

    let one = |sql: &str| -> i64 { db.query_row(sql, [], |r| r.get(0)).expect(sql) };
    println!("projects: {}", one("SELECT COUNT(*) FROM projects"));
    println!("tasks:    {}", one("SELECT COUNT(*) FROM tasks"));
    println!("history:  {}", one("SELECT COUNT(*) FROM task_history"));

    let t0 = Instant::now();
    let n: i64 = db
        .query_row("SELECT COUNT(*) FROM (SELECT id, parent_id, key, name, description, status, priority, created_at, updated_at, completed_at FROM projects)", [], |r| r.get(0))
        .unwrap();
    println!("list_projects sql: {} rows in {:?}", n, t0.elapsed());

    let t1 = Instant::now();
    let mut stmt = db
        .prepare("SELECT id, parent_id, key, name, description, status, priority, created_at, updated_at, completed_at FROM projects ORDER BY key")
        .unwrap();
    let projs: Vec<(i64, Option<i64>, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut cycle = String::from("none");
    for (id, maybe_pid, _) in &projs {
        if let Some(pid) = maybe_pid {
            if !projs.iter().any(|(i, _, _)| i == pid) {
                cycle = format!("dangling parent {pid} on {id}");
            }
        }
    }
    println!("tree source: {} projects in {:?}, cycle={}", projs.len(), t1.elapsed(), cycle);

    let t3 = Instant::now();
    let n: i64 = db.query_row(
        "SELECT COUNT(*) FROM tasks t JOIN projects p ON p.id = t.project_id WHERE 1=1 AND t.status = 'In Progress' AND t.parent_task_id IS NULL",
        [],
        |r| r.get(0),
    ).unwrap();
    println!("list_tasks(filter) sql: {} rows in {:?}", n, t3.elapsed());

    let t4 = Instant::now();
    let n: i64 = db.query_row("SELECT COUNT(*) FROM tasks WHERE status IN ('Todo','In Progress','Blocked')", [], |r| r.get(0)).unwrap();
    println!("dashboard_stats: {} open in {:?}", n, t4.elapsed());

    let t5 = Instant::now();
    let n: i64 = db
        .query_row("SELECT COUNT(*) FROM task_history h", [], |r| r.get(0))
        .unwrap();
    println!("global history source: {} in {:?}", n, t5.elapsed());
}

#[test]
fn probe_db() {
    println!("probe start");
    std::thread::scope(|s| {
        let handle = s.spawn(probe);
        let deadline = Instant::now() + Duration::from_secs(10);
        while handle.is_finished() == false && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        if !handle.is_finished() {
            println!("PROBE HUNG");
            std::process::exit(1);
        }
    });
    println!("probe done");
}