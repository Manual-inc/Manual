use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};
use workflow_viewer::{
    WorkflowViewerApp, color_for_node_kind, sample_agent_handoff_workflow, workflow_to_graph,
};

#[test]
fn converts_workflow_nodes_into_graph_nodes_for_viewing() {
    let workflow = sample_workflow();

    let graph = workflow_to_graph(&workflow);

    assert_eq!(graph.nodes().len(), 3);
    assert_eq!(graph.nodes()[0].id, "trigger");
    assert_eq!(graph.nodes()[0].label, "trigger [entry, trigger]");
    assert_eq!(
        graph.nodes()[0].color.as_deref(),
        Some(color_for_node_kind(NodeKind::Trigger))
    );
    assert_eq!(graph.nodes()[1].id, "inspect");
    assert_eq!(graph.nodes()[1].label, "inspect [llm_task]");
    assert_eq!(
        graph.nodes()[1].color.as_deref(),
        Some(color_for_node_kind(NodeKind::LlmTask))
    );
}

#[test]
fn converts_workflow_edges_into_labeled_graph_edges() {
    let workflow = sample_workflow();

    let graph = workflow_to_graph(&workflow);

    assert_eq!(graph.edges().len(), 3);
    assert_eq!(graph.edges()[0].source, "trigger");
    assert_eq!(graph.edges()[0].target, "inspect");
    assert_eq!(graph.edges()[0].label.as_deref(), Some("sequence"));
    assert_eq!(graph.edges()[1].source, "inspect");
    assert_eq!(graph.edges()[1].target, "report");
    assert_eq!(
        graph.edges()[1].label.as_deref(),
        Some("summarize (sequence)")
    );
    assert_eq!(graph.edges()[2].source, "report");
    assert_eq!(graph.edges()[2].target, "inspect");
    assert_eq!(graph.edges()[2].label.as_deref(), Some("retry (loop_back)"));
}

#[test]
fn workflow_viewer_app_prepares_graph_layout_for_the_workflow() {
    let app = WorkflowViewerApp::new(sample_workflow());

    assert_eq!(app.workflow().id(), "debug-voc");
    assert_eq!(app.graph().nodes().len(), 3);
    assert!(app.layout().position("trigger").is_some());
    assert!(app.layout().position("inspect").is_some());
    assert_eq!(app.status(), "debug-voc: 3 workflow nodes, 3 edges");
}

#[test]
fn sample_agent_handoff_workflow_models_scripts_agents_and_error_stop() {
    let workflow = sample_agent_handoff_workflow();
    let node_ids = workflow
        .nodes()
        .iter()
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        node_ids,
        [
            "request",
            "filter_scope",
            "filter_noise",
            "redact_secrets",
            "normalize_payload",
            "build_context",
            "routing_agent",
            "result_gate",
            "execution_agent",
            "review_agent",
            "final_report",
            "halted",
        ]
    );
    assert_eq!(workflow.entry_node().as_str(), "request");
    assert_eq!(workflow.nodes()[1].kind, NodeKind::CodeTask);
    assert_eq!(
        workflow.nodes()[1].contract.runtime.as_deref(),
        Some("script")
    );
    assert_eq!(workflow.nodes()[6].kind, NodeKind::LlmTask);
    assert_eq!(
        workflow.nodes()[6].contract.runtime.as_deref(),
        Some("agent")
    );
    assert_eq!(workflow.nodes()[7].kind, NodeKind::Condition);

    let ok_edge = workflow
        .edges()
        .iter()
        .find(|edge| {
            edge.source().as_str() == "result_gate" && edge.target().as_str() == "execution_agent"
        })
        .expect("ok branch should hand off to the execution agent");
    assert_eq!(ok_edge.label(), Some("ok"));

    let error_edge = workflow
        .edges()
        .iter()
        .find(|edge| edge.source().as_str() == "result_gate" && edge.target().as_str() == "halted")
        .expect("error edge should stop the workflow");
    assert_eq!(error_edge.label(), Some("error: halt"));
}

#[test]
fn sample_agent_handoff_workflow_renders_branch_and_error_labels() {
    let graph = workflow_to_graph(&sample_agent_handoff_workflow());

    let ok_edge = graph
        .edges()
        .iter()
        .find(|edge| edge.source == "result_gate" && edge.target == "execution_agent")
        .expect("ok branch should be present in the graph");
    assert_eq!(ok_edge.label.as_deref(), Some("ok (branch)"));

    let error_edge = graph
        .edges()
        .iter()
        .find(|edge| edge.source == "result_gate" && edge.target == "halted")
        .expect("error branch should be present in the graph");
    assert_eq!(error_edge.label.as_deref(), Some("error: halt (error)"));
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
            WorkflowEdge::sequence("inspect", "report").with_label("summarize"),
            WorkflowEdge::loop_back("report", "inspect").with_label("retry"),
        ],
    )
    .expect("workflow graph should be valid")
}
