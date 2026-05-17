use app_server::AppServer;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static STORAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn json_rpc_create_start_and_stream_workflow_events() {
    let server = test_server();

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
    let server = test_server();

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
    let server = test_server();

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
    let server = test_server();

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

#[test]
fn json_rpc_can_read_list_update_and_delete_workflows() {
    let server = test_server();

    server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "crud-review",
                    "nodes": [
                        {
                            "id": "message",
                            "kind": "template",
                            "template": "first draft"
                        }
                    ]
                }
            }
        })
        .to_string(),
    );

    let get: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "crud-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(get["result"]["workflow"]["id"], "crud-review");
    assert_eq!(
        get["result"]["workflow"]["nodes"][0]["template"],
        "first draft"
    );

    let list: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workflow.list"
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        list["result"]["workflows"],
        json!([
            {
                "workflow_id": "crud-review",
                "node_count": 1
            }
        ])
    );

    let update: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "workflow.update",
                "params": {
                    "workflow_id": "crud-review",
                    "workflow": {
                        "id": "crud-review",
                        "nodes": [
                            {
                                "id": "message",
                                "kind": "template",
                                "template": "final draft"
                            },
                            {
                                "id": "done",
                                "kind": "template",
                                "template": "published"
                            }
                        ],
                        "dependencies": [
                            {
                                "node": "done",
                                "depends_on": "message"
                            }
                        ]
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        update["result"],
        json!({
            "workflow_id": "crud-review",
            "node_count": 2
        })
    );

    let updated: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "crud-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        updated["result"]["workflow"]["nodes"][0]["template"],
        "final draft"
    );
    assert_eq!(
        updated["result"]["workflow"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let delete: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "workflow.delete",
                "params": {
                    "workflow_id": "crud-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        delete["result"],
        json!({
            "workflow_id": "crud-review",
            "deleted": true
        })
    );

    let missing_after_delete: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "crud-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(missing_after_delete["error"]["code"], -32000);
    assert_eq!(
        missing_after_delete["error"]["message"],
        "workflow not found"
    );
}

#[test]
fn json_rpc_can_patch_workflow_nodes_and_dependencies() {
    let storage_dir = unique_storage_dir("patch");
    let server = AppServer::with_storage_dir(&storage_dir);

    server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "patch-review",
                    "nodes": [
                        {
                            "id": "source",
                            "kind": "constant",
                            "value": {
                                "count": 3
                            }
                        },
                        {
                            "id": "summary",
                            "kind": "template",
                            "template": "count: {{source.count}}"
                        }
                    ],
                    "dependencies": [
                        {
                            "node": "summary",
                            "depends_on": "source"
                        }
                    ]
                }
            }
        })
        .to_string(),
    );

    let patch: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.patch",
                "params": {
                    "workflow_id": "patch-review",
                    "operations": [
                        {
                            "op": "update_node",
                            "node": {
                                "id": "summary",
                                "kind": "template",
                                "template": "total: {{source.count}}"
                            }
                        },
                        {
                            "op": "add_node",
                            "node": {
                                "id": "publish",
                                "kind": "template",
                                "template": "{{summary}} ready"
                            }
                        },
                        {
                            "op": "add_dependency",
                            "dependency": {
                                "node": "publish",
                                "depends_on": "summary"
                            }
                        },
                        {
                            "op": "update_dependency",
                            "node": "summary",
                            "depends_on": "source",
                            "dependency": {
                                "node": "publish",
                                "depends_on": "source"
                            }
                        },
                        {
                            "op": "delete_node",
                            "node_id": "summary"
                        }
                    ]
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        patch["result"],
        json!({
            "workflow_id": "patch-review",
            "node_count": 2,
            "dependency_count": 1
        })
    );

    let restarted_server = AppServer::with_storage_dir(&storage_dir);
    let updated: Value = serde_json::from_str(
        &restarted_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "patch-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        updated["result"]["workflow"],
        json!({
            "id": "patch-review",
            "nodes": [
                {
                    "id": "source",
                    "kind": "constant",
                    "value": {
                        "count": 3
                    },
                    "template": "",
                    "duration_ms": 0,
                    "error": "",
                    "prompt": "",
                    "model": null
                },
                {
                    "id": "publish",
                    "kind": "template",
                    "value": null,
                    "template": "{{summary}} ready",
                    "duration_ms": 0,
                    "error": "",
                    "prompt": "",
                    "model": null
                }
            ],
            "dependencies": [
                {
                    "node": "publish",
                    "depends_on": "source"
                }
            ]
        })
    );
}

