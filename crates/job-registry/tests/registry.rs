use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use job::{Job, JobStatus, NodeRunStatus};
use job_registry::{FileJobStore, JobRegistry, JobRegistryError, JobStore};
use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};

#[test]
fn registry_resolves_job_by_id() {
    let workflow = sample_workflow();
    let job = Job::new("job-001", &workflow, r#"{"ticket":"VOC-1"}"#).expect("job should be valid");
    let mut registry = JobRegistry::new();

    registry
        .insert(job.clone())
        .expect("job should be inserted");

    let resolved = registry.resolve("job-001").expect("job should resolve");
    assert_eq!(resolved, &job);
    assert_eq!(resolved.workflow_id(), "debug-voc");
    assert_eq!(resolved.status(), JobStatus::Created);
}

#[test]
fn registry_rejects_duplicate_job_ids() {
    let workflow = sample_workflow();
    let job = Job::new("job-001", &workflow, r#"{"ticket":"VOC-1"}"#).expect("job should be valid");
    let duplicate = Job::new("job-001", &workflow, r#"{"ticket":"VOC-2"}"#)
        .expect("duplicate job should be valid before registry insert");
    let mut registry = JobRegistry::new();

    registry.insert(job).expect("first job should be inserted");

    let error = registry
        .insert(duplicate)
        .expect_err("registry should reject duplicate job ids");

    assert!(matches!(
        error,
        JobRegistryError::DuplicateId(ref id) if id.as_str() == "job-001"
    ));
    assert_eq!(error.to_string(), "duplicate job id: job-001");
}

#[test]
fn registry_reports_unknown_job_ids() {
    let registry = JobRegistry::new();

    let error = registry
        .resolve("missing")
        .expect_err("registry should report missing jobs");

    assert!(matches!(
        error,
        JobRegistryError::UnknownId(ref id) if id.as_str() == "missing"
    ));
    assert_eq!(error.to_string(), "unknown job id: missing");
}

#[test]
fn registry_lists_jobs_for_a_workflow_template() {
    let workflow = sample_workflow();
    let other_workflow = Workflow::new(
        "release-notes",
        "Release notes",
        "Summarize merged changes.",
        "trigger",
        vec![
            Node::new("trigger", NodeKind::Trigger, "Receive release inputs.")
                .expect("trigger node should be valid"),
        ],
        Vec::new(),
    )
    .expect("other workflow should be valid");
    let mut registry = JobRegistry::new();

    registry
        .insert(Job::new("job-001", &workflow, r#"{"ticket":"VOC-1"}"#).unwrap())
        .unwrap();
    registry
        .insert(Job::new("job-002", &workflow, r#"{"ticket":"VOC-2"}"#).unwrap())
        .unwrap();
    registry
        .insert(Job::new("job-003", &other_workflow, r#"{"release":"1.0"}"#).unwrap())
        .unwrap();

    let jobs = registry.jobs_for_workflow("debug-voc").collect::<Vec<_>>();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].id().as_str(), "job-001");
    assert_eq!(jobs[1].id().as_str(), "job-002");
}

#[test]
fn file_store_reloads_saved_registry_from_disk() {
    let temp_dir = unique_temp_dir("reloads-saved-registry");
    let store_path = temp_dir.join(".manual").join("jobs.toml");
    let store = FileJobStore::new(&store_path);
    let workflow = sample_workflow();
    let mut job =
        Job::new("job-001", &workflow, r#"{"ticket":"VOC-1"}"#).expect("job should be valid");
    let mut registry = JobRegistry::new();

    job.start_node("trigger")
        .expect("entry node should start before save");
    registry.insert(job).expect("job should be inserted");

    store.save(&registry).expect("registry should save");

    let reloaded_store = FileJobStore::new(&store_path);
    let reloaded = reloaded_store.load().expect("registry should reload");
    let resolved = reloaded
        .resolve("job-001")
        .expect("saved job should resolve");

    assert_eq!(resolved.workflow_id(), "debug-voc");
    assert_eq!(resolved.input_json(), r#"{"ticket":"VOC-1"}"#);
    assert_eq!(resolved.status(), JobStatus::Running);
    assert_eq!(
        resolved.node_status("trigger"),
        Some(NodeRunStatus::Running)
    );
    assert_eq!(resolved.nodes()[0].attempts(), 1);

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_loads_predefined_jobs_toml_fixture() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("jobs.toml");

    let registry = FileJobStore::new(&fixture_path)
        .load()
        .expect("fixture should load");

    assert_eq!(registry.len(), 2);

    let running = registry
        .resolve("job-fixture-running")
        .expect("running fixture job should resolve");
    assert_eq!(running.workflow_id(), "debug-voc");
    assert_eq!(running.input_json(), r#"{"ticket":"VOC-42"}"#);
    assert_eq!(running.status(), JobStatus::Running);
    assert_eq!(
        running.node_status("trigger"),
        Some(NodeRunStatus::Succeeded)
    );
    assert_eq!(running.node_status("inspect"), Some(NodeRunStatus::Running));
    assert_eq!(running.node_status("report"), Some(NodeRunStatus::Pending));
    assert_eq!(running.nodes()[0].attempts(), 1);
    assert_eq!(running.nodes()[1].attempts(), 2);

    let created = registry
        .resolve("job-fixture-created")
        .expect("created fixture job should resolve");
    assert_eq!(created.workflow_id(), "release-notes");
    assert_eq!(created.status(), JobStatus::Created);
    assert_eq!(created.node_status("trigger"), Some(NodeRunStatus::Ready));
}

#[test]
fn file_store_loads_missing_file_as_empty_registry() {
    let temp_dir = unique_temp_dir("loads-missing-file");
    let store = FileJobStore::new(temp_dir.join(".manual").join("jobs.toml"));

    let registry = store.load().expect("missing file should load as empty");

    assert!(registry.is_empty());

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_for_repository_uses_manual_jobs_file() {
    let store = FileJobStore::for_repository("/workspace");

    assert_eq!(
        store.path(),
        std::path::Path::new("/workspace")
            .join(".manual")
            .join("jobs.toml")
    );
}

fn sample_workflow() -> Workflow {
    let trigger = Node::new("trigger", NodeKind::Trigger, "Receive the request.")
        .expect("trigger node should be valid");
    let inspect = Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.")
        .expect("inspect node should be valid");
    let report = Node::new("report", NodeKind::Artifact, "Write the final report.")
        .expect("report node should be valid");

    Workflow::new(
        "debug-voc",
        "Debug VOC",
        "Find the root cause and produce a verified report.",
        "trigger",
        vec![trigger, inspect, report],
        vec![
            WorkflowEdge::sequence("trigger", "inspect"),
            WorkflowEdge::sequence("inspect", "report"),
        ],
    )
    .expect("workflow graph should be valid")
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "manual-job-registry-{name}-{timestamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
