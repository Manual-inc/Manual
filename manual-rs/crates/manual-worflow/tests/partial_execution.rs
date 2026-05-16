use std::collections::BTreeMap;
use std::sync::Arc;

use manual_worflow::{
    DependencyDefinition, ExecutionMode, ExecutionOptions, NodeDefinition, NodeKind, RunController,
    WorkflowDefinition, WorkflowRun,
};
use serde_json::{Value, json};

fn make_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "test-wf".to_owned(),
        nodes: vec![
            NodeDefinition { id: "A".to_owned(), kind: NodeKind::Constant, value: json!("a-value"), ..Default::default() },
            NodeDefinition { id: "B".to_owned(), kind: NodeKind::Constant, value: json!("b-value"), ..Default::default() },
            NodeDefinition { id: "C".to_owned(), kind: NodeKind::Constant, value: json!("c-value"), ..Default::default() },
        ],
        dependencies: vec![
            DependencyDefinition { node: "B".to_owned(), depends_on: "A".to_owned() },
            DependencyDefinition { node: "C".to_owned(), depends_on: "B".to_owned() },
        ],
    }
}

fn make_fail_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "fail-wf".to_owned(),
        nodes: vec![
            NodeDefinition { id: "A".to_owned(), kind: NodeKind::Constant, value: json!("a-value"), ..Default::default() },
            NodeDefinition { id: "B".to_owned(), kind: NodeKind::Fail, error: "intentional failure".to_owned(), ..Default::default() },
            NodeDefinition { id: "C".to_owned(), kind: NodeKind::Constant, value: json!("c-value"), ..Default::default() },
        ],
        dependencies: vec![
            DependencyDefinition { node: "B".to_owned(), depends_on: "A".to_owned() },
            DependencyDefinition { node: "C".to_owned(), depends_on: "B".to_owned() },
        ],
    }
}

fn collect_events(wf: &WorkflowDefinition, opts: ExecutionOptions, controller: Option<Arc<RunController>>) -> Vec<Value> {
    let mut events = Vec::new();
    let _ = wf.execute_with_options("run-1", opts, controller, |e| events.push(e));
    events
}

fn node_ids_of_type<'a>(events: &'a [Value], event_type: &str) -> Vec<&'a str> {
    events.iter()
        .filter(|e| e["type"] == event_type)
        .filter_map(|e| e["node_id"].as_str())
        .collect()
}

#[test]
fn start_node_id_skips_upstream_nodes() {
    let wf = make_workflow();

    let mut prev_run = WorkflowRun::pending();
    prev_run.record_event(json!({"type": "node_completed", "node_id": "A", "result": "a-value", "sequence": 0}));

    let opts = ExecutionOptions {
        start_node_id: Some("B".to_owned()),
        previous_run: Some(prev_run),
        ..Default::default()
    };

    let events = collect_events(&wf, opts, None);
    let skipped = node_ids_of_type(&events, "node_skipped");
    let completed = node_ids_of_type(&events, "node_completed");

    assert!(skipped.contains(&"A"), "A는 skip 되어야 함: {skipped:?}");
    assert!(completed.contains(&"B"), "B는 실행되어야 함: {completed:?}");
    assert!(completed.contains(&"C"), "C는 실행되어야 함: {completed:?}");
}

#[test]
fn resume_from_failure_skips_completed_nodes() {
    let wf = make_fail_workflow();

    let mut prev_run = WorkflowRun::pending();
    prev_run.record_event(json!({"type": "node_completed", "node_id": "A", "result": "a-value", "sequence": 0}));
    prev_run.record_event(json!({"type": "node_failed", "node_id": "B", "error": "intentional failure", "sequence": 1}));
    prev_run.record_event(json!({"type": "workflow_failed", "workflow_id": "fail-wf", "sequence": 2}));

    assert_eq!(prev_run.first_failed_node(), Some("B".to_owned()));
    assert!(prev_run.resumable());

    let opts = ExecutionOptions {
        resume_from_failure: true,
        previous_run: Some(prev_run),
        ..Default::default()
    };

    let events = collect_events(&wf, opts, None);
    let skipped = node_ids_of_type(&events, "node_skipped");
    assert!(skipped.contains(&"A"), "A는 이미 완료됐으므로 skip: {skipped:?}");
    assert!(!skipped.contains(&"B"), "B는 재실행 대상");
}

#[test]
fn cancel_emits_workflow_cancelled() {
    let wf = make_workflow();
    let ctrl = Arc::new(RunController::new(&ExecutionMode::Auto));
    ctrl.cancel();

    let events = collect_events(&wf, ExecutionOptions::default(), Some(Arc::clone(&ctrl)));
    let types: Vec<_> = events.iter().filter_map(|e| e["type"].as_str()).collect();
    assert!(types.contains(&"workflow_cancelled"), "취소 이벤트 필요: {types:?}");
    assert!(!types.contains(&"workflow_completed"), "완료 이벤트는 없어야 함");
}

#[test]
fn step_mode_pauses_between_stages() {
    let wf = make_workflow();
    let ctrl = Arc::new(RunController::new(&ExecutionMode::Step));
    let ctrl_clone = Arc::clone(&ctrl);

    let events_store = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let events_store_clone = Arc::clone(&events_store);

    let wf_clone = wf.clone();
    let handle = std::thread::spawn(move || {
        let _ = wf_clone.execute_with_options(
            "run-step",
            ExecutionOptions { mode: ExecutionMode::Step, ..Default::default() },
            Some(ctrl_clone),
            |e| events_store_clone.lock().expect("lock").push(e),
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(200));
    {
        let events = events_store.lock().expect("lock");
        let types: Vec<_> = events.iter().filter_map(|e| e["type"].as_str()).collect();
        assert!(types.contains(&"workflow_paused"), "첫 pause 필요: {types:?}");
    }

    // A 스테이지 진행
    ctrl.request_step();
    std::thread::sleep(std::time::Duration::from_millis(200));

    // B 스테이지 진행
    ctrl.request_step();
    std::thread::sleep(std::time::Duration::from_millis(200));

    // C 스테이지 진행
    ctrl.request_step();
    std::thread::sleep(std::time::Duration::from_millis(300));

    handle.join().expect("실행 스레드 join 실패");

    let events = events_store.lock().expect("lock");
    let types: Vec<_> = events.iter().filter_map(|e| e["type"].as_str()).collect();
    assert!(types.contains(&"workflow_completed"), "최종 완료: {types:?}");
}

#[test]
fn workflow_run_helpers() {
    let mut run = WorkflowRun::pending();
    run.record_event(json!({"type": "node_completed", "node_id": "A", "result": "val-a"}));
    run.record_event(json!({"type": "node_failed", "node_id": "B", "error": "oops"}));
    run.record_event(json!({"type": "workflow_failed", "workflow_id": "wf"}));

    let completed = run.completed_nodes();
    assert_eq!(completed.get("A"), Some(&json!("val-a")));
    assert_eq!(run.first_failed_node(), Some("B".to_owned()));
    assert!(run.resumable());
}
