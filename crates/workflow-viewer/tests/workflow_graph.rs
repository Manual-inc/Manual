use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};
use workflow_viewer::{WorkflowViewerApp, color_for_node_kind, workflow_to_graph};

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
