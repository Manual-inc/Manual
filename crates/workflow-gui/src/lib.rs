use std::fmt;

use eframe::egui::{self, Color32, RichText, Ui};
use manual_graph_viewer::{Edge as GraphEdge, Node as GraphNode, circular_layout};
use node::{Node, NodeIdError, NodeKind};
use workflow::{Workflow, WorkflowEdge, WorkflowEdgeKind, WorkflowError};
use workflow_registry::{WorkflowId, WorkflowRegistry, WorkflowRegistryError};

pub use manual_graph_viewer::{Graph, GraphLayout, GraphView};

pub type NativeRunResult = eframe::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowPage {
    List,
    Detail,
    Create,
    Edit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowGuiEvent {
    StartedCreate,
    OpenedDetail(String),
    StartedEdit(String),
    Created(String),
    Updated(String),
    Deleted(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSummary {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub entry_node: String,
    pub node_count: usize,
    pub edge_count: usize,
}

impl WorkflowSummary {
    fn from_workflow(workflow: &Workflow) -> Self {
        Self {
            id: workflow.id().to_string(),
            name: workflow.name().to_string(),
            goal: workflow.goal().to_string(),
            entry_node: workflow.entry_node().as_str().to_string(),
            node_count: workflow.nodes().len(),
            edge_count: workflow.edges().len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDraft {
    pub id: String,
    pub name: String,
    pub goal: String,
    pub entry_node: String,
    pub nodes: Vec<NodeDraft>,
    pub edges: Vec<EdgeDraft>,
}

impl WorkflowDraft {
    pub fn blank() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            goal: String::new(),
            entry_node: "trigger".to_string(),
            nodes: vec![NodeDraft {
                id: "trigger".to_string(),
                kind: NodeKind::Trigger,
                description: "Receive the request.".to_string(),
                ..Default::default()
            }],
            edges: Vec::new(),
        }
    }

    pub fn from_workflow(workflow: &Workflow) -> Self {
        Self {
            id: workflow.id().to_string(),
            name: workflow.name().to_string(),
            goal: workflow.goal().to_string(),
            entry_node: workflow.entry_node().as_str().to_string(),
            nodes: workflow.nodes().iter().map(NodeDraft::from_node).collect(),
            edges: workflow.edges().iter().map(EdgeDraft::from_edge).collect(),
        }
    }

    pub fn build_workflow(&self) -> Result<Workflow, WorkflowGuiError> {
        let nodes = self
            .nodes
            .iter()
            .map(NodeDraft::build_node)
            .collect::<Result<Vec<_>, _>>()?;
        let edges = self
            .edges
            .iter()
            .map(EdgeDraft::build_edge)
            .collect::<Result<Vec<_>, _>>()?;

        Workflow::new(
            self.id.trim().to_string(),
            self.name.trim().to_string(),
            self.goal.trim().to_string(),
            self.entry_node.trim().to_string(),
            nodes,
            edges,
        )
        .map_err(WorkflowGuiError::Workflow)
    }
}

impl Default for WorkflowDraft {
    fn default() -> Self {
        Self::blank()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDraft {
    pub id: String,
    pub kind: NodeKind,
    pub description: String,
    pub inputs: String,
    pub outputs: String,
    pub sandbox: String,
    pub runtime: String,
    pub artifacts: String,
    pub acceptance: String,
}

impl NodeDraft {
    fn from_node(node: &Node) -> Self {
        Self {
            id: node.id.as_str().to_string(),
            kind: node.kind,
            description: node.description.clone(),
            inputs: join_lines(&node.contract.inputs),
            outputs: join_lines(&node.contract.outputs),
            sandbox: node.contract.sandbox.clone().unwrap_or_default(),
            runtime: node.contract.runtime.clone().unwrap_or_default(),
            artifacts: join_lines(&node.contract.artifacts),
            acceptance: node.contract.acceptance.clone().unwrap_or_default(),
        }
    }

    fn build_node(&self) -> Result<Node, WorkflowGuiError> {
        let mut node = Node::new(self.id.trim(), self.kind, self.description.trim())
            .map_err(WorkflowGuiError::NodeId)?;

        node.contract.inputs = split_lines(&self.inputs);
        node.contract.outputs = split_lines(&self.outputs);
        node.contract.sandbox = optional_text(&self.sandbox);
        node.contract.runtime = optional_text(&self.runtime);
        node.contract.artifacts = split_lines(&self.artifacts);
        node.contract.acceptance = optional_text(&self.acceptance);

        Ok(node)
    }
}

impl Default for NodeDraft {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: NodeKind::LlmTask,
            description: String::new(),
            inputs: String::new(),
            outputs: String::new(),
            sandbox: String::new(),
            runtime: String::new(),
            artifacts: String::new(),
            acceptance: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeDraft {
    pub source: String,
    pub target: String,
    pub kind: WorkflowEdgeKind,
    pub label: String,
}

impl EdgeDraft {
    fn from_edge(edge: &WorkflowEdge) -> Self {
        Self {
            source: edge.source().as_str().to_string(),
            target: edge.target().as_str().to_string(),
            kind: edge.kind(),
            label: edge.label().unwrap_or_default().to_string(),
        }
    }

    fn build_edge(&self) -> Result<WorkflowEdge, WorkflowGuiError> {
        let edge = WorkflowEdge::try_new(self.kind, self.source.trim(), self.target.trim())
            .map_err(WorkflowGuiError::NodeId)?;

        Ok(match optional_text(&self.label) {
            Some(label) => edge.with_label(label),
            None => edge,
        })
    }
}

impl Default for EdgeDraft {
    fn default() -> Self {
        Self {
            source: String::new(),
            target: String::new(),
            kind: WorkflowEdgeKind::Sequence,
            label: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowGuiError {
    UnknownWorkflow(String),
    NoSelection,
    MissingDraft,
    NodeId(NodeIdError),
    Workflow(WorkflowError),
    Registry(WorkflowRegistryError),
}

impl fmt::Display for WorkflowGuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownWorkflow(id) => write!(f, "unknown workflow: {id}"),
            Self::NoSelection => write!(f, "no workflow is selected"),
            Self::MissingDraft => write!(f, "no workflow draft is open"),
            Self::NodeId(error) => write!(f, "{error}"),
            Self::Workflow(error) => write!(f, "{error}"),
            Self::Registry(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for WorkflowGuiError {}

#[derive(Debug, Clone)]
pub struct WorkflowGui {
    registry: WorkflowRegistry,
    page: WorkflowPage,
    selected_workflow_id: Option<WorkflowId>,
    editing_original_id: Option<WorkflowId>,
    draft: Option<WorkflowDraft>,
    graph_view: GraphView,
    last_error: Option<String>,
}

impl WorkflowGui {
    pub fn new(registry: WorkflowRegistry) -> Self {
        Self {
            registry,
            page: WorkflowPage::List,
            selected_workflow_id: None,
            editing_original_id: None,
            draft: None,
            graph_view: GraphView::default(),
            last_error: None,
        }
    }

    pub fn registry(&self) -> &WorkflowRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut WorkflowRegistry {
        &mut self.registry
    }

    pub fn page(&self) -> WorkflowPage {
        self.page
    }

    pub fn draft(&self) -> Option<&WorkflowDraft> {
        self.draft.as_ref()
    }

    pub fn draft_mut(&mut self) -> Option<&mut WorkflowDraft> {
        self.draft.as_mut()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn workflow_summaries(&self) -> Vec<WorkflowSummary> {
        self.registry
            .iter()
            .map(WorkflowSummary::from_workflow)
            .collect()
    }

    pub fn selected_workflow(&self) -> Option<&Workflow> {
        self.selected_workflow_id
            .as_ref()
            .and_then(|id| self.registry.get(id))
    }

    pub fn open_detail(
        &mut self,
        id: impl Into<String>,
    ) -> Result<WorkflowGuiEvent, WorkflowGuiError> {
        let id = WorkflowId::new(id.into())
            .map_err(WorkflowRegistryError::InvalidId)
            .map_err(WorkflowGuiError::Registry)?;
        if self.registry.get(&id).is_none() {
            return Err(WorkflowGuiError::UnknownWorkflow(id.into_string()));
        }

        let id_text = id.as_str().to_string();
        self.selected_workflow_id = Some(id);
        self.editing_original_id = None;
        self.draft = None;
        self.page = WorkflowPage::Detail;
        self.last_error = None;

        Ok(WorkflowGuiEvent::OpenedDetail(id_text))
    }

    pub fn start_create(&mut self) -> WorkflowGuiEvent {
        self.page = WorkflowPage::Create;
        self.selected_workflow_id = None;
        self.editing_original_id = None;
        self.draft = Some(WorkflowDraft::blank());
        self.last_error = None;

        WorkflowGuiEvent::StartedCreate
    }

    pub fn start_edit_selected(&mut self) -> Result<WorkflowGuiEvent, WorkflowGuiError> {
        let selected_id = self
            .selected_workflow_id
            .clone()
            .ok_or(WorkflowGuiError::NoSelection)?;
        let workflow = self
            .registry
            .get(&selected_id)
            .ok_or_else(|| WorkflowGuiError::UnknownWorkflow(selected_id.as_str().to_string()))?;

        self.draft = Some(WorkflowDraft::from_workflow(workflow));
        self.editing_original_id = Some(selected_id.clone());
        self.page = WorkflowPage::Edit;
        self.last_error = None;

        Ok(WorkflowGuiEvent::StartedEdit(selected_id.into_string()))
    }

    pub fn cancel_form(&mut self) -> WorkflowGuiEvent {
        self.draft = None;
        self.editing_original_id = None;
        self.last_error = None;
        self.page = if self.selected_workflow().is_some() {
            WorkflowPage::Detail
        } else {
            WorkflowPage::List
        };

        WorkflowGuiEvent::Cancelled
    }

    pub fn save_draft(&mut self) -> Result<WorkflowGuiEvent, WorkflowGuiError> {
        let draft = self.draft.as_ref().ok_or(WorkflowGuiError::MissingDraft)?;
        let workflow = draft.build_workflow()?;
        let workflow_id = WorkflowId::new(workflow.id())
            .map_err(WorkflowRegistryError::InvalidId)
            .map_err(WorkflowGuiError::Registry)?;
        let workflow_id_text = workflow_id.as_str().to_string();

        let mut next_registry = self.registry.clone();
        let event = match self.page {
            WorkflowPage::Create => {
                next_registry
                    .insert(workflow)
                    .map_err(WorkflowGuiError::Registry)?;
                WorkflowGuiEvent::Created(workflow_id_text.clone())
            }
            WorkflowPage::Edit => {
                let original_id = self
                    .editing_original_id
                    .clone()
                    .ok_or(WorkflowGuiError::NoSelection)?;
                if next_registry.remove(&original_id).is_none() {
                    return Err(WorkflowGuiError::UnknownWorkflow(
                        original_id.as_str().to_string(),
                    ));
                }
                next_registry
                    .insert(workflow)
                    .map_err(WorkflowGuiError::Registry)?;
                WorkflowGuiEvent::Updated(workflow_id_text.clone())
            }
            _ => return Err(WorkflowGuiError::MissingDraft),
        };

        self.registry = next_registry;
        self.selected_workflow_id = Some(workflow_id);
        self.editing_original_id = None;
        self.draft = None;
        self.page = WorkflowPage::Detail;
        self.last_error = None;

        Ok(event)
    }

    pub fn delete_selected(&mut self) -> Result<WorkflowGuiEvent, WorkflowGuiError> {
        let selected_id = self
            .selected_workflow_id
            .clone()
            .ok_or(WorkflowGuiError::NoSelection)?;
        let removed = self.registry.remove(&selected_id);

        if removed.is_none() {
            return Err(WorkflowGuiError::UnknownWorkflow(
                selected_id.as_str().to_string(),
            ));
        }

        let id_text = selected_id.into_string();
        self.selected_workflow_id = None;
        self.editing_original_id = None;
        self.draft = None;
        self.page = WorkflowPage::List;
        self.last_error = None;

        Ok(WorkflowGuiEvent::Deleted(id_text))
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<WorkflowGuiEvent> {
        apply_visuals(ui);

        ui.horizontal(|ui| {
            ui.set_min_height(560.0);
            let navigation_event = self.draw_navigation(ui);
            ui.separator();
            let page_event = self.draw_page(ui);
            page_event.or(navigation_event)
        })
        .inner
    }

    fn draw_navigation(&mut self, ui: &mut Ui) -> Option<WorkflowGuiEvent> {
        let mut event = None;

        ui.vertical(|ui| {
            ui.set_width(260.0);
            ui.heading("Workflows");
            ui.add_space(8.0);

            if ui.button("New workflow").clicked() {
                event = Some(self.start_create());
            }

            ui.add_space(10.0);
            egui::ScrollArea::vertical()
                .id_salt("workflow_gui_navigation")
                .show(ui, |ui| {
                    for summary in self.workflow_summaries() {
                        let selected = self
                            .selected_workflow_id
                            .as_ref()
                            .is_some_and(|id| id.as_str() == summary.id);
                        let row = ui.selectable_label(selected, &summary.name);
                        if row.clicked() {
                            match self.open_detail(summary.id.clone()) {
                                Ok(opened) => event = Some(opened),
                                Err(error) => self.last_error = Some(error.to_string()),
                            }
                        }
                        ui.small(format!(
                            "{} nodes, {} edges",
                            summary.node_count, summary.edge_count
                        ));
                        ui.add_space(6.0);
                    }
                });
        });

        event
    }

    fn draw_page(&mut self, ui: &mut Ui) -> Option<WorkflowGuiEvent> {
        ui.vertical(|ui| {
            ui.set_min_width(620.0);
            if let Some(error) = &self.last_error {
                ui.colored_label(Color32::from_rgb(192, 67, 52), error);
                ui.add_space(8.0);
            }

            match self.page {
                WorkflowPage::List => {
                    self.draw_list_page(ui);
                    None
                }
                WorkflowPage::Detail => self.draw_detail_page(ui),
                WorkflowPage::Create | WorkflowPage::Edit => self.draw_form_page(ui),
            }
        })
        .inner
    }

    fn draw_list_page(&self, ui: &mut Ui) {
        ui.heading("Workflow registry");
        ui.add_space(8.0);

        if self.registry.is_empty() {
            ui.label("No workflows registered.");
            return;
        }

        egui::Grid::new("workflow_gui_list_grid")
            .num_columns(5)
            .spacing([18.0, 10.0])
            .striped(true)
            .show(ui, |ui| {
                draw_header(ui, "ID");
                draw_header(ui, "Name");
                draw_header(ui, "Entry");
                draw_header(ui, "Nodes");
                draw_header(ui, "Edges");
                ui.end_row();

                for summary in self.workflow_summaries() {
                    ui.monospace(summary.id);
                    ui.label(summary.name);
                    ui.monospace(summary.entry_node);
                    ui.label(summary.node_count.to_string());
                    ui.label(summary.edge_count.to_string());
                    ui.end_row();
                }
            });
    }

    fn draw_detail_page(&mut self, ui: &mut Ui) -> Option<WorkflowGuiEvent> {
        let Some(workflow) = self.selected_workflow().cloned() else {
            ui.label("Select a workflow from the list.");
            return None;
        };

        let mut event = None;
        ui.horizontal_wrapped(|ui| {
            ui.heading(workflow.name());
            ui.add_space(12.0);
            if ui.button("Edit").clicked() {
                match self.start_edit_selected() {
                    Ok(started) => event = Some(started),
                    Err(error) => self.last_error = Some(error.to_string()),
                }
            }
            if ui.button("Delete").clicked() {
                match self.delete_selected() {
                    Ok(deleted) => event = Some(deleted),
                    Err(error) => self.last_error = Some(error.to_string()),
                }
            }
        });
        ui.add_space(6.0);
        ui.monospace(workflow.id());
        ui.label(workflow.goal());
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(270.0);
                ui.heading("Nodes");
                egui::ScrollArea::vertical()
                    .id_salt("workflow_gui_node_details")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for node in workflow.nodes() {
                            egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.monospace(node.id.as_str());
                                        ui.label(
                                            RichText::new(node.kind.as_str())
                                                .color(Color32::from_rgb(57, 108, 184)),
                                        );
                                    });
                                    ui.small(&node.description);
                                });
                            ui.add_space(6.0);
                        }
                    });

                ui.add_space(10.0);
                ui.heading("Edges");
                for edge in workflow.edges() {
                    ui.small(format!(
                        "{} -> {} ({})",
                        edge.source(),
                        edge.target(),
                        edge.kind()
                    ));
                }
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.heading("Graph");
                let graph = workflow_to_graph(&workflow);
                let layout = circular_layout(&graph);
                let available = ui.available_size();
                ui.allocate_ui_with_layout(
                    egui::vec2(available.x.max(320.0), 340.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.graph_view.ui(ui, &graph, &layout);
                    },
                );
            });
        });

        event
    }

    fn draw_form_page(&mut self, ui: &mut Ui) -> Option<WorkflowGuiEvent> {
        let title = match self.page {
            WorkflowPage::Create => "Create workflow",
            WorkflowPage::Edit => "Edit workflow",
            _ => "Workflow form",
        };
        let mut event = None;

        ui.heading(title);
        ui.add_space(8.0);

        let Some(draft) = self.draft.as_mut() else {
            ui.label("No draft is open.");
            return None;
        };

        egui::Grid::new("workflow_gui_form_identity")
            .num_columns(2)
            .spacing([14.0, 8.0])
            .show(ui, |ui| {
                ui.label("ID");
                ui.text_edit_singleline(&mut draft.id);
                ui.end_row();

                ui.label("Name");
                ui.text_edit_singleline(&mut draft.name);
                ui.end_row();

                ui.label("Entry node");
                ui.text_edit_singleline(&mut draft.entry_node);
                ui.end_row();

                ui.label("Goal");
                ui.text_edit_multiline(&mut draft.goal);
                ui.end_row();
            });

        ui.add_space(12.0);
        draw_nodes_editor(ui, draft);
        ui.add_space(12.0);
        draw_edges_editor(ui, draft);

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                match self.save_draft() {
                    Ok(saved) => event = Some(saved),
                    Err(error) => self.last_error = Some(error.to_string()),
                }
            }
            if ui.button("Cancel").clicked() {
                event = Some(self.cancel_form());
            }
        });

        event
    }
}

