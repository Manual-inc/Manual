use node::{Node, NodeKind};
use workflow::{
    Workflow, WorkflowEdge, WorkflowEdgeKind, WorkflowEdgeKindParseError, WorkflowError,
};

#[test]
fn workflow_edge_kind_uses_workflow_spec_identifiers() {
    assert_eq!(WorkflowEdgeKind::Sequence.as_str(), "sequence");
    assert_eq!(WorkflowEdgeKind::Branch.as_str(), "branch");
    assert_eq!(WorkflowEdgeKind::LoopBack.as_str(), "loop_back");
    assert_eq!(WorkflowEdgeKind::Parallel.as_str(), "parallel");
    assert_eq!(WorkflowEdgeKind::Join.as_str(), "join");
    assert_eq!(WorkflowEdgeKind::Error.as_str(), "error");
}

#[test]
fn parses_workflow_edge_kind_from_workflow_spec_identifier() {
    assert_eq!("sequence".parse(), Ok(WorkflowEdgeKind::Sequence));
    assert_eq!("loop_back".parse(), Ok(WorkflowEdgeKind::LoopBack));
    assert_eq!(
        "unknown".parse::<WorkflowEdgeKind>(),
        Err(WorkflowEdgeKindParseError::Unknown("unknown".to_string()))
    );
    assert_eq!(WorkflowEdgeKind::Parallel.to_string(), "parallel");
    assert_eq!(
        "unknown"
            .parse::<WorkflowEdgeKind>()
            .unwrap_err()
            .to_string(),
        "unknown workflow edge kind: unknown"
    );
}

#[test]
fn workflow_combines_nodes_into_a_task_graph() {
    let trigger = Node::new("trigger", NodeKind::Trigger, "Receive the request.")
        .expect("trigger node should be valid");
    let inspect = Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.")
        .expect("inspect node should be valid");
    let report = Node::new("report", NodeKind::Artifact, "Write the final report.")
        .expect("report node should be valid");

    let workflow = Workflow::new(
        "debug-voc",
        "Debug VOC",
        "Find the root cause and produce a verified report.",
        "trigger",
        vec![trigger.clone(), inspect.clone(), report.clone()],
        vec![
            WorkflowEdge::sequence("trigger", "inspect"),
            WorkflowEdge::sequence("inspect", "report").with_label("summarize"),
        ],
    )
    .expect("workflow graph should be valid");

    assert_eq!(workflow.id(), "debug-voc");
    assert_eq!(workflow.name(), "Debug VOC");
    assert_eq!(
        workflow.goal(),
        "Find the root cause and produce a verified report."
    );
    assert_eq!(workflow.entry_node().as_str(), "trigger");
    assert_eq!(workflow.nodes(), [trigger, inspect, report]);
    assert_eq!(workflow.edges()[0].source().as_str(), "trigger");
    assert_eq!(workflow.edges()[0].target().as_str(), "inspect");
    assert_eq!(workflow.edges()[0].kind(), WorkflowEdgeKind::Sequence);
    assert_eq!(workflow.edges()[1].label(), Some("summarize"));
}

#[test]
fn workflow_rejects_graphs_with_missing_edge_endpoints() {
    let trigger = Node::new("trigger", NodeKind::Trigger, "Receive the request.")
        .expect("trigger node should be valid");

    let error = Workflow::new(
        "broken",
        "Broken workflow",
        "Demonstrate endpoint validation.",
        "trigger",
        vec![trigger],
        vec![WorkflowEdge::sequence("trigger", "missing")],
    )
    .expect_err("workflow should reject edges pointing to missing nodes");

    assert_eq!(
        error,
        WorkflowError::MissingEndpoint {
            edge_index: 0,
            endpoint: "target",
            node_id: "missing".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "edge 0 references missing target node: missing"
    );
}

#[test]
fn workflow_rejects_empty_graphs() {
    let error = Workflow::new(
        "empty",
        "Empty workflow",
        "Demonstrate empty graph validation.",
        "trigger",
        Vec::new(),
        Vec::new(),
    )
    .expect_err("workflow should reject empty graphs");

    assert_eq!(error, WorkflowError::EmptyGraph);
    assert_eq!(error.to_string(), "workflow must contain at least one node");
}

#[test]
fn workflow_rejects_duplicate_node_ids() {
    let first = Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.")
        .expect("first node should be valid");
    let second = Node::new("inspect", NodeKind::CodeTask, "Run a reproduction.")
        .expect("second node should be valid");

    let error = Workflow::new(
        "duplicate",
        "Duplicate workflow",
        "Demonstrate duplicate node validation.",
        "inspect",
        vec![first, second],
        Vec::new(),
    )
    .expect_err("workflow should reject duplicate node ids");

    assert_eq!(error, WorkflowError::DuplicateNode("inspect".to_string()));
    assert_eq!(error.to_string(), "duplicate node id: inspect");
}

#[test]
fn workflow_rejects_missing_entry_nodes() {
    let inspect = Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.")
        .expect("inspect node should be valid");

    let error = Workflow::new(
        "missing-entry",
        "Missing entry workflow",
        "Demonstrate entry validation.",
        "trigger",
        vec![inspect],
        Vec::new(),
    )
    .expect_err("workflow should reject missing entry nodes");

    assert_eq!(
        error,
        WorkflowError::MissingEntryNode("trigger".to_string())
    );
    assert_eq!(error.to_string(), "entry node does not exist: trigger");
}

#[test]
fn workflow_rejects_direct_self_loops() {
    let inspect = Node::new("inspect", NodeKind::Loop, "Repeat bounded inspection.")
        .expect("inspect node should be valid");

    let error = Workflow::new(
        "self-loop",
        "Self-loop workflow",
        "Demonstrate direct loop validation.",
        "inspect",
        vec![inspect],
        vec![WorkflowEdge::loop_back("inspect", "inspect")],
    )
    .expect_err("workflow should reject direct self-loops");

    assert_eq!(
        error,
        WorkflowError::SelfLoop {
            edge_index: 0,
            node_id: "inspect".to_string(),
        }
    );
    assert_eq!(error.to_string(), "edge 0 cannot point inspect to itself");
}