#[test]
fn json_rpc_patch_rejects_invalid_workflow_changes() {
    let server = test_server();

    server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "invalid-patch",
                    "nodes": [
                        {
                            "id": "message",
                            "kind": "template",
                            "template": "hello"
                        }
                    ]
                }
            }
        })
        .to_string(),
    );

    let patch: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.patch",
                "params": {
                    "workflow_id": "invalid-patch",
                    "operations": [
                        {
                            "op": "add_dependency",
                            "dependency": {
                                "node": "message",
                                "depends_on": "missing"
                            }
                        }
                    ]
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(patch["error"]["code"], -32602);
    assert!(
        patch["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unknown node")
    );

    let unchanged: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "invalid-patch"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(unchanged["result"]["workflow"]["dependencies"], json!([]));
}

#[test]
fn workflows_are_loaded_from_storage_after_server_restart() {
    let storage_dir = unique_storage_dir("restart");
    let first_server = AppServer::with_storage_dir(&storage_dir);

    let create: Value = serde_json::from_str(
        &first_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "workflow.create",
                "params": {
                    "workflow": {
                        "id": "durable-review",
                        "nodes": [
                            {
                                "id": "message",
                                "kind": "template",
                                "template": "survives restart"
                            }
                        ]
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(create["result"]["workflow_id"], "durable-review");

    let restarted_server = AppServer::with_storage_dir(&storage_dir);
    let get: Value = serde_json::from_str(
        &restarted_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "durable-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(get["result"]["workflow"]["id"], "durable-review");
    assert_eq!(
        get["result"]["workflow"]["nodes"][0]["template"],
        "survives restart"
    );

    let list: Value = serde_json::from_str(
        &restarted_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "workflow.list"
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(
        list["result"]["workflows"],
        json!([
            {
                "workflow_id": "durable-review",
                "node_count": 1
            }
        ])
    );

    let update: Value = serde_json::from_str(
        &restarted_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "workflow.update",
                "params": {
                    "workflow_id": "durable-review",
                    "workflow": {
                        "id": "durable-review",
                        "nodes": [
                            {
                                "id": "message",
                                "kind": "template",
                                "template": "updated after restart"
                            }
                        ]
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(update["result"]["workflow_id"], "durable-review");

    let updated_server = AppServer::with_storage_dir(&storage_dir);
    let updated: Value = serde_json::from_str(
        &updated_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "durable-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(
        updated["result"]["workflow"]["nodes"][0]["template"],
        "updated after restart"
    );

    let delete: Value = serde_json::from_str(
        &updated_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "workflow.delete",
                "params": {
                    "workflow_id": "durable-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(delete["result"]["deleted"], true);

    let deleted_server = AppServer::with_storage_dir(&storage_dir);
    let missing: Value = serde_json::from_str(
        &deleted_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "workflow.get",
                "params": {
                    "workflow_id": "durable-review"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(missing["error"]["code"], -32000);
}

#[test]
fn run_events_and_node_state_are_loaded_from_storage_after_server_restart() {
    let storage_dir = unique_storage_dir("run-restart");
    let first_server = AppServer::with_storage_dir(&storage_dir);

    first_server.handle_json(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workflow.create",
            "params": {
                "workflow": {
                    "id": "durable-run",
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

    let start: Value = serde_json::from_str(
        &first_server.handle_json(
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "workflow.start",
                "params": {
                    "workflow_id": "durable-run"
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start["result"]["run_id"].as_str().unwrap();

    let in_progress = poll_events_until(&first_server, run_id, 0, |events| {
        events["result"]["run"]["status"] == "running"
            && events["result"]["run"]["nodes"]["pause"]["status"] == "running"
    });
    let cursor_after_restart = in_progress["result"]["next_cursor"].as_u64().unwrap() as usize;

    let restarted_server = AppServer::with_storage_dir(&storage_dir);
    let resumed = poll_events_until(&restarted_server, run_id, cursor_after_restart, |events| {
        events["result"]["completed"].as_bool().unwrap()
    });

    assert_eq!(resumed["result"]["completed"], true);
    assert_eq!(resumed["result"]["run"]["status"], "completed");
    assert_eq!(
        resumed["result"]["run"]["nodes"]["pause"],
        json!({
            "status": "completed",
            "result": null
        })
    );
    assert_eq!(
        resumed["result"]["run"]["nodes"]["message"],
        json!({
            "status": "completed",
            "result": "done"
        })
    );
    assert!(
        resumed["result"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["type"] == "workflow_completed")
    );
}

#[test]
fn workflow_stop_cancels_running_workflow() {
    let server = AppServer::with_storage_dir(unique_storage_dir("stop-cancel"));
    // Use two sequential nodes so cancel can be detected between stages.
    // First node (quick) completes, then cancel is checked before the second (long delay) starts.
    let workflow = json!({
        "id": "delay-wf",
        "nodes": [
            {"id": "quick", "kind": "delay", "duration_ms": 150},
            {"id": "slow", "kind": "delay", "duration_ms": 5000},
        ],
        "dependencies": [{"node": "slow", "depends_on": "quick"}]
    });
    let create_result: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0", "id": 1, "method": "workflow.create",
                "params": {"workflow": workflow}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert!(create_result["result"].is_object());

    let start_result: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "workflow.start",
                "params": {"workflow_id": "delay-wf"}
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start_result["result"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Wait for quick node to start, then stop before slow node begins
    poll_events_until(&server, &run_id, 0, |events| {
        events["result"]["events"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .any(|e| e["type"] == "node_started" && e["node_id"] == "quick")
            })
            .unwrap_or(false)
    });

    let stop_result: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0", "id": 3, "method": "workflow.stop",
                "params": {"run_id": run_id}
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(stop_result["result"]["cancelled"], true);

    // 실행 완료 대기 (cancelled 이벤트 수신) - give extra time for quick node to finish
    let events = poll_events_until_timeout(&server, &run_id, 0, Duration::from_secs(5), |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });
    let summary = &events["result"]["run"];
    assert_eq!(summary["status"], "cancelled");
}

#[test]
fn workflow_start_step_mode_pauses() {
    let server = AppServer::with_storage_dir(unique_storage_dir("step-mode"));
    let workflow = json!({
        "id": "step-wf",
        "nodes": [
            {"id": "A", "kind": "constant", "value": 1},
            {"id": "B", "kind": "constant", "value": 2},
        ],
        "dependencies": [{"node": "B", "depends_on": "A"}]
    });
    server.handle_json(
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "workflow.create",
            "params": {"workflow": workflow}
        })
        .to_string(),
    );

    let start_result: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "workflow.start",
                "params": {"workflow_id": "step-wf", "mode": "step"}
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start_result["result"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // paused 이벤트 대기
    let events = poll_events_until(&server, &run_id, 0, |events| {
        events["result"]["run"]["paused"].as_bool().unwrap_or(false)
    });
    let summary = &events["result"]["run"];
    assert_eq!(
        summary["paused"], true,
        "step 모드에서 paused=true 예상: {summary}"
    );

    // resume → A 실행
    server.handle_json(
        &json!({
            "jsonrpc": "2.0", "id": 4, "method": "workflow.resume",
            "params": {"run_id": run_id}
        })
        .to_string(),
    );

    // A가 완료되고 다시 paused 또는 completed 대기
    poll_events_until(&server, &run_id, 0, |events| {
        let run = &events["result"]["run"];
        run["nodes"]["A"]["status"] == "completed"
    });

    // resume → B 실행
    server.handle_json(
        &json!({
            "jsonrpc": "2.0", "id": 5, "method": "workflow.resume",
            "params": {"run_id": run_id}
        })
        .to_string(),
    );

    let final_events = poll_events_until(&server, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });
    let summary = &final_events["result"]["run"];
    assert_eq!(summary["status"], "completed");
}

#[test]
fn run_summary_includes_first_failed_node_and_resumable() {
    let server = AppServer::with_storage_dir(unique_storage_dir("fail-summary"));
    let workflow = json!({
        "id": "fail-wf",
        "nodes": [
            {"id": "A", "kind": "constant", "value": 1},
            {"id": "B", "kind": "fail", "error": "boom"},
        ],
        "dependencies": [{"node": "B", "depends_on": "A"}]
    });
    server.handle_json(
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "workflow.create",
            "params": {"workflow": workflow}
        })
        .to_string(),
    );

    let start_result: Value = serde_json::from_str(
        &server.handle_json(
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "workflow.start",
                "params": {"workflow_id": "fail-wf"}
            })
            .to_string(),
        ),
    )
    .unwrap();
    let run_id = start_result["result"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let events = poll_events_until(&server, &run_id, 0, |events| {
        events["result"]["completed"].as_bool().unwrap_or(false)
    });
    let summary = &events["result"]["run"];
    assert_eq!(summary["status"], "failed");
    assert_eq!(summary["first_failed_node"], "B");
    assert_eq!(summary["resumable"], true);
}

fn poll_events_until(
    server: &AppServer,
    run_id: &str,
    cursor: usize,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    poll_events_until_timeout(server, run_id, cursor, Duration::from_secs(1), predicate)
}

fn poll_events_until_timeout(
    server: &AppServer,
    run_id: &str,
    cursor: usize,
    timeout: Duration,
    predicate: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = Instant::now() + timeout;

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

fn test_server() -> AppServer {
    AppServer::with_storage_dir(unique_storage_dir("json-rpc-workflow"))
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
