use app_server::AppServer;
use serde_json::{Value, json};
use std::time::Duration;

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

    let events: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workflow.events",
                "params": {
                    "run_id": run_id,
                    "cursor": 0
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(events["jsonrpc"], "2.0");
    assert_eq!(events["id"], 3);
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
fn run_events_can_be_observed_as_a_stream() {
    let server = AppServer::new();

    server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "streaming-review",
                    "nodes": [
                        {
                            "id": "source",
                            "kind": "constant",
                            "value": {
                                "name": "Manual"
                            }
                        },
                        {
                            "id": "message",
                            "kind": "template",
                            "template": "hello {{source.name}}"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "message",
                            "depends_on": "source"
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
                    "workflow_id": "streaming-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start["result"]["run_id"].as_str().unwrap();
    let stream = server.subscribe_run(run_id).unwrap();

    let mut event_types = Vec::new();
    while let Ok(event) = stream.recv_timeout(Duration::from_millis(20)) {
        event_types.push(event["type"].as_str().unwrap().to_owned());

        if event["type"] == "workflow_completed" {
            break;
        }
    }

    assert_eq!(
        event_types,
        [
            "workflow_started",
            "node_started",
            "node_completed",
            "node_started",
            "node_completed",
            "workflow_completed"
        ]
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

    let events: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workflow.events",
                "params": {
                    "run_id": run_id,
                    "cursor": 0
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        events["result"]["events"][4]["result"],
        "next action: clear stale tickets"
    );
}