impl Default for WorkflowGui {
    fn default() -> Self {
        Self::new(WorkflowRegistry::new())
    }
}

pub struct WorkflowGuiApp {
    gui: WorkflowGui,
}

impl WorkflowGuiApp {
    pub fn new(registry: WorkflowRegistry) -> Self {
        Self {
            gui: WorkflowGui::new(registry),
        }
    }

    pub fn gui(&self) -> &WorkflowGui {
        &self.gui
    }

    pub fn gui_mut(&mut self) -> &mut WorkflowGui {
        &mut self.gui
    }
}

impl eframe::App for WorkflowGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("workflow_gui_toolbar")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Manual Workflow GUI");
                    ui.separator();
                    self.gui.graph_view.zoom_controls(ui);
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.gui.ui(ui);
            });
    }
}

pub fn run_native(registry: WorkflowRegistry) -> NativeRunResult {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 780.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Manual Workflow GUI",
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(WorkflowGuiApp::new(registry.clone())))),
    )
}

pub fn sample_registry() -> WorkflowRegistry {
    let mut registry = WorkflowRegistry::new();
    registry
        .insert(sample_agent_handoff_workflow())
        .expect("sample agent handoff workflow should register");
    registry
}

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

pub fn sample_agent_handoff_workflow() -> Workflow {
    let request = Node::new(
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
    let result_gate = Node::new(
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
    let final_report = Node::new(
        "final_report",
        NodeKind::Artifact,
        "Persist the approved result, summary, and audit trail.",
    )
    .expect("final report node should be valid")
    .with_input("reviewed_result")
    .with_artifact("final-report.md");
    let halted = Node::new(
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

fn draw_nodes_editor(ui: &mut Ui, draft: &mut WorkflowDraft) {
    ui.horizontal(|ui| {
        ui.heading("Nodes");
        if ui.button("Add node").clicked() {
            draft.nodes.push(NodeDraft::default());
        }
    });

    let mut remove_index = None;
    for (index, node) in draft.nodes.iter_mut().enumerate() {
        ui.push_id(("node_draft", index), |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("ID");
                        ui.text_edit_singleline(&mut node.id);
                        egui::ComboBox::from_id_salt("node_kind")
                            .selected_text(node.kind.as_str())
                            .show_ui(ui, |ui| {
                                for kind in NodeKind::ALL {
                                    ui.selectable_value(&mut node.kind, kind, kind.as_str());
                                }
                            });
                        if ui.button("Remove").clicked() {
                            remove_index = Some(index);
                        }
                    });
                    ui.label("Description");
                    ui.text_edit_multiline(&mut node.description);
                    ui.horizontal(|ui| {
                        labeled_singleline(ui, "Runtime", &mut node.runtime);
                        labeled_singleline(ui, "Sandbox", &mut node.sandbox);
                    });
                    ui.columns(3, |columns| {
                        columns[0].label("Inputs");
                        columns[0].text_edit_multiline(&mut node.inputs);
                        columns[1].label("Outputs");
                        columns[1].text_edit_multiline(&mut node.outputs);
                        columns[2].label("Artifacts");
                        columns[2].text_edit_multiline(&mut node.artifacts);
                    });
                    ui.label("Acceptance");
                    ui.text_edit_multiline(&mut node.acceptance);
                });
            ui.add_space(6.0);
        });
    }

    if let Some(index) = remove_index {
        draft.nodes.remove(index);
    }
}

