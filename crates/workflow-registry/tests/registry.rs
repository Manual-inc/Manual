use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};
use workflow_registry::{
    FileWorkflowStore, WorkflowId, WorkflowRegistry, WorkflowRegistryError, WorkflowStore,
};

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

#[test]
fn file_store_reloads_saved_registry_from_disk() {
    let temp_dir = unique_temp_dir("reloads-saved-registry");
    let store_path = temp_dir.join(".manual").join("workflows.toml");
    let store = FileWorkflowStore::new(&store_path);
    let mut registry = WorkflowRegistry::new();
    let debug = sample_workflow("debug-voc", "Debug VOC");
    let release = sample_workflow("release-notes", "Release Notes");

    registry.insert(debug.clone()).unwrap();
    registry.insert(release.clone()).unwrap();

    store.save(&registry).unwrap();

    let reloaded_store = FileWorkflowStore::new(&store_path);
    let reloaded = reloaded_store.load().unwrap();

    assert_eq!(reloaded.resolve("debug-voc").unwrap(), &debug);
    assert_eq!(reloaded.resolve("release-notes").unwrap(), &release);

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_loads_missing_file_as_empty_registry() {
    let temp_dir = unique_temp_dir("loads-missing-file");
    let store = FileWorkflowStore::new(temp_dir.join(".manual").join("workflows.toml"));

    let registry = store.load().unwrap();

    assert!(registry.is_empty());

    fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn file_store_for_repository_uses_manual_workflows_file() {
    let store = FileWorkflowStore::for_repository("/workspace/manual");

    assert_eq!(
        store.path(),
        PathBuf::from("/workspace/manual/.manual/workflows.toml")
    );
}

fn sample_workflow(id: &str, name: &str) -> Workflow {
    let trigger = Node::new("trigger", NodeKind::Trigger, "Receive the request.")
        .expect("trigger node should be valid");
    let inspect = Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.")
        .expect("inspect node should be valid")
        .with_input("repository")
        .with_output("findings")
        .with_sandbox("read-only")
        .with_runtime("codex")
        .with_artifact("analysis.md")
        .with_acceptance("Root cause is supported by evidence.");
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
            WorkflowEdge::sequence("inspect", "report").with_label("summarize"),
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
        "manual-workflow-registry-{name}-{timestamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
