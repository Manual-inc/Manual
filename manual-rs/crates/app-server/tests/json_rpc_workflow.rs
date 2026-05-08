use app_server::AppServer;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

#[test]
fn json_rpc_create_start_and_stream_workflow_events() {
    let server = AppServer::new();

    let create = server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "lead-review",
                    "nodes": [
                        {
                            "id": "lead_payload",
                            "kind": "constant",
                            "value": {
                                "lead_count": 128,
                                "qualified_count": 42
                            }
                        },
                        {
                            "id": "score",
                            "kind": "template",
                            "template": "qualified leads: {{lead_payload.qualified_count}} / {{lead_payload.lead_count}}"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "score",
                            "depends_on": "lead_payload"
                        }
                    ]
                }
            }
        })
        .to_string(),
    );

    assert_eq!(
        create,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "workflow_id": "lead-review",
                "node_count": 2
            }
        })
        .to_string()
    );

    let start: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.start",
                "params": {
                    "workflow_id": "lead-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start["result"]["run_id"].as_str().unwrap();

    let events = poll_events_until(&server, run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap()
    });

    assert_eq!(events["jsonrpc"], "2.0");
    assert_eq!(events["result"]["completed"], true);
    assert_eq!(events["result"]["next_cursor"], 6);
    assert_eq!(
        events["result"]["events"],
        json!([
            {
                "run_id": run_id,
                "sequence": 0,
                "type": "workflow_started",
                "workflow_id": "lead-review"
            },
            {
                "run_id": run_id,
                "sequence": 1,
                "type": "node_started",
                "node_id": "lead_payload"
            },
            {
                "run_id": run_id,
                "sequence": 2,
                "type": "node_completed",
                "node_id": "lead_payload",
                "result": {
                    "lead_count": 128,
                    "qualified_count": 42
                }
            },
            {
                "run_id": run_id,
                "sequence": 3,
                "type": "node_started",
                "node_id": "score"
            },
            {
                "run_id": run_id,
                "sequence": 4,
                "type": "node_completed",
                "node_id": "score",
                "result": "qualified leads: 42 / 128"
            },
            {
                "run_id": run_id,
                "sequence": 5,
                "type": "workflow_completed",
                "workflow_id": "lead-review"
            }
        ])
    );
}

#[test]
fn workflow_start_runs_in_background_and_events_can_poll_progress() {
    let server = AppServer::new();

    server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "background-review",
                    "nodes": [
                        {
                            "id": "pause",
                            "kind": "delay",
                            "duration_ms": 100
                        },
                        {
                            "id": "message",
                            "kind": "template",
                            "template": "done"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "message",
                            "depends_on": "pause"
                        }
                    ]
                }
            }
        })
        .to_string(),
    );

    let started_at = Instant::now();
    let start: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.start",
                "params": {
                    "workflow_id": "background-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert!(started_at.elapsed() < Duration::from_millis(50));

    let run_id = start["result"]["run_id"].as_str().unwrap();
    let in_progress = poll_events_until(&server, run_id, 0, |events| {
        !events["result"]["completed"].as_bool().unwrap()
            && events["result"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["type"] == "node_started" && event["node_id"] == "pause")
    });

    assert_eq!(in_progress["result"]["completed"], false);
    let next_cursor = in_progress["result"]["next_cursor"].as_u64().unwrap() as usize;

    let completed = poll_events_until(&server, run_id, next_cursor, |events| {
        events["result"]["completed"].as_bool().unwrap()
    });

    assert_eq!(completed["result"]["completed"], true);
    assert!(
        completed["result"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["type"] == "workflow_completed")
    );
}

#[test]
fn workflow_events_reports_background_execution_failure() {
    let server = AppServer::new();

    server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "failing-review",
                    "nodes": [
                        {
                            "id": "explode",
                            "kind": "fail",
                            "error": "boom"
                        }
                    ]
                }
            }
        })
        .to_string(),
    );

    let start: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.start",
                "params": {
                    "workflow_id": "failing-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start["result"]["run_id"].as_str().unwrap();

    let events = poll_events_until(&server, run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap()
    });

    assert_eq!(events["result"]["completed"], true);
    assert_eq!(
        events["result"]["events"],
        json!([
            {
                "run_id": run_id,
                "sequence": 0,
                "type": "workflow_started",
                "workflow_id": "failing-review"
            },
            {
                "run_id": run_id,
                "sequence": 1,
                "type": "node_started",
                "node_id": "explode"
            },
            {
                "run_id": run_id,
                "sequence": 2,
                "type": "node_failed",
                "node_id": "explode",
                "error": "boom"
            },
            {
                "run_id": run_id,
                "sequence": 3,
                "type": "workflow_failed",
                "workflow_id": "failing-review",
                "error": "boom"
            }
        ])
    );
}

#[test]
fn template_nodes_can_reference_scalar_upstream_results() {
    let server = AppServer::new();

    server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "scalar-template",
                    "nodes": [
                        {
                            "id": "recommendation",
                            "kind": "template",
                            "template": "clear stale tickets"
                        },
                        {
                            "id": "digest",
                            "kind": "template",
                            "template": "next action: {{recommendation}}"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "digest",
                            "depends_on": "recommendation"
                        }
                    ]
                }
            }
        })
        .to_string(),
    );

    let start: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.start",
                "params": {
                    "workflow_id": "scalar-template"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start["result"]["run_id"].as_str().unwrap();

    let events = poll_events_until(&server, run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap()
    });

    assert_eq!(
        events["result"]["events"][4]["result"],
        "next action: clear stale tickets"
    );
}

fn poll_events_until(
    server: &AppServer,
    run_id: &str,
    cursor: usize,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = Instant::now() + Duration::from_secs(1);

    loop {
        let events: Value = serde_json::from_str(
            &server.handle_json(
                &json!({
                    "jsonrpc": "2.0",
                    "id": 99,
                    "method": "workflow.events",
                    "params": {
                        "run_id": run_id,
                        "cursor": cursor
                    }
                })
                .to_string(),
            ),
        )
        .unwrap();

        if predicate(&events) {
            return events;
        }

        assert!(Instant::now() < deadline, "timed out waiting for events");
        std::thread::sleep(Duration::from_millis(5));
    }
}
