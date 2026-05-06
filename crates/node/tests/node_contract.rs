use node::Node;
use node::NodeId;
use node::NodeIdError;
use node::NodeKind;
use node::NodeKindParseError;

#[test]
fn node_kind_uses_workflow_spec_identifiers() {
    assert_eq!(NodeKind::Trigger.as_str(), "trigger");
    assert_eq!(NodeKind::LlmTask.as_str(), "llm_task");
    assert_eq!(NodeKind::CodeTask.as_str(), "code_task");
    assert_eq!(NodeKind::Integration.as_str(), "integration");
    assert_eq!(NodeKind::Condition.as_str(), "condition");
    assert_eq!(NodeKind::Loop.as_str(), "loop");
    assert_eq!(NodeKind::Join.as_str(), "join");
    assert_eq!(NodeKind::Approval.as_str(), "approval");
    assert_eq!(NodeKind::Artifact.as_str(), "artifact");
}

#[test]
fn parses_node_kind_from_workflow_spec_identifier() {
    assert_eq!("llm_task".parse(), Ok(NodeKind::LlmTask));
    assert_eq!("code_task".parse(), Ok(NodeKind::CodeTask));
    assert_eq!(
        "unknown".parse::<NodeKind>(),
        Err(NodeKindParseError::Unknown("unknown".to_string()))
    );
}

#[test]
fn node_id_rejects_empty_or_whitespace_values() {
    assert_eq!(NodeId::new(""), Err(NodeIdError::Empty));
    assert_eq!(NodeId::new("   "), Err(NodeIdError::Empty));
    assert_eq!(
        NodeId::new("inspect step"),
        Err(NodeIdError::ContainsWhitespace("inspect step".to_string()))
    );
}

#[test]
fn node_captures_workflow_contract_metadata() {
    let node = Node::new(
        "inspect",
        NodeKind::LlmTask,
        "Inspect symptoms, logs, and likely code paths.",
    )
    .expect("node should be created")
    .with_input("voc_ticket")
    .with_output("root_cause_hypothesis")
    .with_sandbox("read-only")
    .with_runtime("codex")
    .with_artifact("inspection-notes.md")
    .with_acceptance("Likely code paths are identified.");

    assert_eq!(node.id.as_str(), "inspect");
    assert_eq!(node.kind, NodeKind::LlmTask);
    assert_eq!(
        node.description,
        "Inspect symptoms, logs, and likely code paths."
    );
    assert_eq!(node.contract.inputs, ["voc_ticket"]);
    assert_eq!(node.contract.outputs, ["root_cause_hypothesis"]);
    assert_eq!(node.contract.sandbox.as_deref(), Some("read-only"));
    assert_eq!(node.contract.runtime.as_deref(), Some("codex"));
    assert_eq!(node.contract.artifacts, ["inspection-notes.md"]);
    assert_eq!(
        node.contract.acceptance.as_deref(),
        Some("Likely code paths are identified.")
    );
}
