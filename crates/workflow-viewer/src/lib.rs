use eframe::egui::{self, Color32, Ui};
use manual_graph_viewer::{Edge as GraphEdge, Node as GraphNode, circular_layout};
use node::{Node as WorkflowNode, NodeKind};
use workflow::{Workflow, WorkflowEdge};

pub use manual_graph_viewer::{Graph, GraphLayout, GraphView};

pub fn workflow_to_graph(workflow: &Workflow) -> Graph {
    let entry_node_id = workflow.entry_node().as_str();
    let nodes = workflow
        .nodes()
        .iter()
        .map(|node| GraphNode {
            id: node.id.as_str().to_string(),
            label: workflow_node_label(node, entry_node_id),
            color: Some(color_for_node_kind(node.kind).to_string()),
        })
        .collect();
    let edges = workflow
        .edges()
        .iter()
        .map(|edge| GraphEdge {
            source: edge.source().as_str().to_string(),
            target: edge.target().as_str().to_string(),
            label: Some(workflow_edge_label(edge)),
        })
        .collect();

    Graph::new(nodes, edges).expect("validated workflow should convert into a valid graph")
}

pub fn color_for_node_kind(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Trigger => "#2f9e44",
        NodeKind::LlmTask => "#4f8cff",
        NodeKind::CodeTask => "#26a69a",
        NodeKind::Integration => "#8b6fd6",
        NodeKind::Condition => "#e0a33a",
        NodeKind::Loop => "#d6723f",
        NodeKind::Join => "#8590a6",
        NodeKind::Approval => "#d45d79",
        NodeKind::Artifact => "#64748b",
    }
}

pub struct WorkflowViewerApp {
    workflow: Workflow,
    graph: Graph,
    layout: GraphLayout,
    status: String,
    view: GraphView,
}

impl WorkflowViewerApp {
    pub fn new(workflow: Workflow) -> Self {
        let graph = workflow_to_graph(&workflow);
        let layout = circular_layout(&graph);
        let status = workflow_status(&workflow);

        Self {
            workflow,
            graph,
            layout,
            status,
            view: GraphView::default(),
        }
    }

    pub fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn layout(&self) -> &GraphLayout {
        &self.layout
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    fn draw_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Manual Workflow Viewer");
            ui.separator();
            ui.strong(self.workflow.name());
            ui.separator();
            self.view.zoom_controls(ui);
            ui.separator();
            ui.colored_label(Color32::from_rgb(90, 145, 94), &self.status);
        });
    }

    fn draw_workflow(&mut self, ui: &mut Ui) {
        self.view.ui(ui, &self.graph, &self.layout);
    }
}

impl eframe::App for WorkflowViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("workflow_viewer_toolbar")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                self.draw_toolbar(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.draw_workflow(ui);
            });
    }
}

pub fn run_native(workflow: Workflow) -> eframe::Result {
    let title = format!("Manual Workflow Viewer - {}", workflow.name());
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 760.0]),
        ..Default::default()
    };

    eframe::run_native(
        &title,
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(WorkflowViewerApp::new(workflow.clone())))),
    )
}

