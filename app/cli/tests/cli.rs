use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn create_sends_workflow_file_to_app_server() {
    let temp = TestDir::new("manual-cli-create");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let workflow = temp.path().join("workflow.json");
    fs::write(
        &workflow,
        r#"{"id":"lead-review","nodes":[{"id":"message","kind":"template","template":"done"}]}"#,
    )
    .unwrap();

    let output = manual_cli()
        .arg("--server-bin")
        .arg(&server)
        .arg("workflow")
        .arg("create")
        .arg(&workflow)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "{\n  \"node_count\": 1,\n  \"workflow_id\": \"lead-review\"\n}\n"
    );

    let request = fs::read_to_string(log).unwrap();
    assert!(request.contains(r#""method":"workflow.create""#));
    assert!(request.contains(r#""workflow":{"id":"lead-review""#));
}

#[test]
fn events_supports_cursor_and_prints_server_run_summary() {
    let temp = TestDir::new("manual-cli-events");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = manual_cli()
        .arg("--server-bin")
        .arg(&server)
        .arg("workflow")
        .arg("events")
        .arg("run-7")
        .arg("--cursor")
        .arg("4")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "{\n  \"completed\": true,\n  \"events\": [\n    {\n      \"sequence\": 4,\n      \"type\": \"workflow_completed\"\n    }\n  ],\n  \"next_cursor\": 5,\n  \"run\": {\n    \"run_id\": \"run-7\",\n    \"status\": \"completed\"\n  }\n}\n"
    );

    let request = fs::read_to_string(log).unwrap();
    assert!(request.contains(r#""method":"workflow.events""#));
    assert!(request.contains(r#""run_id":"run-7""#));
    assert!(request.contains(r#""cursor":4"#));
}

#[test]
fn rpc_errors_are_reported_with_nonzero_exit_status() {
    let temp = TestDir::new("manual-cli-error");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = manual_cli()
        .arg("--server-bin")
        .arg(&server)
        .arg("workflow")
        .arg("get")
        .arg("missing")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "app-server error -32000: workflow not found\n"
    );
}

#[test]
fn workflow_stop_sends_run_id_to_server() {
    let temp = TestDir::new("manual-cli-stop");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = manual_cli()
        .arg("--server-bin")
        .arg(&server)
        .arg("workflow")
        .arg("stop")
        .arg("run-42")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let request = fs::read_to_string(log).unwrap();
    assert!(request.contains(r#""method":"workflow.stop""#));
    assert!(request.contains(r#""run_id":"run-42""#));
}

#[test]
fn workflow_resume_sends_run_id_to_server() {
    let temp = TestDir::new("manual-cli-resume");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = manual_cli()
        .arg("--server-bin")
        .arg(&server)
        .arg("workflow")
        .arg("resume")
        .arg("run-99")
        .arg("--mode")
        .arg("step")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let request = fs::read_to_string(log).unwrap();
    assert!(request.contains(r#""method":"workflow.resume""#));
    assert!(request.contains(r#""run_id":"run-99""#));
    assert!(request.contains(r#""mode":"step""#));
}

#[test]
fn workflow_start_passes_mode_and_flags() {
    let temp = TestDir::new("manual-cli-start-flags");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = manual_cli()
        .arg("--server-bin")
        .arg(&server)
        .arg("workflow")
        .arg("start")
        .arg("my-workflow")
        .arg("--mode")
        .arg("step")
        .arg("--resume-from-failure")
        .arg("--start-node")
        .arg("node-2")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let request = fs::read_to_string(log).unwrap();
    assert!(request.contains(r#""method":"workflow.start""#));
    assert!(request.contains(r#""workflow_id":"my-workflow""#));
    assert!(request.contains(r#""mode":"step""#));
    assert!(request.contains(r#""resume_from_failure":true"#));
    assert!(request.contains(r#""start_node_id":"node-2""#));
}

fn manual_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_manual"))
}

fn fake_server(temp: &TestDir, log: &Path) -> PathBuf {
    let server = temp.path().join("fake_app_server.py");
    fs::write(
        &server,
        format!(
            r#"#!/usr/bin/env python3
import json
import sys

log_path = {log_path:?}

for line in sys.stdin:
    request = json.loads(line)
    with open(log_path, "a", encoding="utf-8") as log:
        log.write(json.dumps(request, separators=(",", ":")) + "\n")

    method = request["method"]
    if method == "workflow.create":
        workflow = request["params"]["workflow"]
        result = {{"workflow_id": workflow["id"], "node_count": len(workflow["nodes"])}}
        response = {{"jsonrpc": "2.0", "id": request["id"], "result": result}}
    elif method == "workflow.events":
        response = {{
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {{
                "events": [{{"sequence": request["params"]["cursor"], "type": "workflow_completed"}}],
                "next_cursor": request["params"]["cursor"] + 1,
                "completed": True,
                "run": {{"run_id": request["params"]["run_id"], "status": "completed"}},
            }},
        }}
    elif method == "workflow.get":
        response = {{
            "jsonrpc": "2.0",
            "id": request["id"],
            "error": {{"code": -32000, "message": "workflow not found"}},
        }}
    else:
        response = {{"jsonrpc": "2.0", "id": request["id"], "result": {{"ok": True}}}}

    print(json.dumps(response, separators=(",", ":")), flush=True)
"#,
            log_path = log.display().to_string()
        ),
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&server).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&server, permissions).unwrap();
    }

    server
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
