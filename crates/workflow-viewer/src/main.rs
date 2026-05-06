use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};

fn main() -> eframe::Result {
    workflow_viewer::run_native(sample_workflow())
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
        ],
    )
    .expect("sample workflow should be valid")
}