pub fn sample_agent_handoff_workflow() -> Workflow {
    let request = WorkflowNode::new(
        "request",
        NodeKind::Trigger,
        "Receive a user request and raw context bundle.",
    )
    .expect("request node should be valid")
    .with_output("raw_request");
    let filter_scope = script_node(
        "filter_scope",
        "Keep only request fields that are relevant to the workflow goal.",
        "raw_request",
        "scoped_request",
    );
    let filter_noise = script_node(
        "filter_noise",
        "Drop duplicate logs, noisy telemetry, and low-signal attachments.",
        "scoped_request",
        "filtered_request",
    );
    let redact_secrets = script_node(
        "redact_secrets",
        "Remove secrets and protected metadata before any agent sees the input.",
        "filtered_request",
        "safe_request",
    );
    let normalize_payload = script_node(
        "normalize_payload",
        "Normalize the request into the structured schema expected by agents.",
        "safe_request",
        "normalized_payload",
    );
    let build_context = script_node(
        "build_context",
        "Build the compact context package and acceptance checklist.",
        "normalized_payload",
        "agent_context",
    );
    let routing_agent = agent_node(
        "routing_agent",
        "Interpret the request, choose the work route, and return an ok/error decision.",
        "agent_context",
        "routing_result",
    );
    let result_gate = WorkflowNode::new(
        "result_gate",
        NodeKind::Condition,
        "Route ok results to the execution agent and stop immediately on error.",
    )
    .expect("result gate node should be valid")
    .with_input("routing_result")
    .with_output("ok_or_error");
    let execution_agent = agent_node(
        "execution_agent",
        "Perform the requested work using the prepared context and route decision.",
        "routing_result",
        "work_result",
    );
    let review_agent = agent_node(
        "review_agent",
        "Review the work result against acceptance criteria before finalizing.",
        "work_result",
        "reviewed_result",
    );
    let final_report = WorkflowNode::new(
        "final_report",
        NodeKind::Artifact,
        "Persist the approved result, summary, and audit trail.",
    )
    .expect("final report node should be valid")
    .with_input("reviewed_result")
    .with_artifact("final-report.md");
    let halted = WorkflowNode::new(
        "halted",
        NodeKind::Artifact,
        "Stop the workflow and persist the routing error for inspection.",
    )
    .expect("halted node should be valid")
    .with_input("routing_result")
    .with_artifact("halt-reason.md");

    Workflow::new(
        "agent-handoff-pipeline",
        "Agent Handoff Pipeline",
        "Filter and preprocess a request with scripts, let an agent decide whether the work is safe to continue, hand off ok results to the next agents, and stop on errors.",
        "request",
        vec![
            request,
            filter_scope,
            filter_noise,
            redact_secrets,
            normalize_payload,
            build_context,
            routing_agent,
            result_gate,
            execution_agent,
            review_agent,
            final_report,
            halted,
        ],
        vec![
            WorkflowEdge::sequence("request", "filter_scope").with_label("raw input"),
            WorkflowEdge::sequence("filter_scope", "filter_noise").with_label("in scope"),
            WorkflowEdge::sequence("filter_noise", "redact_secrets").with_label("filtered"),
            WorkflowEdge::sequence("redact_secrets", "normalize_payload").with_label("safe"),
            WorkflowEdge::sequence("normalize_payload", "build_context").with_label("normalized"),
            WorkflowEdge::sequence("build_context", "routing_agent").with_label("agent context"),
            WorkflowEdge::sequence("routing_agent", "result_gate").with_label("routing result"),
            WorkflowEdge::branch("result_gate", "execution_agent").with_label("ok"),
            WorkflowEdge::error("result_gate", "halted").with_label("error: halt"),
            WorkflowEdge::sequence("execution_agent", "review_agent").with_label("handoff"),
            WorkflowEdge::sequence("review_agent", "final_report").with_label("approved"),
        ],
    )
    .expect("sample agent handoff workflow should be valid")
}

fn workflow_node_label(node: &WorkflowNode, entry_node_id: &str) -> String {
    if node.id.as_str() == entry_node_id {
        format!("{} [entry, {}]", node.id.as_str(), node.kind.as_str())
    } else {
        format!("{} [{}]", node.id.as_str(), node.kind.as_str())
    }
}

fn script_node(
    id: &'static str,
    description: &'static str,
    input: &'static str,
    output: &'static str,
) -> WorkflowNode {
    WorkflowNode::new(id, NodeKind::CodeTask, description)
        .expect("script node should be valid")
        .with_runtime("script")
        .with_input(input)
        .with_output(output)
        .with_acceptance("Script exits successfully and emits valid structured JSON.")
}

fn agent_node(
    id: &'static str,
    description: &'static str,
    input: &'static str,
    output: &'static str,
) -> WorkflowNode {
    WorkflowNode::new(id, NodeKind::LlmTask, description)
        .expect("agent node should be valid")
        .with_runtime("agent")
        .with_input(input)
        .with_output(output)
        .with_acceptance("Agent response satisfies the node contract.")
}

fn workflow_edge_label(edge: &WorkflowEdge) -> String {
    match edge.label() {
        Some(label) if !label.trim().is_empty() => {
            format!("{} ({})", label, edge.kind().as_str())
        }
        _ => edge.kind().as_str().to_string(),
    }
}

fn workflow_status(workflow: &Workflow) -> String {
    format!(
        "{}: {} workflow nodes, {} edges",
        workflow.id(),
        workflow.nodes().len(),
        workflow.edges().len()
    )
}
