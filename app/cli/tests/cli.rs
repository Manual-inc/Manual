use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const EXPECTED_APP_SERVER_METHODS: &[&str] = &[
    "agent.list",
    "manual.activate",
    "manual.archive",
    "manual.clone",
    "manual.create",
    "manual.delete",
    "manual.get",
    "manual.list",
    "manual.update",
    "manual.versions",
    "node.create",
    "node.delete",
    "node.get",
    "node.list",
    "node.run",
    "node.run.events",
    "node.run.get",
    "node.schema",
    "node.testcase.save",
    "node.testcase.verify",
    "node.update",
    "optimization.analyze",
    "optimization.compare",
    "optimization.record_run",
    "optimization.report",
    "sandbox.create",
    "sandbox.evaluate",
    "sandbox.get",
    "sandbox.list",
    "sandbox.update",
    "skill.agent_capabilities",
    "skill.candidates",
    "skill.configure",
    "skill.record_execution",
    "skill.verify",
    "workflow.compose_from_registry",
    "workflow.create",
    "workflow.delete",
    "workflow.events",
    "workflow.get",
    "workflow.list",
    "workflow.patch",
    "workflow.resume",
    "workflow.start",
    "workflow.stop",
    "workflow.update",
];

#[test]
fn expected_method_set_matches_current_app_server_dispatch() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manual-rs/crates/app-server/src/lib.rs"),
    )
    .unwrap();
    let start = source.find("match request.method.as_str() {").unwrap();
    let end = source[start..].find("_ => rpc_error").unwrap() + start;
    let dispatch_block = &source[start..end];
    let actual = dispatch_block
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let method = trimmed.strip_prefix('"')?.split_once('"')?.0;
            Some(method.to_owned())
        })
        .collect::<BTreeSet<_>>();
    let expected = EXPECTED_APP_SERVER_METHODS
        .iter()
        .map(|method| (*method).to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(expected.len(), 46);
    assert_eq!(actual, expected);
}

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
fn doctor_reports_missing_binary_and_discovery_without_launching_server() {
    let temp = TestDir::new("manual-cli-doctor");
    let missing_server = temp.path().join("manual-app-server");
    let missing_discovery = temp.path().join("app-server.json");

    let output = manual_cli()
        .arg("--server-bin")
        .arg(&missing_server)
        .arg("--discovery-file")
        .arg(&missing_discovery)
        .arg("doctor")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Manual Doctor"));
    assert!(stdout.contains("Server binary: missing"));
    assert!(stdout.contains("Discovery file: missing"));
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
        "{\n  \"completed\": true,\n  \"events\": [\n    {\n      \"sequence\": 4,\n      \"type\": \"workflow_completed\"\n    }\n  ],\n  \"next_cursor\": 5,\n  \"optimization_analysis\": {\n    \"bottlenecks\": {\n      \"slow_steps\": [\n        \"implement\"\n      ],\n      \"token_waste\": [\n        \"implement\"\n      ],\n      \"unstable_tasks\": [\n        \"implement\"\n      ],\n      \"verification_gaps\": [\n        \"review\"\n      ]\n    },\n    \"measurement_mode\": \"derived\",\n    \"measurement_note\": \"Estimated from workflow events.\",\n    \"regression\": {\n      \"possible\": true,\n      \"reason\": \"tokens and time increased while success rate fell\",\n      \"step_id\": \"implement\"\n    },\n    \"suggestions\": [\n      \"preprocess file discovery\"\n    ]\n  },\n  \"optimization_report\": {\n    \"main_issue\": \"implementation step used most tokens\",\n    \"measurement_mode\": \"derived\",\n    \"measurement_note\": \"Estimated from workflow events.\",\n    \"recommendations\": [\n      \"preprocess file discovery\"\n    ],\n    \"sections\": [\n      \"Token Usage\",\n      \"Verification\",\n      \"Time\"\n    ]\n  },\n  \"run\": {\n    \"run_id\": \"run-7\",\n    \"status\": \"completed\"\n  }\n}\n"
    );

    let request = fs::read_to_string(log).unwrap();
    assert!(request.contains(r#""method":"workflow.events""#));
    assert!(request.contains(r#""run_id":"run-7""#));
    assert!(request.contains(r#""cursor":4"#));
}

#[test]
fn human_events_support_same_request_and_print_human_summary() {
    let temp = TestDir::new("manual-cli-events-human");
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
        .arg("--human")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Workflow Events"));
    assert!(stdout.contains("Optimization Report"));
    assert!(stdout.contains("Optimization Analysis"));

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

#[test]
fn workflow_extended_commands_send_expected_requests() {
    let temp = TestDir::new("manual-cli-workflow-extended");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let operations = write_temp_json(
        &temp,
        "workflow-patch.json",
        r#"[{"op":"delete_node","node_id":"draft"}]"#,
    );

    assert_request(
        &server,
        &log,
        [
            "workflow".into(),
            "patch".into(),
            "lead-review".into(),
            operations.display().to_string(),
        ],
        "workflow.patch",
        &[r#""workflow_id":"lead-review""#, r#""op":"delete_node""#],
    );

    assert_request(
        &server,
        &log,
        [
            "workflow".into(),
            "compose-from-registry".into(),
            "digest".into(),
        ],
        "workflow.compose_from_registry",
        &[r#""node_id":"digest""#],
    );
}

#[test]
fn node_commands_send_expected_requests() {
    let temp = TestDir::new("manual-cli-node");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let node = write_temp_json(
        &temp,
        "node.json",
        r#"{"id":"digest","kind":"template","template":"topic={{__storybook_input__.topic}}"}"#,
    );
    let inputs = write_temp_json(&temp, "inputs.json", r#"{"topic":"cli"}"#);
    let expected = write_temp_json(&temp, "expected.json", r#""Rendered string""#);
    let criteria = write_temp_json(&temp, "criteria.json", r#"{"contains":"Rendered string"}"#);

    assert_request(
        &server,
        &log,
        [
            "node".into(),
            "create".into(),
            node.display().to_string(),
            "--name".into(),
            "Digest node".into(),
            "--description".into(),
            "Summarizes a topic".into(),
        ],
        "node.create",
        &[
            r#""name":"Digest node""#,
            r#""description":"Summarizes a topic""#,
            r#""node":{"id":"digest""#,
        ],
    );

    assert_request(
        &server,
        &log,
        ["node".into(), "get".into(), "digest".into()],
        "node.get",
        &[r#""node_id":"digest""#],
    );

    assert_request(
        &server,
        &log,
        ["node".into(), "list".into()],
        "node.list",
        &[],
    );

    assert_request(
        &server,
        &log,
        [
            "node".into(),
            "update".into(),
            "digest".into(),
            "--node".into(),
            node.display().to_string(),
            "--name".into(),
            "Digest node v2".into(),
            "--description".into(),
            "Updates the digest node".into(),
        ],
        "node.update",
        &[
            r#""node_id":"digest""#,
            r#""name":"Digest node v2""#,
            r#""description":"Updates the digest node""#,
        ],
    );

    assert_request(
        &server,
        &log,
        ["node".into(), "delete".into(), "digest".into()],
        "node.delete",
        &[r#""node_id":"digest""#],
    );

    assert_request(
        &server,
        &log,
        ["node".into(), "schema".into(), "template".into()],
        "node.schema",
        &[r#""kind":"template""#],
    );

    assert_request(
        &server,
        &log,
        [
            "node".into(),
            "run".into(),
            node.display().to_string(),
            "--inputs".into(),
            inputs.display().to_string(),
        ],
        "node.run",
        &[r#""node":{"id":"digest""#, r#""inputs":{"topic":"cli"}"#],
    );

    assert_request(
        &server,
        &log,
        ["node".into(), "run-get".into(), "node-run-7".into()],
        "node.run.get",
        &[r#""run_id":"node-run-7""#],
    );

    assert_request(
        &server,
        &log,
        [
            "node".into(),
            "run-events".into(),
            "node-run-7".into(),
            "--cursor".into(),
            "3".into(),
        ],
        "node.run.events",
        &[r#""run_id":"node-run-7""#, r#""cursor":3"#],
    );

    assert_request(
        &server,
        &log,
        [
            "node".into(),
            "testcase-save".into(),
            "node-run-7".into(),
            "--expected-output".into(),
            expected.display().to_string(),
            "--criteria".into(),
            criteria.display().to_string(),
        ],
        "node.testcase.save",
        &[
            r#""run_id":"node-run-7""#,
            r#""expected_output":"Rendered string""#,
            r#""criteria":{"contains":"Rendered string"}"#,
        ],
    );

    assert_request(
        &server,
        &log,
        ["node".into(), "testcase-verify".into(), "digest".into()],
        "node.testcase.verify",
        &[r#""node_id":"digest""#],
    );
}

#[test]
fn manual_commands_send_expected_requests() {
    let temp = TestDir::new("manual-cli-manuals");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let manual = write_temp_json(
        &temp,
        "manual.json",
        r#"{"name":"Starter manual","description":"Explain onboarding"}"#,
    );
    let changes = write_temp_json(
        &temp,
        "manual-changes.json",
        r#"{"description":"Refined onboarding flow"}"#,
    );

    assert_request(
        &server,
        &log,
        [
            "manual".into(),
            "create".into(),
            manual.display().to_string(),
        ],
        "manual.create",
        &[
            r#""name":"Starter manual""#,
            r#""description":"Explain onboarding""#,
        ],
    );

    assert_request(
        &server,
        &log,
        ["manual".into(), "get".into(), "manual-1".into()],
        "manual.get",
        &[r#""manual_id":"manual-1""#],
    );

    assert_request(
        &server,
        &log,
        [
            "manual".into(),
            "list".into(),
            "--status".into(),
            "active".into(),
            "--query".into(),
            "starter".into(),
            "--tag".into(),
            "onboarding".into(),
        ],
        "manual.list",
        &[
            r#""status":"active""#,
            r#""query":"starter""#,
            r#""tag":"onboarding""#,
        ],
    );

    assert_request(
        &server,
        &log,
        [
            "manual".into(),
            "update".into(),
            "manual-1".into(),
            "--changes".into(),
            changes.display().to_string(),
            "--execution-affecting".into(),
        ],
        "manual.update",
        &[
            r#""manual_id":"manual-1""#,
            r#""changes":{"description":"Refined onboarding flow"}"#,
            r#""execution_affecting":true"#,
        ],
    );

    assert_request(
        &server,
        &log,
        ["manual".into(), "clone".into(), "manual-1".into()],
        "manual.clone",
        &[r#""manual_id":"manual-1""#],
    );

    assert_request(
        &server,
        &log,
        ["manual".into(), "archive".into(), "manual-1".into()],
        "manual.archive",
        &[r#""manual_id":"manual-1""#],
    );

    assert_request(
        &server,
        &log,
        ["manual".into(), "delete".into(), "manual-1".into()],
        "manual.delete",
        &[r#""manual_id":"manual-1""#],
    );

    assert_request(
        &server,
        &log,
        ["manual".into(), "activate".into(), "manual-1".into()],
        "manual.activate",
        &[r#""manual_id":"manual-1""#],
    );

    assert_request(
        &server,
        &log,
        ["manual".into(), "versions".into(), "manual-1".into()],
        "manual.versions",
        &[r#""manual_id":"manual-1""#],
    );
}

#[test]
fn optimization_sandbox_skill_and_agent_commands_send_expected_requests() {
    let temp = TestDir::new("manual-cli-nonworkflow");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let agent_params = write_temp_json(&temp, "agent-list.json", r#"{"preferred":"codex"}"#);
    let optimization_run = write_temp_json(
        &temp,
        "optimization-run.json",
        r#"{"manual_id":"manual-1","provider":"openai","tokens":120}"#,
    );
    let sandbox = write_temp_json(
        &temp,
        "sandbox.json",
        r#"{"name":"restricted","allow_write":["docs"],"allow_commands":["rg"]}"#,
    );
    let sandbox_changes = write_temp_json(
        &temp,
        "sandbox-changes.json",
        r#"{"allow_write":["docs","tmp"]}"#,
    );
    let skill_step = write_temp_json(
        &temp,
        "skill-step.json",
        r#"{"workflow_id":"wf-1","node_id":"digest","skill":"llm-wiki"}"#,
    );
    let skill_candidates = write_temp_json(
        &temp,
        "skill-candidates.json",
        r#"{"agent":"codex","task":"summarize docs"}"#,
    );
    let skill_execution = write_temp_json(
        &temp,
        "skill-execution.json",
        r#"{"execution":{"status":"used","duration_ms":42}}"#,
    );

    assert_request(
        &server,
        &log,
        [
            "agent".into(),
            "list".into(),
            "--params".into(),
            agent_params.display().to_string(),
        ],
        "agent.list",
        &[r#""preferred":"codex""#],
    );

    assert_request(
        &server,
        &log,
        [
            "optimization".into(),
            "record-run".into(),
            optimization_run.display().to_string(),
        ],
        "optimization.record_run",
        &[r#""manual_id":"manual-1""#, r#""tokens":120"#],
    );

    assert_request(
        &server,
        &log,
        ["optimization".into(), "analyze".into()],
        "optimization.analyze",
        &[],
    );

    assert_request(
        &server,
        &log,
        ["optimization".into(), "analyze".into(), "--human".into()],
        "optimization.analyze",
        &[],
    );

    assert_request(
        &server,
        &log,
        ["optimization".into(), "compare".into()],
        "optimization.compare",
        &[],
    );

    assert_request(
        &server,
        &log,
        ["optimization".into(), "compare".into(), "--human".into()],
        "optimization.compare",
        &[],
    );

    assert_request(
        &server,
        &log,
        ["optimization".into(), "report".into()],
        "optimization.report",
        &[],
    );

    assert_request(
        &server,
        &log,
        ["optimization".into(), "report".into(), "--human".into()],
        "optimization.report",
        &[],
    );

    assert_request(
        &server,
        &log,
        [
            "sandbox".into(),
            "create".into(),
            sandbox.display().to_string(),
        ],
        "sandbox.create",
        &[r#""name":"restricted""#, r#""allow_commands":["rg"]"#],
    );

    assert_request(
        &server,
        &log,
        [
            "sandbox".into(),
            "update".into(),
            "sandbox-1".into(),
            "--changes".into(),
            sandbox_changes.display().to_string(),
        ],
        "sandbox.update",
        &[
            r#""sandbox_id":"sandbox-1""#,
            r#""changes":{"allow_write":["docs","tmp"]}"#,
        ],
    );

    assert_request(
        &server,
        &log,
        [
            "sandbox".into(),
            "evaluate".into(),
            "sandbox-1".into(),
            "write".into(),
            "./docs".into(),
        ],
        "sandbox.evaluate",
        &[
            r#""sandbox_id":"sandbox-1""#,
            r#""operation":"write_file""#,
            r#""target":"./docs""#,
        ],
    );

    assert_request(
        &server,
        &log,
        ["sandbox".into(), "get".into(), "sandbox-1".into()],
        "sandbox.get",
        &[r#""sandbox_id":"sandbox-1""#],
    );

    assert_request(
        &server,
        &log,
        ["sandbox".into(), "list".into()],
        "sandbox.list",
        &[],
    );

    assert_request(
        &server,
        &log,
        [
            "skill".into(),
            "configure".into(),
            skill_step.display().to_string(),
        ],
        "skill.configure",
        &[r#""skill":"llm-wiki""#, r#""node_id":"digest""#],
    );

    assert_request(
        &server,
        &log,
        [
            "skill".into(),
            "candidates".into(),
            skill_candidates.display().to_string(),
        ],
        "skill.candidates",
        &[r#""agent":"codex""#, r#""task":"summarize docs""#],
    );

    assert_request(
        &server,
        &log,
        [
            "skill".into(),
            "record-execution".into(),
            "step-1".into(),
            "--execution".into(),
            skill_execution.display().to_string(),
        ],
        "skill.record_execution",
        &[
            r#""step_id":"step-1""#,
            r#""status":"used""#,
            r#""duration_ms":42"#,
        ],
    );

    assert_request(
        &server,
        &log,
        ["skill".into(), "verify".into(), "step-1".into()],
        "skill.verify",
        &[r#""step_id":"step-1""#],
    );

    assert_request(
        &server,
        &log,
        ["skill".into(), "agent-capabilities".into()],
        "skill.agent_capabilities",
        &[],
    );
}

fn manual_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_manual"))
}

fn assert_request<const N: usize>(
    server: &Path,
    log: &Path,
    args: [String; N],
    expected_method: &str,
    expected_fragments: &[&str],
) {
    let output = run_manual(server, args);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let request = last_request(log);
    assert!(
        request.contains(&format!(r#""method":"{expected_method}""#)),
        "expected method {expected_method}, request was {request}"
    );

    for fragment in expected_fragments {
        assert!(
            request.contains(fragment),
            "missing fragment {fragment} in request {request}"
        );
    }
}

fn run_manual<I, S>(server: &Path, args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = manual_cli();
    command.arg("--server-bin").arg(server);
    for arg in args {
        command.arg(arg);
    }
    command.output().unwrap()
}

fn last_request(log: &Path) -> String {
    fs::read_to_string(log)
        .unwrap()
        .lines()
        .last()
        .unwrap()
        .to_owned()
}

fn write_temp_json(temp: &TestDir, name: &str, contents: &str) -> PathBuf {
    let path = temp.path().join(name);
    fs::write(&path, contents).unwrap();
    path
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
                "optimization_report": {{
                    "sections": ["Token Usage", "Verification", "Time"],
                    "main_issue": "implementation step used most tokens",
                    "recommendations": ["preprocess file discovery"],
                    "measurement_mode": "derived",
                    "measurement_note": "Estimated from workflow events."
                }},
                "optimization_analysis": {{
                    "measurement_mode": "derived",
                    "measurement_note": "Estimated from workflow events.",
                    "regression": {{
                        "possible": True,
                        "step_id": "implement",
                        "reason": "tokens and time increased while success rate fell"
                    }},
                    "bottlenecks": {{
                        "token_waste": ["implement"],
                        "verification_gaps": ["review"],
                        "slow_steps": ["implement"],
                        "unstable_tasks": ["implement"]
                    }},
                    "suggestions": ["preprocess file discovery"]
                }},
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
