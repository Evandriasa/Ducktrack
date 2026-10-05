use ducktrack_lib::models::*;

#[test]
fn frontend_task_input_payload() {
    let js = r#"{"project_id":1,"title":"Fix pump","description":"desc","priority":"High","task_type":"Bug","status":"Todo","due_date":"2026-10-01","assignee":"Local User"}"#;
    let t: TaskInput = serde_json::from_str(js).expect("TaskInput camelCase mismatch");
    assert_eq!(t.project_id, 1);
    assert_eq!(t.task_type.as_deref(), Some("Bug"));
}

#[test]
fn frontend_task_filter_payload() {
    let js = r#"{"status":"In Progress","include_subtasks":false}"#;
    let f: TaskFilter = serde_json::from_str(js).expect("TaskFilter camelCase mismatch");
    assert_eq!(f.status.as_deref(), Some("In Progress"));
}

#[test]
fn frontend_history_filter_payload() {
    let js = r#"{"limit":10}"#;
    let f: HistoryFilter = serde_json::from_str(js).expect("HistoryFilter mismatch");
    assert_eq!(f.limit, Some(10));
}

#[test]
fn frontend_work_log_input_payload() {
    let js = r#"{"description":"did stuff","duration_minutes":45,"project_id":2,"task_id":3}"#;
    let w: WorkLogInput = serde_json::from_str(js).expect("WorkLogInput mismatch");
    assert_eq!(w.duration_minutes, Some(45));
    assert_eq!(w.task_id, Some(3));
}

#[test]
fn frontend_document_input_payload() {
    let js = r#"{"project_id":1,"title":"T","content":"c"}"#;
    let d: DocumentInput = serde_json::from_str(js).expect("DocumentInput mismatch");
    assert_eq!(d.project_id, 1);
}

#[test]
fn frontend_update_payloads() {
    let js = r#"{"status":"Completed","priority":"High","task_type":"Bug","project_id":1,"assignee":"x","due_date":"2026-10-01"}"#;
    let u: TaskUpdate = serde_json::from_str(js).expect("TaskUpdate mismatch");
    assert_eq!(u.status.as_deref(), Some("Completed"));

    let pj = r#"{"name":"n","description":"d","priority":"High","status":"Active","parent_id":null}"#;
    let p: ProjectUpdate = serde_json::from_str(pj).expect("ProjectUpdate mismatch");
    assert_eq!(p.name.as_deref(), Some("n"));
}

#[test]
fn frontend_project_input_payload() {
    let js = r#"{"key":"AGV","name":"Robot","description":"d","parent_id":null}"#;
    let p: ProjectInput = serde_json::from_str(js).expect("ProjectInput mismatch");
    assert_eq!(p.key, "AGV");
}

#[test]
fn response_is_camelcase() {
    let stats = DashboardStats::default();
    let json = serde_json::to_string(&stats).expect("serialize DashboardStats");
    for key in ["openTasks", "inProgress", "completedTasks", "blocked", "workLogCount"] {
        assert!(json.contains(key), "missing camelCase key {key} in {json}");
    }
}