use job::{Job, JobStatus};
use job_registry::{JobRegistry, JobRegistryError};
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
