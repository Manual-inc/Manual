use app_server::AppServer;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static STORAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn optimization_endpoints_reflect_persisted_run_history() {
    let server = test_server("optimization-data-driven");

    rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "optimization.record_run",
            "params": {
                "run_id": "before",
                "workflow_id": "wf-main",
                "status": "completed",
                "token_usage": {
                    "total": 1800,
                    "by_step": [
                        { "step_id": "plan", "tokens": 1800, "budget": 2500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 }
                    ],
                    "by_model": [
                        { "model": "gpt-5.4-mini", "tokens": 1800, "cost": 0.04 }
                    ],
                    "hotspots": ["plan"]
                },
                "verification": {
                    "pass_rate": 0.94,
                    "requirements_satisfied": 0.94,
                    "items": [],
                    "missing": [],
                    "risks": []
                },
                "time": {
                    "total_ms": 700,
                    "by_step": [{ "step_id": "plan", "duration_ms": 700, "retries": 0 }],
                    "review_ms": 0
                }
            }
        }),
    );
    rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "optimization.record_run",
            "params": {
                "run_id": "after",
                "workflow_id": "wf-main",
                "status": "completed",
                "token_usage": {
                    "total": 6200,
                    "by_step": [
                        { "step_id": "plan", "tokens": 1000, "budget": 1500, "over_budget": false, "over_by": 0, "over_ratio": 0.0 },
                        { "step_id": "implement", "tokens": 5200, "budget": 3200, "over_budget": true, "over_by": 2000, "over_ratio": 0.625 }
                    ],
                    "by_model": [
                        { "model": "gpt-5.5", "tokens": 5200, "cost": 0.52 }
                    ],
                    "hotspots": ["implement"]
                },
                "verification": {
                    "pass_rate": 0.7,
                    "requirements_satisfied": 0.78,
                    "items": [
                        { "name": "review", "status": "unknown", "evidence": [] }
                    ],
                    "missing": ["review"],
                    "risks": ["review evidence missing"]
                },
                "time": {
                    "total_ms": 3100,
                    "by_step": [{ "step_id": "implement", "duration_ms": 3100, "retries": 2 }],
                    "review_ms": 400
                }
            }
        }),
    );

    let report = rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "optimization.report",
            "params": { "workflow_id": "wf-main" }
        }),
    );
    let analysis = rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "optimization.analyze",
            "params": { "workflow_id": "wf-main" }
        }),
    );
    let comparison = rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "optimization.compare",
            "params": {
                "workflow_id": "wf-main",
                "before_run_id": "before",
                "after_run_id": "after"
            }
        }),
    );

    assert_eq!(report["result"]["main_issue"], "implementation step used most tokens");
    assert_eq!(analysis["result"]["regression"]["possible"], true);
    assert_eq!(analysis["result"]["measurement_mode"], "reported");
    assert_eq!(comparison["result"]["measurement_mode"], "reported");
    assert_eq!(comparison["result"]["token_delta"], 4400);
}

#[test]
fn completed_workflow_automatically_records_optimization_evidence() {
    let server = test_server("workflow-auto-optimization");

    rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "wf-auto-opt",
                    "nodes": [
                        {
                            "id": "context",
                            "kind": "constant",
                            "value": { "message": "hello" }
                        },
                        {
                            "id": "digest",
                            "kind": "template",
                            "template": "digest: {{context.message}}"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "digest",
                            "depends_on": "context"
                        }
                    ]
                }
            }
        }),
    );

    let started = rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "workflow.start",
            "params": {
                "workflow_id": "wf-auto-opt"
            }
        }),
    );
    let run_id = started["result"]["run_id"]
        .as_str()
        .expect("workflow should return run_id")
        .to_owned();

    let _events = poll_events_until(&server, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });
    let completed_events = rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "workflow.events",
            "params": { "run_id": run_id, "cursor": 0 }
        }),
    );

    let analysis = rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "optimization.analyze",
            "params": { "workflow_id": "wf-auto-opt" }
        }),
    );
    let report = rpc(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "optimization.report",
            "params": { "workflow_id": "wf-auto-opt" }
        }),
    );

    assert!(
        analysis["result"]["candidates"]
            .as_array()
            .is_some_and(|candidates| !candidates.is_empty())
    );
    assert_eq!(analysis["result"]["measurement_mode"], "derived");
    assert!(
        analysis["result"]["measurement_note"]
            .as_str()
            .is_some_and(|note| note.contains("Estimated"))
    );
    assert_eq!(completed_events["result"]["optimization_report"]["measurement_mode"], "derived");
    assert!(
        completed_events["result"]["optimization_analysis"]["candidates"]
            .as_array()
            .is_some_and(|candidates| !candidates.is_empty())
    );
    assert_eq!(report["result"]["measurement_mode"], "derived");
    assert!(
        report["result"]["measurement_note"]
            .as_str()
            .is_some_and(|note| note.contains("Estimated"))
    );
    assert_ne!(
        report["result"]["main_issue"],
        Value::String("insufficient run history to identify a main issue".to_owned())
    );
}

fn rpc(server: &AppServer, payload: Value) -> Value {
    serde_json::from_str(&server.handle_json(&payload.to_string())).unwrap()
}

fn poll_events_until(
    server: &AppServer,
    run_id: &str,
    cursor: usize,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(2);

    loop {
        let events = rpc(
            server,
            json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "workflow.events",
                "params": {
                    "run_id": run_id,
                    "cursor": cursor
                }
            }),
        );

        if predicate(&events) {
            return events;
        }

        assert!(Instant::now() < deadline, "timed out waiting for workflow events");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn test_server(name: &str) -> AppServer {
    AppServer::with_storage_dir(unique_storage_dir(name))
}

fn unique_storage_dir(name: &str) -> PathBuf {
    let counter = STORAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let unique = format!(
        "{name}-{}-{:?}-{counter}",
        std::process::id(),
        std::thread::current().id()
    );
    std::env::temp_dir().join("manual-rs-tests").join(unique)
}
