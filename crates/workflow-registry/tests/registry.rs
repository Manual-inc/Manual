use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};
use workflow_registry::{WorkflowId, WorkflowRegistry, WorkflowRegistryError};

#[test]
fn registry_resolves_workflow_by_id() {
    let workflow = sample_workflow("debug-voc", "Debug VOC");
    let mut registry = WorkflowRegistry::new();

    registry
        .insert(workflow.clone())
        .expect("workflow should be inserted");

    let resolved = registry
        .resolve("debug-voc")
        .expect("workflow should resolve");

    assert_eq!(resolved, &workflow);
    assert_eq!(resolved.name(), "Debug VOC");
    assert_eq!(
        resolved.goal(),
        "Find the root cause and produce a verified report."
    );
}

#[test]
fn registry_rejects_duplicate_workflow_ids() {
    let workflow = sample_workflow("debug-voc", "Debug VOC");
    let duplicate = sample_workflow("debug-voc", "Debug VOC v2");
    let mut registry = WorkflowRegistry::new();

    registry
        .insert(workflow)
        .expect("first workflow should be inserted");

    let error = registry
        .insert(duplicate)
        .expect_err("registry should reject duplicate workflow ids");

    assert!(matches!(
        error,
        WorkflowRegistryError::DuplicateId(ref id) if id.as_str() == "debug-voc"
    ));
    assert_eq!(error.to_string(), "duplicate workflow id: debug-voc");
}

#[test]
fn registry_reports_unknown_workflow_ids() {
    let registry = WorkflowRegistry::new();

    let error = registry
        .resolve("missing")
        .expect_err("registry should report missing workflows");

    assert!(matches!(
        error,
        WorkflowRegistryError::UnknownId(ref id) if id.as_str() == "missing"
    ));
    assert_eq!(error.to_string(), "unknown workflow id: missing");
}

#[test]
fn workflow_id_rejects_empty_or_whitespace_values() {
    assert!(WorkflowId::new("").is_err());
    assert!(WorkflowId::new("   ").is_err());
    assert!(WorkflowId::new("debug voc").is_err());
}

#[test]
fn registry_iterates_workflows_by_id_order() {
    let mut registry = WorkflowRegistry::new();

    registry
        .insert(sample_workflow("release-notes", "Release Notes"))
        .unwrap();
    registry
        .insert(sample_workflow("debug-voc", "Debug VOC"))
        .unwrap();

    let ids = registry.iter().map(Workflow::id).collect::<Vec<_>>();

    assert_eq!(ids, ["debug-voc", "release-notes"]);
}

fn sample_workflow(id: &str, name: &str) -> Workflow {
    let trigger = Node::new("trigger", NodeKind::Trigger, "Receive the request.")
        .expect("trigger node should be valid");
    let inspect = Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.")
        .expect("inspect node should be valid");
    let report = Node::new("report", NodeKind::Artifact, "Write the final report.")
        .expect("report node should be valid");

    Workflow::new(
        id,
        name,
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
