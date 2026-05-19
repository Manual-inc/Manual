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
    "starter.get",
    "starter.list",
    "starter.record",
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

    assert_eq!(expected.len(), 49);
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
    assert!(stdout.contains("Next steps"));
    assert!(stdout.contains("cargo build --manifest-path manual-rs/Cargo.toml -p app-server --bin manual-app-server"));
    assert!(stdout.contains("manual demo optimization"));
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
fn workflow_starter_auto_selects_agent_and_creates_code_review_workflow() {
    let temp = TestDir::new("manual-cli-starter");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo(&temp, "repo");
    let canonical_repo = fs::canonicalize(&repo).unwrap();

    let output = run_manual(
        &server,
        [
            "workflow".into(),
            "starter".into(),
            "code-review".into(),
            "--repo".into(),
            repo.display().to_string(),
            "--workflow-id".into(),
            "starter-review".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Workflow Starter"));
    assert!(stdout.contains("Preset: code-review"));
    assert!(stdout.contains("Workflow ID: starter-review"));
    assert!(stdout.contains("Agent: codex"));
    assert!(stdout.contains("manual workflow run starter-review --human"));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"agent.list""#));
    assert!(requests.contains(r#""method":"workflow.create""#));
    assert!(requests.contains(r#""id":"starter-review""#));
    assert!(requests.contains(r#""kind":"codex""#));
    assert!(requests.contains(&format!(r#""cwd":"{}""#, canonical_repo.display())));
    assert!(requests.contains(r#""node":"review""#));
    assert!(requests.contains(r#""depends_on":"collect_diff""#));
}

#[test]
fn workflow_starter_catalog_lists_available_presets_without_rpc() {
    let temp = TestDir::new("manual-cli-starter-catalog");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = run_manual::<Vec<String>, String>(&server, vec!["workflow".into(), "starter".into()]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Workflow Starter Catalog"));
    assert!(stdout.contains("code-review"));
    assert!(stdout.contains("change-summary"));
    assert!(stdout.contains("test-plan"));
    assert!(stdout.contains("manual workflow starter code-review --run"));
    assert!(stdout.contains("manual workflow starter change-summary --run"));
    assert!(stdout.contains("manual workflow starter test-plan --run"));
    assert!(stdout.contains("Best when: You want a correctness-focused review before trusting the change."));
    assert!(stdout.contains("You get: A concise review of bugs, regressions, risky assumptions, and missing tests."));
    assert!(stdout.contains("You get: A short human-readable change update with follow-up verification guidance."));
    assert!(stdout.contains("You get: A focused test plan covering the highest-value automated and manual checks."));
    assert!(
        !log.exists() || fs::read_to_string(&log).unwrap_or_default().trim().is_empty(),
        "starter catalog should not need an app-server RPC"
    );
}

#[test]
fn workflow_starter_catalog_recommends_change_summary_for_docs_only_changes() {
    let temp = TestDir::new("manual-cli-starter-catalog-docs");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo_with_identity(&temp, "docs-repo");
    write_file_and_commit(&repo, "docs/guide.md", "# guide\n");
    std::fs::write(repo.join("docs/guide.md"), "# guide updated\n").unwrap();

    let output = run_manual::<Vec<String>, String>(
        &server,
        vec![
            "workflow".into(),
            "starter".into(),
            "--repo".into(),
            repo.display().to_string(),
        ],
    );

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recommended now: change-summary"));
    assert!(stdout.contains("Why: Detected mostly documentation or markdown changes"));
    assert!(stdout.contains("Changed files: docs/guide.md"));
    assert!(stdout.contains("You get: A short human-readable change update with follow-up verification guidance."));
    assert!(stdout.contains("manual workflow starter change-summary --run"));
}

#[test]
fn workflow_starter_catalog_recommends_test_plan_for_code_changes_without_tests() {
    let temp = TestDir::new("manual-cli-starter-catalog-tests");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo_with_identity(&temp, "code-repo");
    write_file_and_commit(&repo, "src/lib.rs", "pub fn value() -> i32 { 1 }\n");
    std::fs::write(repo.join("src/lib.rs"), "pub fn value() -> i32 { 2 }\n").unwrap();

    let output = run_manual::<Vec<String>, String>(
        &server,
        vec![
            "workflow".into(),
            "starter".into(),
            "--repo".into(),
            repo.display().to_string(),
        ],
    );

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recommended now: test-plan"));
    assert!(stdout.contains("Why: Detected code changes without matching test updates"));
    assert!(stdout.contains("Changed files: src/lib.rs"));
    assert!(stdout.contains("You get: A focused test plan covering the highest-value automated and manual checks."));
    assert!(stdout.contains("manual workflow starter test-plan --run"));
}

#[test]
fn workflow_starter_run_without_explicit_preset_executes_recommended_change_summary() {
    let temp = TestDir::new("manual-cli-starter-run-recommended-summary");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo_with_identity(&temp, "docs-repo");
    write_file_and_commit(&repo, "docs/guide.md", "# guide\n");
    std::fs::write(repo.join("docs/guide.md"), "# guide updated\n").unwrap();

    let output = run_manual::<Vec<String>, String>(
        &server,
        vec![
            "workflow".into(),
            "starter".into(),
            "--repo".into(),
            repo.display().to_string(),
            "--run".into(),
        ],
    );

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Preset: change-summary"));
    assert!(stdout.contains("Recommended now: change-summary"));
    assert!(stdout.contains("Changed files: docs/guide.md"));
    assert!(stdout.contains("Summary Output"));
    assert!(stdout.contains("Starter Outcome"));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"agent.list""#));
    assert!(requests.contains(r#""method":"workflow.create""#));
    assert!(requests.contains(r#""id":"summary""#));
}

#[test]
fn workflow_starter_run_without_explicit_preset_executes_recommended_test_plan() {
    let temp = TestDir::new("manual-cli-starter-run-recommended-tests");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo_with_identity(&temp, "code-repo");
    write_file_and_commit(&repo, "src/lib.rs", "pub fn value() -> i32 { 1 }\n");
    std::fs::write(repo.join("src/lib.rs"), "pub fn value() -> i32 { 2 }\n").unwrap();

    let output = run_manual::<Vec<String>, String>(
        &server,
        vec![
            "workflow".into(),
            "starter".into(),
            "--repo".into(),
            repo.display().to_string(),
            "--run".into(),
        ],
    );

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Preset: test-plan"));
    assert!(stdout.contains("Recommended now: test-plan"));
    assert!(stdout.contains("Changed files: src/lib.rs"));
    assert!(stdout.contains("Test Plan Output"));
    assert!(stdout.contains("Starter Outcome"));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"agent.list""#));
    assert!(requests.contains(r#""method":"workflow.create""#));
    assert!(requests.contains(r#""id":"test_plan""#));
}

#[test]
fn workflow_starter_catalog_uses_remembered_repository_when_outside_git_repo() {
    let temp = TestDir::new("manual-cli-starter-remembered-catalog");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo_with_identity(&temp, "docs-repo");
    let canonical_repo = fs::canonicalize(&repo).unwrap();
    let discovery = temp.path().join("manual").join("app-server.json");
    write_file_and_commit(&repo, "docs/guide.md", "# guide\n");
    std::fs::write(repo.join("docs/guide.md"), "# guide updated\n").unwrap();

    let seed = run_manual_in_dir(
        &server,
        temp.path(),
        vec![
            "--discovery-file".into(),
            discovery.display().to_string(),
            "workflow".into(),
            "starter".into(),
            "--repo".into(),
            repo.display().to_string(),
        ],
    );
    assert!(seed.status.success(), "stderr: {}", String::from_utf8_lossy(&seed.stderr));

    let output = run_manual_in_dir(
        &server,
        temp.path(),
        vec![
            "--discovery-file".into(),
            discovery.display().to_string(),
            "workflow".into(),
            "starter".into(),
        ],
    );

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("Repository: {}", canonical_repo.display())));
    assert!(stdout.contains("Recommended now: change-summary"));
}

#[test]
fn workflow_starter_catalog_lists_recent_starters_from_state() {
    let temp = TestDir::new("manual-cli-starter-history");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo_with_identity(&temp, "history-repo");
    let canonical_repo = fs::canonicalize(&repo).unwrap();
    let discovery = temp.path().join("manual").join("app-server.json");
    write_file_and_commit(&repo, "src/lib.rs", "pub fn value() -> i32 { 1 }\n");
    std::fs::write(repo.join("src/lib.rs"), "pub fn value() -> i32 { 2 }\n").unwrap();

    let seed = run_manual_in_dir(
        &server,
        temp.path(),
        vec![
            "--discovery-file".into(),
            discovery.display().to_string(),
            "workflow".into(),
            "starter".into(),
            "--repo".into(),
            repo.display().to_string(),
            "--run".into(),
        ],
    );
    assert!(seed.status.success(), "stderr: {}", String::from_utf8_lossy(&seed.stderr));

    let output = run_manual_in_dir(
        &server,
        temp.path(),
        vec![
            "--discovery-file".into(),
            discovery.display().to_string(),
            "workflow".into(),
            "starter".into(),
        ],
    );

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recent starters"));
    assert!(stdout.contains("test-plan"));
    assert!(stdout.contains(&canonical_repo.display().to_string()));
    assert!(stdout.contains("Why it fit: Detected code changes without matching test updates."));
    assert!(stdout.contains("You get: A focused test plan covering the highest-value automated and manual checks."));
    assert!(stdout.contains("Last result: 1. Run unit tests"));
    assert!(stdout.contains("manual workflow run starter-history-repo-test-plan --human"));
    assert!(stdout.contains("manual workflow starter-outcome starter-history-repo-test-plan"));
    assert!(stdout.contains("manual workflow starter-outcome starter-history-repo-test-plan --copy"));
    assert!(stdout.contains("manual workflow starter-outcome --latest --copy"));
}

#[test]
fn workflow_starter_outcome_prints_stored_summary() {
    let temp = TestDir::new("manual-cli-starter-outcome");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = run_manual::<Vec<String>, String>(
        &server,
        vec![
            "workflow".into(),
            "starter-outcome".into(),
            "starter-review".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starter Outcome"));
    assert!(stdout.contains("Workflow ID: starter-review"));
    assert!(stdout.contains("Reusable command: manual workflow run starter-review --human"));
    assert!(stdout.contains("Review Output"));
    assert!(stdout.contains("Looks good overall."));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"starter.get""#));
}

#[test]
fn workflow_starter_outcome_latest_prints_most_recent_stored_summary() {
    let temp = TestDir::new("manual-cli-starter-outcome-latest");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = run_manual::<Vec<String>, String>(
        &server,
        vec![
            "workflow".into(),
            "starter-outcome".into(),
            "--latest".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starter Outcome"));
    assert!(stdout.contains("Workflow ID: starter-review"));
    assert!(stdout.contains("Review Output"));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"starter.list""#));
    assert!(!requests.contains(r#""method":"starter.get""#));
}

#[test]
fn workflow_starter_outcome_copy_writes_summary_to_configured_clipboard_command() {
    let temp = TestDir::new("manual-cli-starter-outcome-copy");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let capture = temp.path().join("clipboard.txt");
    let clipboard = fake_clipboard(&temp, &capture);

    let output = run_manual_with_env::<Vec<String>, String>(
        &server,
        vec![
            ("MANUAL_CLIPBOARD_CMD", clipboard.display().to_string()),
            ("MANUAL_CLIPBOARD_CAPTURE", capture.display().to_string()),
        ],
        vec![
            "workflow".into(),
            "starter-outcome".into(),
            "starter-review".into(),
            "--copy".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starter Outcome"));
    assert!(stdout.contains("Copied starter outcome to clipboard."));

    let copied = fs::read_to_string(capture).unwrap();
    assert!(copied.contains("Workflow ID: starter-review"));
    assert!(copied.contains("Looks good overall."));
}

#[test]
fn workflow_starter_outcome_latest_copy_writes_latest_summary_to_configured_clipboard_command() {
    let temp = TestDir::new("manual-cli-starter-outcome-latest-copy");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let capture = temp.path().join("clipboard.txt");
    let clipboard = fake_clipboard(&temp, &capture);

    let output = run_manual_with_env::<Vec<String>, String>(
        &server,
        vec![
            ("MANUAL_CLIPBOARD_CMD", clipboard.display().to_string()),
            ("MANUAL_CLIPBOARD_CAPTURE", capture.display().to_string()),
        ],
        vec![
            "workflow".into(),
            "starter-outcome".into(),
            "--latest".into(),
            "--copy".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let copied = fs::read_to_string(capture).unwrap();
    assert!(copied.contains("Workflow ID: starter-review"));
    assert!(copied.contains("Looks good overall."));
}

#[test]
fn workflow_starter_run_uses_remembered_repository_when_outside_git_repo() {
    let temp = TestDir::new("manual-cli-starter-remembered-run");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo_with_identity(&temp, "code-repo");
    let canonical_repo = fs::canonicalize(&repo).unwrap();
    let discovery = temp.path().join("manual").join("app-server.json");
    write_file_and_commit(&repo, "src/lib.rs", "pub fn value() -> i32 { 1 }\n");
    std::fs::write(repo.join("src/lib.rs"), "pub fn value() -> i32 { 2 }\n").unwrap();

    let seed = run_manual_in_dir(
        &server,
        temp.path(),
        vec![
            "--discovery-file".into(),
            discovery.display().to_string(),
            "workflow".into(),
            "starter".into(),
            "--repo".into(),
            repo.display().to_string(),
        ],
    );
    assert!(seed.status.success(), "stderr: {}", String::from_utf8_lossy(&seed.stderr));

    let output = run_manual_in_dir(
        &server,
        temp.path(),
        vec![
            "--discovery-file".into(),
            discovery.display().to_string(),
            "workflow".into(),
            "starter".into(),
            "--run".into(),
        ],
    );

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("Repository: {}", canonical_repo.display())));
    assert!(stdout.contains("Preset: test-plan"));
    assert!(stdout.contains("Recommended now: test-plan"));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"workflow.create""#));
    assert!(requests.contains(r#""id":"test_plan""#));
}

#[test]
fn workflow_starter_creates_change_summary_workflow() {
    let temp = TestDir::new("manual-cli-starter-summary");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo(&temp, "repo");
    let canonical_repo = fs::canonicalize(&repo).unwrap();

    let output = run_manual(
        &server,
        [
            "workflow".into(),
            "starter".into(),
            "change-summary".into(),
            "--repo".into(),
            repo.display().to_string(),
            "--workflow-id".into(),
            "starter-summary".into(),
            "--agent".into(),
            "codex".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Preset: change-summary"));
    assert!(stdout.contains("Workflow ID: starter-summary"));
    assert!(stdout.contains("summarize the repository changes"));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"workflow.create""#));
    assert!(requests.contains(r#""id":"starter-summary""#));
    assert!(requests.contains(r#""id":"summary""#));
    assert!(requests.contains(r#""kind":"codex""#));
    assert!(requests.contains(&format!(r#""cwd":"{}""#, canonical_repo.display())));
    assert!(requests.contains(r#""depends_on":"collect_diff""#));
    assert!(requests.contains(r#""method":"starter.record""#));
}

#[test]
fn workflow_starter_creates_test_plan_workflow_with_default_id() {
    let temp = TestDir::new("manual-cli-starter-test-plan");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo(&temp, "repo");
    let canonical_repo = fs::canonicalize(&repo).unwrap();

    let output = run_manual(
        &server,
        [
            "workflow".into(),
            "starter".into(),
            "test-plan".into(),
            "--repo".into(),
            repo.display().to_string(),
            "--agent".into(),
            "codex".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Preset: test-plan"));
    assert!(stdout.contains("Workflow ID: starter-repo-test-plan"));
    assert!(stdout.contains("outline the highest-value automated and manual checks"));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"workflow.create""#));
    assert!(requests.contains(r#""id":"starter-repo-test-plan""#));
    assert!(requests.contains(r#""id":"test_plan""#));
    assert!(requests.contains(r#""kind":"codex""#));
    assert!(requests.contains(&format!(r#""cwd":"{}""#, canonical_repo.display())));
    assert!(requests.contains(r#""depends_on":"collect_diff""#));
    assert!(requests.contains(r#""method":"starter.record""#));
}

#[test]
fn workflow_starter_run_prints_review_output_after_completion() {
    let temp = TestDir::new("manual-cli-starter-run");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);
    let repo = init_git_repo(&temp, "repo");

    let output = run_manual(
        &server,
        [
            "workflow".into(),
            "starter".into(),
            "code-review".into(),
            "--repo".into(),
            repo.display().to_string(),
            "--workflow-id".into(),
            "starter-review".into(),
            "--agent".into(),
            "codex".into(),
            "--run".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Workflow Starter"));
    assert!(stdout.contains("Started workflow run starter-review-run"));
    assert!(stdout.contains("Workflow Events"));
    assert!(stdout.contains("Review Output"));
    assert!(stdout.contains("Starter Outcome"));
    assert!(stdout.contains("Reusable command: manual workflow run starter-review --human"));
    assert!(stdout.contains("Looks good overall."));

    let requests = fs::read_to_string(log).unwrap();
    assert!(requests.contains(r#""method":"workflow.create""#));
    assert!(requests.contains(r#""method":"workflow.start""#));
    assert!(requests.contains(r#""method":"workflow.events""#));
}

#[test]
fn generic_workflow_run_human_reprints_starter_outcome_for_starter_workflow() {
    let temp = TestDir::new("manual-cli-generic-starter-run");
    let log = temp.path().join("requests.jsonl");
    let server = fake_server(&temp, &log);

    let output = run_manual::<Vec<String>, String>(
        &server,
        vec![
            "workflow".into(),
            "run".into(),
            "starter-review".into(),
            "--human".into(),
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Started workflow run starter-review-run"));
    assert!(stdout.contains("Workflow Events"));
    assert!(stdout.contains("Review Output"));
    assert!(stdout.contains("Starter Outcome"));
    assert!(stdout.contains("Reusable command: manual workflow run starter-review --human"));
    assert!(stdout.contains("Looks good overall."));
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

fn run_manual_with_env<I, S>(
    server: &Path,
    envs: Vec<(&str, String)>,
    args: I,
) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = manual_cli();
    command.arg("--server-bin").arg(server);
    for (key, value) in envs {
        command.env(key, value);
    }
    for arg in args {
        command.arg(arg);
    }
    command.output().unwrap()
}

fn run_manual_in_dir<I, S>(server: &Path, cwd: &Path, args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = manual_cli();
    command.arg("--server-bin").arg(server).current_dir(cwd);
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

fn init_git_repo(temp: &TestDir, name: &str) -> PathBuf {
    let repo = temp.path().join(name);
    fs::create_dir_all(&repo).unwrap();
    let status = Command::new("git")
        .arg("init")
        .arg("-q")
        .arg(&repo)
        .status()
        .unwrap();
    assert!(status.success());
    repo
}

fn init_git_repo_with_identity(temp: &TestDir, name: &str) -> PathBuf {
    let repo = init_git_repo(temp, name);
    let email = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["config", "user.email", "starter@example.com"])
        .status()
        .unwrap();
    assert!(email.success());
    let name = Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["config", "user.name", "Starter"])
        .status()
        .unwrap();
    assert!(name.success());
    repo
}

fn write_file_and_commit(repo: &Path, relative: &str, contents: &str) {
    let path = repo.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, contents).unwrap();
    let add = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["add", relative])
        .status()
        .unwrap();
    assert!(add.success());
    let commit = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["commit", "-q", "-m", "seed"])
        .status()
        .unwrap();
    assert!(commit.success());
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
    elif method == "workflow.start":
        workflow_id = request["params"].get("workflow_id", "workflow")
        if workflow_id == "starter-review":
            run_id = "starter-review-run"
        elif workflow_id.endswith("-summary"):
            run_id = workflow_id + "-run"
        elif workflow_id.endswith("-test-plan"):
            run_id = workflow_id + "-run"
        else:
            run_id = "run-1"
        response = {{
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {{"run_id": run_id}}
        }}
    elif method == "workflow.events":
        run_id = request["params"]["run_id"]
        if run_id == "starter-review-run":
            response = {{
                "jsonrpc": "2.0",
                "id": request["id"],
                "result": {{
                    "events": [
                        {{"sequence": request["params"]["cursor"], "type": "workflow_completed"}}
                    ],
                    "next_cursor": request["params"]["cursor"] + 1,
                    "completed": True,
                    "run": {{
                        "run_id": run_id,
                        "status": "completed",
                        "nodes": {{
                            "review": {{
                                "status": "completed",
                                "result": {{
                                    "status_code": 0,
                                    "stdout": "Looks good overall.",
                                    "stderr": ""
                                }}
                            }}
                        }}
                    }},
                    "optimization_report": {{
                        "sections": ["Token Usage", "Verification", "Time"],
                        "main_issue": "review step used most tokens",
                        "recommendations": ["trim diff context"],
                        "measurement_mode": "derived",
                        "measurement_note": "Estimated from workflow events."
                    }},
                    "optimization_analysis": {{
                        "measurement_mode": "derived",
                        "measurement_note": "Estimated from workflow events.",
                        "regression": {{
                            "possible": False,
                            "step_id": "review",
                            "reason": "stable"
                        }},
                        "bottlenecks": {{
                            "token_waste": ["review"],
                            "verification_gaps": [],
                            "slow_steps": [],
                            "unstable_tasks": []
                        }},
                        "suggestions": ["trim diff context"]
                    }},
                }},
            }}
        elif run_id.endswith("-summary-run"):
            response = {{
                "jsonrpc": "2.0",
                "id": request["id"],
                "result": {{
                    "events": [
                        {{"sequence": request["params"]["cursor"], "type": "workflow_completed"}}
                    ],
                    "next_cursor": request["params"]["cursor"] + 1,
                    "completed": True,
                    "run": {{
                        "run_id": run_id,
                        "status": "completed",
                        "nodes": {{
                            "summary": {{
                                "status": "completed",
                                "result": {{
                                    "status_code": 0,
                                    "stdout": "Changed docs and updated the reader-facing guide.",
                                    "stderr": ""
                                }}
                            }}
                        }}
                    }},
                    "optimization_report": {{
                        "sections": ["Token Usage", "Verification", "Time"],
                        "main_issue": "summary step used most tokens",
                        "recommendations": ["trim diff context"],
                        "measurement_mode": "derived",
                        "measurement_note": "Estimated from workflow events."
                    }},
                    "optimization_analysis": {{
                        "measurement_mode": "derived",
                        "measurement_note": "Estimated from workflow events.",
                        "regression": {{
                            "possible": False,
                            "step_id": "summary",
                            "reason": "stable"
                        }},
                        "bottlenecks": {{
                            "token_waste": ["summary"],
                            "verification_gaps": [],
                            "slow_steps": [],
                            "unstable_tasks": []
                        }},
                        "suggestions": ["trim diff context"]
                    }},
                }},
            }}
        elif run_id.endswith("-test-plan-run"):
            response = {{
                "jsonrpc": "2.0",
                "id": request["id"],
                "result": {{
                    "events": [
                        {{"sequence": request["params"]["cursor"], "type": "workflow_completed"}}
                    ],
                    "next_cursor": request["params"]["cursor"] + 1,
                    "completed": True,
                    "run": {{
                        "run_id": run_id,
                        "status": "completed",
                        "nodes": {{
                            "test_plan": {{
                                "status": "completed",
                                "result": {{
                                    "status_code": 0,
                                    "stdout": "1. Run unit tests\\n2. Verify docs links\\n3. Smoke the changed workflow path",
                                    "stderr": ""
                                }}
                            }}
                        }}
                    }},
                    "optimization_report": {{
                        "sections": ["Token Usage", "Verification", "Time"],
                        "main_issue": "test plan step used most tokens",
                        "recommendations": ["trim diff context"],
                        "measurement_mode": "derived",
                        "measurement_note": "Estimated from workflow events."
                    }},
                    "optimization_analysis": {{
                        "measurement_mode": "derived",
                        "measurement_note": "Estimated from workflow events.",
                        "regression": {{
                            "possible": False,
                            "step_id": "test_plan",
                            "reason": "stable"
                        }},
                        "bottlenecks": {{
                            "token_waste": ["test_plan"],
                            "verification_gaps": [],
                            "slow_steps": [],
                            "unstable_tasks": []
                        }},
                        "suggestions": ["trim diff context"]
                    }},
                }},
            }}
        else:
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
    elif method == "agent.list":
        response = {{
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {{
                "agents": [
                    {{"name": "codex", "available": True, "path": "/usr/bin/codex"}},
                    {{"name": "claude", "available": False, "path": None}},
                    {{"name": "pi", "available": False, "path": None}}
                ]
            }},
        }}
    elif method == "workflow.get":
        response = {{
            "jsonrpc": "2.0",
            "id": request["id"],
            "error": {{"code": -32000, "message": "workflow not found"}},
        }}
    elif method == "starter.get":
        workflow_id = request["params"].get("workflow_id", "starter-review")
        if workflow_id.endswith("-summary"):
            starter = {{
                "workflow_id": workflow_id,
                "preset_id": "change-summary",
                "repository_root": "/tmp/repo",
                "recommendation_reason": "Detected mostly documentation or markdown changes.",
                "outcome_label": "Summary Output",
                "outcome_text": "Changed docs and updated the reader-facing guide."
            }}
        elif workflow_id.endswith("-test-plan"):
            starter = {{
                "workflow_id": workflow_id,
                "preset_id": "test-plan",
                "repository_root": "/tmp/repo",
                "recommendation_reason": "Detected code changes without matching test updates.",
                "outcome_label": "Test Plan Output",
                "outcome_text": "1. Run unit tests\\n2. Verify docs links\\n3. Smoke the changed workflow path"
            }}
        else:
            starter = {{
                "workflow_id": workflow_id,
                "preset_id": "code-review",
                "repository_root": "/tmp/repo",
                "recommendation_reason": "Detected implementation changes that benefit from a correctness and regression review.",
                "outcome_label": "Review Output",
                "outcome_text": "Looks good overall."
            }}
        response = {{
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {{"starter": starter}}
        }}
    elif method == "starter.list":
        response = {{
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {{
                "starters": [
                    {{
                        "workflow_id": "starter-review",
                        "preset_id": "code-review",
                        "repository_root": "/tmp/repo",
                        "recommendation_reason": "Detected implementation changes that benefit from a correctness and regression review.",
                        "outcome_label": "Review Output",
                        "outcome_text": "Looks good overall."
                    }}
                ]
            }}
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

fn fake_clipboard(temp: &TestDir, capture: &Path) -> PathBuf {
    let script = temp.path().join("fake_clipboard.sh");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\ncat > \"{}\"\n",
            capture.display()
        ),
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).unwrap();
    }

    script
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