fn draw_edges_editor(ui: &mut Ui, draft: &mut WorkflowDraft) {
    ui.horizontal(|ui| {
        ui.heading("Edges");
        if ui.button("Add edge").clicked() {
            draft.edges.push(EdgeDraft::default());
        }
    });

    let mut remove_index = None;
    for (index, edge) in draft.edges.iter_mut().enumerate() {
        ui.push_id(("edge_draft", index), |ui| {
            ui.horizontal_wrapped(|ui| {
                labeled_singleline(ui, "Source", &mut edge.source);
                labeled_singleline(ui, "Target", &mut edge.target);
                egui::ComboBox::from_id_salt("edge_kind")
                    .selected_text(edge.kind.as_str())
                    .show_ui(ui, |ui| {
                        for kind in WorkflowEdgeKind::ALL {
                            ui.selectable_value(&mut edge.kind, kind, kind.as_str());
                        }
                    });
                labeled_singleline(ui, "Label", &mut edge.label);
                if ui.button("Remove").clicked() {
                    remove_index = Some(index);
                }
            });
        });
    }

    if let Some(index) = remove_index {
        draft.edges.remove(index);
    }
}

fn labeled_singleline(ui: &mut Ui, label: &'static str, value: &mut String) {
    ui.label(label);
    ui.text_edit_singleline(value);
}

fn draw_header(ui: &mut Ui, label: &'static str) {
    ui.label(RichText::new(label).strong());
}

fn workflow_node_label(node: &Node, entry_node_id: &str) -> String {
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
) -> Node {
    Node::new(id, NodeKind::CodeTask, description)
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
) -> Node {
    Node::new(id, NodeKind::LlmTask, description)
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

fn split_lines(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn join_lines(values: &[String]) -> String {
    values.join("\n")
}

fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn apply_visuals(ui: &mut Ui) {
    let visuals = ui.visuals_mut();
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(232, 238, 246);
    visuals.widgets.active.bg_fill = Color32::from_rgb(212, 225, 243);
    visuals.selection.bg_fill = Color32::from_rgb(70, 124, 208);
    visuals.selection.stroke.color = Color32::WHITE;
}
