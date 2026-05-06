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

fn workflow_node_label(node: &WorkflowNode, entry_node_id: &str) -> String {
    if node.id.as_str() == entry_node_id {
        format!("{} [entry, {}]", node.id.as_str(), node.kind.as_str())
    } else {
        format!("{} [{}]", node.id.as_str(), node.kind.as_str())
    }
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
