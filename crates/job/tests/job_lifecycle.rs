use job::{Job, JobError, JobStatus, NodeRunStatus};
use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};

#[test]
fn jobs_instantiate_from_workflow_templates_independently() {
    let workflow = sample_workflow();

    let first = Job::new("job-001", &workflow, r#"{"ticket":"VOC-1"}"#)
        .expect("first job should be created");
    let second = Job::new("job-002", &workflow, r#"{"ticket":"VOC-2"}"#)
        .expect("second job should be created");

    assert_eq!(first.id().as_str(), "job-001");
    assert_eq!(second.id().as_str(), "job-002");
    assert_eq!(first.workflow_id(), "debug-voc");
    assert_eq!(second.workflow_id(), "debug-voc");
    assert_eq!(first.input_json(), r#"{"ticket":"VOC-1"}"#);
    assert_eq!(second.input_json(), r#"{"ticket":"VOC-2"}"#);
    assert_eq!(first.status(), JobStatus::Created);
    assert_eq!(second.status(), JobStatus::Created);

    assert_eq!(first.nodes().len(), workflow.nodes().len());
    assert_eq!(first.node_status("trigger"), Some(NodeRunStatus::Ready));
    assert_eq!(first.node_status("inspect"), Some(NodeRunStatus::Pending));
    assert_eq!(first.node_status("report"), Some(NodeRunStatus::Pending));
}

#[test]
fn job_records_node_execution_state_for_one_workflow_run() {
    let workflow = sample_workflow();
    let mut job =
        Job::new("job-001", &workflow, r#"{"ticket":"VOC-1"}"#).expect("job should be created");

    job.start_node("trigger")
        .expect("entry node should be ready to start");

    assert_eq!(job.status(), JobStatus::Running);
    assert_eq!(job.node_status("trigger"), Some(NodeRunStatus::Running));

    job.succeed_node("trigger")
        .expect("running node should be able to succeed");
    job.mark_node_ready("inspect")
        .expect("next node should become ready");
    job.start_node("inspect")
        .expect("ready node should be able to start");
    job.succeed_node("inspect")
        .expect("running node should be able to succeed");
    job.mark_node_ready("report")
        .expect("final node should become ready");
    job.start_node("report")
        .expect("ready node should be able to start");
    job.succeed_node("report")
        .expect("running node should be able to succeed");

    assert_eq!(job.status(), JobStatus::Succeeded);
    assert_eq!(job.node_status("trigger"), Some(NodeRunStatus::Succeeded));
    assert_eq!(job.node_status("inspect"), Some(NodeRunStatus::Succeeded));
    assert_eq!(job.node_status("report"), Some(NodeRunStatus::Succeeded));
}

#[test]
fn job_rejects_unknown_nodes_and_invalid_transitions() {
    let workflow = sample_workflow();
    let mut job =
        Job::new("job-001", &workflow, r#"{"ticket":"VOC-1"}"#).expect("job should be created");

    let error = job
        .start_node("missing")
        .expect_err("job should reject unknown node ids");

    assert_eq!(error, JobError::UnknownNode("missing".to_string()));
    assert_eq!(error.to_string(), "job references unknown node: missing");

    let error = job
        .succeed_node("inspect")
        .expect_err("pending node should not be able to succeed");

    assert_eq!(
        error,
        JobError::InvalidNodeTransition {
            node_id: "inspect".to_string(),
            from: NodeRunStatus::Pending,
            to: NodeRunStatus::Succeeded,
        }
    );
    assert_eq!(
        error.to_string(),
        "node inspect cannot transition from pending to succeeded"
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
