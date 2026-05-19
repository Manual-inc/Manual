//! Why this exists: docs/wiki/architecture/manual-cli-command-surface.md requires
//! the dedicated CLI command groups to work against the real app-server
//! implementation, not only a fake JSON-RPC stub.

use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use app_server::{AppServer, HttpServerConfig, serve_http_listener};
use serde_json::{Value, json};

#[test]
fn workflow_and_node_commands_work_against_real_app_server() {
    let harness = RealHarness::new("manual-cli-real-workflow");

    let workflow_main = harness.write_json(
        "workflow-main.json",
        &json!({
            "id": "wf-main",
            "nodes": [
                { "id": "draft", "kind": "constant", "value": "hello" },
                { "id": "digest", "kind": "template", "template": "value={{draft}}" }
            ],
            "dependencies": [
                { "node": "digest", "depends_on": "draft" }
            ]
        }),
    );
    let workflow_updated = harness.write_json(
        "workflow-updated.json",
        &json!({
            "id": "wf-main",
            "nodes": [
                { "id": "draft", "kind": "constant", "value": "hello" },
                { "id": "digest", "kind": "template", "template": "updated={{draft}}" }
            ],
            "dependencies": [
                { "node": "digest", "depends_on": "draft" }
            ]
        }),
    );
    let workflow_patch = harness.write_json(
        "workflow-patch.json",
        &json!([
            {
                "op": "update_node",
                "node": {
                    "id": "digest",
                    "kind": "template",
                    "template": "patched={{draft}}"
                }
            }
        ]),
    );
    let workflow_stop = harness.write_json(
        "workflow-stop.json",
        &json!({
            "id": "wf-stop",
            "nodes": [
                { "id": "pause", "kind": "delay", "duration_ms": 350 },
                { "id": "done", "kind": "template", "template": "done" }
            ],
            "dependencies": [
                { "node": "done", "depends_on": "pause" }
            ]
        }),
    );
    let workflow_step = harness.write_json(
        "workflow-step.json",
        &json!({
            "id": "wf-step",
            "nodes": [
                { "id": "draft", "kind": "constant", "value": "hello" }
            ]
        }),
    );
    let workflow_watch = harness.write_json(
        "workflow-watch.json",
        &json!({
            "id": "wf-watch",
            "nodes": [
                { "id": "pause", "kind": "delay", "duration_ms": 120 },
                { "id": "done", "kind": "template", "template": "done" }
            ],
            "dependencies": [
                { "node": "done", "depends_on": "pause" }
            ]
        }),
    );
    let node_initial = harness.write_json(
        "node-initial.json",
        &json!({
            "id": "story-digest",
            "kind": "template",
            "template": "topic={{__storybook_input__.topic}}"
        }),
    );
    let node_updated = harness.write_json(
        "node-updated.json",
        &json!({
            "id": "story-digest",
            "kind": "template",
            "template": "patched-topic={{__storybook_input__.topic}}"
        }),
    );
    let node_inputs = harness.write_json("node-inputs.json", &json!({ "topic": "cli" }));
    let node_expected = harness.write_json("node-expected.json", &json!("patched-topic=cli"));
    let node_criteria = harness.write_json(
        "node-criteria.json",
        &json!({
            "comparison": "json_equal",
            "schema_match_required": true
        }),
    );

    let created = harness.run_jsons([
        "workflow".into(),
        "create".into(),
        workflow_main.display().to_string(),
    ]);
    assert_eq!(created[0]["workflow_id"], "wf-main");
    assert_eq!(created[0]["node_count"], 2);

    let listed = harness.run_jsons(["workflow".into(), "list".into()]);
    assert!(
        listed[0]["workflows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|workflow| workflow["workflow_id"] == "wf-main")
    );

    let fetched = harness.run_jsons(["workflow".into(), "get".into(), "wf-main".into()]);
    assert_eq!(fetched[0]["workflow"]["id"], "wf-main");
    assert_eq!(
        fetched[0]["workflow"]["nodes"][1]["template"],
        "value={{draft}}"
    );

    let updated = harness.run_jsons([
        "workflow".into(),
        "update".into(),
        "wf-main".into(),
        workflow_updated.display().to_string(),
    ]);
    assert_eq!(updated[0]["workflow_id"], "wf-main");

    let patched = harness.run_jsons([
        "workflow".into(),
        "patch".into(),
        "wf-main".into(),
        workflow_patch.display().to_string(),
    ]);
    assert_eq!(patched[0]["workflow_id"], "wf-main");

    let node_created = harness.run_jsons([
        "node".into(),
        "create".into(),
        node_initial.display().to_string(),
        "--name".into(),
        "Story digest".into(),
        "--description".into(),
        "Registry template".into(),
    ]);
    assert_eq!(node_created[0]["template"]["id"], "story-digest");

    let node_listed = harness.run_jsons(["node".into(), "list".into()]);
    assert!(
        node_listed[0]["templates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|template| template["id"] == "story-digest")
    );

    let node_fetched = harness.run_jsons(["node".into(), "get".into(), "story-digest".into()]);
    assert_eq!(node_fetched[0]["template"]["name"], "Story digest");

    let node_updated_result = harness.run_jsons([
        "node".into(),
        "update".into(),
        "story-digest".into(),
        "--node".into(),
        node_updated.display().to_string(),
        "--name".into(),
        "Story digest v2".into(),
        "--description".into(),
        "Updated registry template".into(),
    ]);
    assert_eq!(
        node_updated_result[0]["template"]["name"],
        "Story digest v2"
    );
    assert_eq!(
        node_updated_result[0]["template"]["node"]["template"],
        "patched-topic={{__storybook_input__.topic}}"
    );

    let schema = harness.run_jsons(["node".into(), "schema".into(), "template".into()]);
    assert_eq!(schema[0]["schema"]["kind"], "template");

    let node_run = harness.run_jsons([
        "node".into(),
        "run".into(),
        node_updated.display().to_string(),
        "--inputs".into(),
        node_inputs.display().to_string(),
    ]);
    let node_run_id = node_run[0]["run_id"].as_str().unwrap().to_owned();
    let node_events = harness.wait_for_node_completion(&node_run_id);
    assert_eq!(node_events["run"]["status"], "completed");

    let node_run_get = harness.run_jsons(["node".into(), "run-get".into(), node_run_id.clone()]);
    assert_eq!(node_run_get[0]["run"]["status"], "completed");
    assert_eq!(node_run_get[0]["run"]["result"], "patched-topic=cli");

    let node_run_events = harness.run_jsons([
        "node".into(),
        "run-events".into(),
        node_run_id.clone(),
        "--cursor".into(),
        "0".into(),
    ]);
    assert_eq!(node_run_events[0]["completed"], true);

    let node_case = harness.run_jsons([
        "node".into(),
        "testcase-save".into(),
        node_run_id.clone(),
        "--expected-output".into(),
        node_expected.display().to_string(),
        "--criteria".into(),
        node_criteria.display().to_string(),
    ]);
    assert_eq!(node_case[0]["test_case"]["node_id"], "story-digest");

    let node_verify = harness.run_jsons([
        "node".into(),
        "testcase-verify".into(),
        "story-digest".into(),
    ]);
    assert_eq!(node_verify[0]["failed"], json!([]));
    assert_eq!(node_verify[0]["results"][0]["passed"], true);

    let composed = harness.run_jsons([
        "workflow".into(),
        "compose-from-registry".into(),
        "story-digest".into(),
    ]);
    assert_eq!(composed[0]["candidate"]["id"], "story-digest");

    let workflow_start = harness.run_jsons(["workflow".into(), "start".into(), "wf-main".into()]);
    let workflow_run_id = workflow_start[0]["run_id"].as_str().unwrap().to_owned();
    let workflow_events = harness.wait_for_workflow_completion(&workflow_run_id);
    assert_eq!(workflow_events["run"]["status"], "completed");

    let workflow_watch_run = harness.run_jsons([
        "workflow".into(),
        "create".into(),
        workflow_watch.display().to_string(),
    ]);
    assert_eq!(workflow_watch_run[0]["workflow_id"], "wf-watch");
    let watch_started = harness.run_jsons(["workflow".into(), "start".into(), "wf-watch".into()]);
    let watch_run_id = watch_started[0]["run_id"].as_str().unwrap().to_owned();
    let watch_events = harness.run_jsons([
        "workflow".into(),
        "events".into(),
        watch_run_id,
        "--watch".into(),
        "--interval-ms".into(),
        "10".into(),
    ]);
    assert_eq!(watch_events.last().unwrap()["completed"], true);

    let workflow_run_docs = harness.run_jsons([
        "workflow".into(),
        "run".into(),
        "wf-main".into(),
        "--interval-ms".into(),
        "10".into(),
    ]);
    assert!(workflow_run_docs.len() >= 2);
    assert!(workflow_run_docs[0]["run_id"].is_string());
    assert_eq!(workflow_run_docs.last().unwrap()["completed"], true);

    let workflow_run_human = harness.run::<Vec<String>, String>(vec![
        "workflow".into(),
        "run".into(),
        "wf-main".into(),
        "--interval-ms".into(),
        "10".into(),
        "--human".into(),
    ]);
    assert!(workflow_run_human.status.success());
    let workflow_human_stdout = String::from_utf8_lossy(&workflow_run_human.stdout);
    assert!(workflow_human_stdout.contains("Started workflow run"));
    assert!(workflow_human_stdout.contains("Workflow Events"));
    assert!(workflow_human_stdout.contains("Optimization Report"));
    assert!(workflow_human_stdout.contains("Optimization Analysis"));
    assert!(workflow_human_stdout.contains("Measurements"));
    assert_eq!(workflow_human_stdout.matches("Workflow Events").count(), 1);

    let workflow_stop_created = harness.run_jsons([
        "workflow".into(),
        "create".into(),
        workflow_stop.display().to_string(),
    ]);
    assert_eq!(workflow_stop_created[0]["workflow_id"], "wf-stop");
    let stop_started = harness.run_jsons(["workflow".into(), "start".into(), "wf-stop".into()]);
    let stop_run_id = stop_started[0]["run_id"].as_str().unwrap().to_owned();
    let stopped = harness.run_jsons(["workflow".into(), "stop".into(), stop_run_id.clone()]);
    assert_eq!(stopped[0]["cancelled"], true);
    let stop_events = harness.wait_for_workflow_completion(&stop_run_id);
    assert_eq!(stop_events["run"]["status"], "cancelled");

    let workflow_step_created = harness.run_jsons([
        "workflow".into(),
        "create".into(),
        workflow_step.display().to_string(),
    ]);
    assert_eq!(workflow_step_created[0]["workflow_id"], "wf-step");
    let step_started = harness.run_jsons([
        "workflow".into(),
        "start".into(),
        "wf-step".into(),
        "--mode".into(),
        "step".into(),
    ]);
    let step_run_id = step_started[0]["run_id"].as_str().unwrap().to_owned();
    let paused = harness.wait_for_workflow_pause(&step_run_id);
    assert_eq!(paused["run"]["paused"], true);
    let resumed = harness.run_jsons([
        "workflow".into(),
        "resume".into(),
        step_run_id.clone(),
        "--mode".into(),
        "step".into(),
    ]);
    assert_eq!(resumed[0]["resumed"], true);
    let resumed_events = harness.wait_for_workflow_completion(&step_run_id);
    assert_eq!(resumed_events["run"]["status"], "completed");

    let deleted = harness.run_jsons(["workflow".into(), "delete".into(), "wf-main".into()]);
    assert_eq!(deleted[0]["deleted"], true);

    let node_deleted = harness.run_jsons(["node".into(), "delete".into(), "story-digest".into()]);
    assert_eq!(node_deleted[0]["deleted"], true);
}

#[test]
fn manual_sandbox_skill_optimization_and_agent_commands_work_against_real_app_server() {
    let harness = RealHarness::new("manual-cli-real-manual");

    let manual_json = harness.write_json(
        "manual.json",
        &json!({
            "name": "Starter manual",
            "description": "Reusable docs workflow",
            "tags": ["docs", "mvp"],
            "default_agent": "codex"
        }),
    );
    let manual_changes = harness.write_json(
        "manual-changes.json",
        &json!({
            "description": "Updated docs workflow",
            "workflow_steps": [
                {
                    "id": "agent-step",
                    "kind": "codex",
                    "input_schema": [{ "name": "prompt", "required": true }],
                    "output_schema": "agent result object",
                    "verification_policy": { "required": true, "criteria": ["tests pass", "docs linked"] },
                    "sandbox_policy": { "sandbox_id": "default" },
                    "token_budget": 5000
                }
            ]
        }),
    );
    let sandbox_json = harness.write_json(
        "sandbox.json",
        &json!({
            "name": "Scratch Writer",
            "allow_write": ["/tmp/**"]
        }),
    );
    let sandbox_changes = harness.write_json(
        "sandbox-changes.json",
        &json!({
            "allow_write": ["/tmp/**", "/var/tmp/**"]
        }),
    );
    let optimization_before_json = harness.write_json(
        "optimization-before.json",
        &json!({
            "run_id": "opt-before",
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
                "requirements_satisfied": 0.94,
                "pass_rate": 0.94,
                "items": [],
                "missing": [],
                "risks": []
            },
            "time": {
                "total_ms": 700,
                "by_step": [{ "step_id": "plan", "duration_ms": 700, "retries": 0 }],
                "review_ms": 0
            }
        }),
    );
    let optimization_after_json = harness.write_json(
        "optimization-after.json",
        &json!({
            "run_id": "opt-after",
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
                "requirements_satisfied": 0.78,
                "pass_rate": 0.7,
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
            },
            "model_calls": [
                { "step_id": "implement", "model": "gpt-5.5", "tokens": 5200, "cost": 0.52, "reason": "high-risk implementation" }
            ]
        }),
    );
    let optimization_analyze_json = harness.write_json(
        "optimization-analyze.json",
        &json!({
            "workflow_id": "wf-main"
        }),
    );
    let optimization_compare_json = harness.write_json(
        "optimization-compare.json",
        &json!({
            "workflow_id": "wf-main",
            "before_run_id": "opt-before",
            "after_run_id": "opt-after"
        }),
    );
    let skill_json = harness.write_json(
        "skill.json",
        &json!({
            "step_id": "agent-step",
            "task_type": "documentation",
            "agent": "codex",
            "skills": ["llm-wiki"]
        }),
    );
    let skill_candidates_json = harness.write_json(
        "skill-candidates.json",
        &json!({
            "task_type": "documentation"
        }),
    );
    let skill_execution_json = harness.write_json(
        "skill-execution.json",
        &json!({
            "execution": {
                "observed_skill_signals": ["other-skill"],
                "logs": ["custom execution log"]
            }
        }),
    );
    let agent_bin = harness.temp.path().join("bin");
    fs::create_dir_all(&agent_bin).unwrap();
    let fake_codex = agent_bin.join("codex");
    fs::write(&fake_codex, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&fake_codex).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_codex, permissions).unwrap();
    }
    let agent_params_json = harness.write_json(
        "agent-list.json",
        &json!({
            "candidates": ["codex", "missing-agent"],
            "path_dirs": [agent_bin]
        }),
    );

    let created_manual = harness.run_jsons([
        "manual".into(),
        "create".into(),
        manual_json.display().to_string(),
    ]);
    let manual_id = created_manual[0]["manual"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(created_manual[0]["manual"]["name"], "Starter manual");

    let fetched_manual = harness.run_jsons(["manual".into(), "get".into(), manual_id.clone()]);
    assert_eq!(fetched_manual[0]["manual"]["id"], manual_id);

    let listed_manuals = harness.run_jsons([
        "manual".into(),
        "list".into(),
        "--status".into(),
        "draft".into(),
        "--query".into(),
        "starter".into(),
        "--tag".into(),
        "docs".into(),
    ]);
    assert!(
        listed_manuals[0]["manuals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|manual| manual["id"] == manual_id)
    );

    let updated_manual = harness.run_jsons([
        "manual".into(),
        "update".into(),
        manual_id.clone(),
        "--changes".into(),
        manual_changes.display().to_string(),
        "--execution-affecting".into(),
    ]);
    assert_eq!(updated_manual[0]["manual"]["current_version"], 2);

    let manual_versions =
        harness.run_jsons(["manual".into(), "versions".into(), manual_id.clone()]);
    assert_eq!(manual_versions[0]["current_version"], 2);

    let cloned_manual = harness.run_jsons(["manual".into(), "clone".into(), manual_id.clone()]);
    let cloned_manual_id = cloned_manual[0]["manual"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(cloned_manual_id, manual_id);

    let archived_manual =
        harness.run_jsons(["manual".into(), "archive".into(), cloned_manual_id.clone()]);
    assert_eq!(archived_manual[0]["manual"]["status"], "archived");

    let activated_manual =
        harness.run_jsons(["manual".into(), "activate".into(), manual_id.clone()]);
    assert_eq!(activated_manual[0]["manual"]["status"], "active");

    let deleted_manual = harness.run_jsons(["manual".into(), "delete".into(), cloned_manual_id]);
    assert_eq!(deleted_manual[0]["manual"]["deleted"], true);

    let sandbox_created = harness.run_jsons([
        "sandbox".into(),
        "create".into(),
        sandbox_json.display().to_string(),
    ]);
    let sandbox_id = sandbox_created[0]["sandbox"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(sandbox_created[0]["sandbox"]["name"], "Scratch Writer");

    let sandbox_listed = harness.run_jsons(["sandbox".into(), "list".into()]);
    assert!(
        sandbox_listed[0]["sandboxes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|sandbox| sandbox["id"] == sandbox_id)
    );

    let sandbox_fetched = harness.run_jsons(["sandbox".into(), "get".into(), sandbox_id.clone()]);
    assert_eq!(sandbox_fetched[0]["sandbox"]["id"], sandbox_id);

    let sandbox_updated = harness.run_jsons([
        "sandbox".into(),
        "update".into(),
        sandbox_id.clone(),
        "--changes".into(),
        sandbox_changes.display().to_string(),
    ]);
    assert_eq!(
        sandbox_updated[0]["sandbox"]["allow_write"][1],
        "/var/tmp/**"
    );

    let sandbox_evaluated = harness.run_jsons([
        "sandbox".into(),
        "evaluate".into(),
        sandbox_id,
        "write".into(),
        "/tmp/test-file.txt".into(),
    ]);
    assert_eq!(sandbox_evaluated[0]["decision"]["allowed"], true);

    let optimization_before_recorded = harness.run_jsons([
        "optimization".into(),
        "record-run".into(),
        optimization_before_json.display().to_string(),
    ]);
    assert_eq!(optimization_before_recorded[0]["run"]["id"], "opt-before");

    let optimization_after_recorded = harness.run_jsons([
        "optimization".into(),
        "record-run".into(),
        optimization_after_json.display().to_string(),
    ]);
    assert_eq!(optimization_after_recorded[0]["run"]["id"], "opt-after");

    let optimization_analysis = harness.run_jsons([
        "optimization".into(),
        "analyze".into(),
        "--params".into(),
        optimization_analyze_json.display().to_string(),
    ]);
    assert!(optimization_analysis[0]["candidates"].is_array());
    assert_eq!(optimization_analysis[0]["regression"]["possible"], true);

    let optimization_analysis_human = harness.run([
        "optimization".into(),
        "analyze".into(),
        "--params".into(),
        optimization_analyze_json.display().to_string(),
        "--human".into(),
    ]);
    assert!(optimization_analysis_human.status.success());
    let analysis_stdout = String::from_utf8_lossy(&optimization_analysis_human.stdout);
    assert!(analysis_stdout.contains("Optimization Analysis"));
    assert!(analysis_stdout.contains("Regression"));
    assert!(analysis_stdout.contains("Implement"));
    assert!(analysis_stdout.contains("Measurements"));

    let optimization_compare = harness.run_jsons([
        "optimization".into(),
        "compare".into(),
        "--params".into(),
        optimization_compare_json.display().to_string(),
    ]);
    assert_eq!(optimization_compare[0]["token_delta"], 4400);

    let optimization_compare_human = harness.run([
        "optimization".into(),
        "compare".into(),
        "--params".into(),
        optimization_compare_json.display().to_string(),
        "--human".into(),
    ]);
    assert!(optimization_compare_human.status.success());
    let compare_stdout = String::from_utf8_lossy(&optimization_compare_human.stdout);
    assert!(compare_stdout.contains("Optimization Comparison"));
    assert!(compare_stdout.contains("Token Delta"));
    assert!(compare_stdout.contains("4400"));
    assert!(compare_stdout.contains("Measurements"));

    let optimization_report = harness.run_jsons([
        "optimization".into(),
        "report".into(),
        "--params".into(),
        optimization_analyze_json.display().to_string(),
    ]);
    assert_eq!(
        optimization_report[0]["main_issue"],
        "implementation step used most tokens"
    );

    let optimization_report_human = harness.run([
        "optimization".into(),
        "report".into(),
        "--params".into(),
        optimization_analyze_json.display().to_string(),
        "--human".into(),
    ]);
    assert!(optimization_report_human.status.success());
    let human_stdout = String::from_utf8_lossy(&optimization_report_human.stdout);
    assert!(human_stdout.contains("Optimization Report"));
    assert!(human_stdout.contains("Main Issue"));
    assert!(human_stdout.contains("implementation step used most tokens"));
    assert!(human_stdout.contains("Recommendations"));

    let configured_skill = harness.run_jsons([
        "skill".into(),
        "configure".into(),
        skill_json.display().to_string(),
    ]);
    assert_eq!(configured_skill[0]["step"]["id"], "agent-step");

    let skill_candidates = harness.run_jsons([
        "skill".into(),
        "candidates".into(),
        skill_candidates_json.display().to_string(),
    ]);
    assert!(
        skill_candidates[0]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate["name"] == "llm-wiki")
    );

    let recorded_skill = harness.run_jsons([
        "skill".into(),
        "record-execution".into(),
        "agent-step".into(),
        "--execution".into(),
        skill_execution_json.display().to_string(),
    ]);
    assert_eq!(
        recorded_skill[0]["execution"]["observed_skill_signals"],
        json!(["other-skill"])
    );
    assert_eq!(
        recorded_skill[0]["execution"]["logs"],
        json!(["custom execution log"])
    );

    let verified_skill = harness.run_jsons(["skill".into(), "verify".into(), "agent-step".into()]);
    assert_eq!(verified_skill[0]["used"], false);
    assert_eq!(verified_skill[0]["status"], "unknown");

    let skill_capabilities = harness.run_jsons(["skill".into(), "agent-capabilities".into()]);
    assert!(skill_capabilities[0]["agents"].is_array());

    let listed_agents = harness.run_jsons([
        "agent".into(),
        "list".into(),
        "--params".into(),
        agent_params_json.display().to_string(),
    ]);
    assert_eq!(listed_agents[0]["agents"][0]["name"], "codex");
    assert_eq!(listed_agents[0]["agents"][0]["available"], true);
    assert_eq!(listed_agents[0]["agents"][1]["available"], false);
}

#[test]
fn demo_optimization_command_runs_end_to_end_flow() {
    let harness = RealHarness::new("manual-cli-demo");

    let output = harness.run(vec![
        "demo".to_owned(),
        "optimization".to_owned(),
    ]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Started workflow run"));
    assert!(stdout.contains("Workflow Events"));
    assert!(stdout.contains("Optimization Report"));
    assert!(stdout.contains("Optimization Analysis"));
    assert!(stdout.contains("Digest step used most tokens"));
    assert_eq!(stdout.matches("Workflow Events").count(), 1);
    assert_eq!(stdout.matches("Optimization Report").count(), 1);
    assert_eq!(stdout.matches("Optimization Analysis").count(), 1);

    let second_output = harness.run(vec![
        "demo".to_owned(),
        "optimization".to_owned(),
    ]);
    assert!(
        second_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second_output.stderr)
    );

    let workflows = harness.run_jsons(["workflow".into(), "list".into()]);
    let demo_count = workflows[0]["workflows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|workflow| workflow["workflow_id"] == "demo-optimization")
        .count();
    assert_eq!(demo_count, 1);
}

struct RealHarness {
    temp: TestDir,
    server_url: String,
    auth_token: String,
}

impl RealHarness {
    fn new(name: &str) -> Self {
        let temp = TestDir::new(name);
        let storage_dir = temp.path().join("state");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = AppServer::with_storage_dir(&storage_dir);
        let auth_token = format!("token-{}", unique_suffix());
        let config = HttpServerConfig {
            auth_token: auth_token.clone(),
        };

        thread::spawn(move || {
            serve_http_listener(listener, server, config).expect("real app-server should serve");
        });

        let harness = Self {
            temp,
            server_url: format!("http://{address}"),
            auth_token,
        };
        harness.wait_until_ready();
        harness
    }

    fn wait_until_ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if health_check(&self.server_url).unwrap_or(false) {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }

        panic!("real app-server did not become ready");
    }

    fn write_json(&self, name: &str, value: &Value) -> PathBuf {
        let path = self.temp.path().join(name);
        fs::write(&path, serde_json::to_string(value).unwrap()).unwrap();
        path
    }

    fn run_jsons<const N: usize>(&self, args: [String; N]) -> Vec<Value> {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        parse_json_documents(&String::from_utf8_lossy(&output.stdout))
    }

    fn run<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(env!("CARGO_BIN_EXE_manual"));
        command
            .arg("--server-url")
            .arg(&self.server_url)
            .arg("--auth-token")
            .arg(&self.auth_token);
        for arg in args {
            command.arg(arg);
        }
        command.output().unwrap()
    }

    fn wait_for_workflow_completion(&self, run_id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let events = self.run_jsons([
                "workflow".into(),
                "events".into(),
                run_id.to_owned(),
                "--cursor".into(),
                "0".into(),
            ]);
            if events[0]["completed"] == true {
                return events[0].clone();
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("workflow {run_id} did not complete in time");
    }

    fn wait_for_workflow_pause(&self, run_id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let events = self.run_jsons([
                "workflow".into(),
                "events".into(),
                run_id.to_owned(),
                "--cursor".into(),
                "0".into(),
            ]);
            if events[0]["run"]["paused"] == true {
                return events[0].clone();
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("workflow {run_id} did not pause in time");
    }

    fn wait_for_node_completion(&self, run_id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let events = self.run_jsons([
                "node".into(),
                "run-events".into(),
                run_id.to_owned(),
                "--cursor".into(),
                "0".into(),
            ]);
            if events[0]["completed"] == true {
                return events[0].clone();
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("node run {run_id} did not complete in time");
    }
}

fn parse_json_documents(stdout: &str) -> Vec<Value> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    serde_json::Deserializer::from_str(trimmed)
        .into_iter::<Value>()
        .map(|value| value.unwrap())
        .collect()
}

fn health_check(server_url: &str) -> std::io::Result<bool> {
    let address = server_url.strip_prefix("http://").unwrap();
    let mut stream = TcpStream::connect(address)?;
    let request = format!("GET /health HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response.starts_with("HTTP/1.1 200 OK"))
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{name}-{}", unique_suffix()));
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
