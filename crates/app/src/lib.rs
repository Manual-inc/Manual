use eframe::egui::{self, Color32, RichText};
use job::Job;
use job_registry::JobRegistry;
use manual_core::workspace_descriptor;
use node::Node;
use sandbox_registry::{SandboxDefinition, SandboxRegistry};
use script::ScriptDependency;
use script_registry::{ScriptDefinition, ScriptRegistry};
use std::path::Path;
use workflow_registry::WorkflowRegistry;

pub use agent_gui::{AGENT_GUI_PAGE_COUNT, AgentManagerPanel, AgentProfile, AgentRuntime};
pub use job_gui::{
    JobDetailPage, JobGui, JobGuiApp, JobGuiResponse, JobGuiState, JobListPage, JobSummaryRow,
    job_summary_rows,
};
pub use node_gui::NodeDetailsView;
pub use sandbox_gui::{SandboxGui, SandboxGuiApp};
pub use script_gui::{ScriptManagerApp, ScriptRegistryPanel};
pub use workflow_gui::{WorkflowGui, WorkflowGuiApp};

pub type NativeRunResult = eframe::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSection {
    Workflows,
    Jobs,
    Nodes,
    Scripts,
    Sandboxes,
    Agents,
}

impl AppSection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Workflows => "Workflows",
            Self::Jobs => "Jobs",
            Self::Nodes => "Nodes",
            Self::Scripts => "Scripts",
            Self::Sandboxes => "Sandboxes",
            Self::Agents => "Agents",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManualApp {
    sections: Vec<AppSection>,
    active_section: AppSection,
    workflow_gui: WorkflowGui,
    job_gui: JobGui,
    job_state: JobGuiState,
    job_registry: JobRegistry,
    node_details: NodeDetailsView,
    node_detail_node: Node,
    script_panel: ScriptRegistryPanel,
    script_registry: ScriptRegistry,
    sandbox_gui: SandboxGui,
    agent_panel: AgentManagerPanel,
}

pub fn run() -> String {
    let descriptor = workspace_descriptor();

    format!(
        "{} app is ready with {} workspace packages and agent GUI {} pages",
        descriptor.name,
        descriptor.packages.len(),
        AGENT_GUI_PAGE_COUNT
    )
}

pub fn manual_app() -> ManualApp {
    let workflow = workflow_gui::sample_agent_handoff_workflow();
    let workflow_gui = WorkflowGui::new(workflow_gui::sample_registry());
    let node_detail_node = workflow
        .nodes()
        .iter()
        .find(|node| node.id.as_str() == "routing_agent")
        .expect("sample workflow should include routing_agent")
        .clone();

    ManualApp {
        sections: vec![
            AppSection::Workflows,
            AppSection::Jobs,
            AppSection::Nodes,
            AppSection::Scripts,
            AppSection::Sandboxes,
            AppSection::Agents,
        ],
        active_section: AppSection::Workflows,
        workflow_gui,
        job_gui: JobGui::new(),
        job_state: JobGuiState::new(),
        job_registry: sample_job_registry(&workflow),
        node_details: NodeDetailsView::new(),
        node_detail_node,
        script_panel: ScriptRegistryPanel::new(),
        script_registry: sample_script_registry(),
        sandbox_gui: SandboxGui::new(sample_sandbox_registry()),
        agent_panel: sample_agent_panel(),
    }
}

impl ManualApp {
    pub fn sections(&self) -> &[AppSection] {
        &self.sections
    }

    pub fn active_section(&self) -> AppSection {
        self.active_section
    }

    pub fn set_active_section(&mut self, section: AppSection) {
        self.active_section = section;
    }

    pub fn workflow_gui(&self) -> &WorkflowGui {
        &self.workflow_gui
    }

    pub fn job_gui(&self) -> &JobGui {
        &self.job_gui
    }

    pub fn job_state(&self) -> &JobGuiState {
        &self.job_state
    }

    pub fn job_registry(&self) -> &JobRegistry {
        &self.job_registry
    }

    pub fn node_detail_node(&self) -> &Node {
        &self.node_detail_node
    }

    pub fn script_registry(&self) -> &ScriptRegistry {
        &self.script_registry
    }

    pub fn script_panel(&self) -> &ScriptRegistryPanel {
        &self.script_panel
    }

    pub fn sandbox_gui(&self) -> &SandboxGui {
        &self.sandbox_gui
    }

    pub fn agent_panel(&self) -> &AgentManagerPanel {
        &self.agent_panel
    }

    fn draw_navigation(&mut self, ui: &mut egui::Ui) {
        ui.set_width(184.0);
        ui.add_space(8.0);
        ui.label(
            RichText::new("Manual")
                .strong()
                .size(21.0)
                .color(Color32::from_rgb(34, 42, 54)),
        );
        ui.add_space(12.0);

        for section in self.sections.clone() {
            let selected = self.active_section == section;
            if ui
                .add_sized(
                    [168.0, 32.0],
                    egui::Button::selectable(selected, section.label()),
                )
                .clicked()
            {
                self.active_section = section;
            }
        }
    }

    fn draw_active_section(&mut self, ui: &mut egui::Ui) {
        match self.active_section {
            AppSection::Workflows => {
                self.workflow_gui.ui(ui);
            }
            AppSection::Jobs => {
                self.job_gui.ui(ui, &mut self.job_state, &self.job_registry);
            }
            AppSection::Nodes => {
                ui.heading("Node Details");
                ui.add_space(8.0);
                self.node_details.ui(ui, &self.node_detail_node);
            }
            AppSection::Scripts => {
                self.script_panel.ui(ui, &mut self.script_registry);
            }
            AppSection::Sandboxes => {
                self.sandbox_gui.ui(ui);
            }
            AppSection::Agents => {
                self.agent_panel.ui(ui);
            }
        }
    }
}

impl Default for ManualApp {
    fn default() -> Self {
        manual_app()
    }
}

impl eframe::App for ManualApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("manual_app_toolbar")
            .exact_height(44.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Manual App");
                    ui.separator();
                    ui.colored_label(
                        Color32::from_rgb(91, 101, 116),
                        format!(
                            "{} workspace packages",
                            workspace_descriptor().packages.len()
                        ),
                    );
                });
            });

        egui::SidePanel::left("manual_app_navigation")
            .resizable(false)
            .exact_width(208.0)
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(244, 246, 248)))
            .show(ctx, |ui| self.draw_navigation(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                self.draw_active_section(ui);
            });
    }
}

pub fn run_native() -> NativeRunResult {
    run_native_app("Manual", manual_app())
}

pub fn run_agent_gui() -> NativeRunResult {
    let mut app = manual_app();
    app.set_active_section(AppSection::Agents);

    run_native_app("Manual Agent Manager", app)
}

fn run_native_app(title: &'static str, app: ManualApp) -> NativeRunResult {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };

    eframe::run_native(
        title,
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(app))),
    )
}

pub fn sandbox_gui_component(registry: SandboxRegistry) -> SandboxGui {
    SandboxGui::new(registry)
}

pub fn script_gui_component() -> ScriptRegistryPanel {
    ScriptRegistryPanel::new()
}

pub fn run_script_gui(repository_root: impl AsRef<Path>) -> script_gui::GuiResult {
    script_gui::run_repository_script_gui(repository_root)
}

pub fn workflow_gui_component(registry: WorkflowRegistry) -> WorkflowGui {
    WorkflowGui::new(registry)
}

pub fn run_workflow_gui() -> workflow_gui::NativeRunResult {
    workflow_gui::run_native(workflow_gui::sample_registry())
}

pub fn job_gui_component() -> (JobGui, JobGuiState) {
    (JobGui::new(), JobGuiState::new())
}

pub fn run_job_gui() -> NativeRunResult {
    job_gui::run_native(sample_job_registry(
        &workflow_gui::sample_agent_handoff_workflow(),
    ))
}

pub fn node_gui_component() -> NodeDetailsView {
    NodeDetailsView::new()
}

pub fn agent_gui_component(agents: Vec<AgentProfile>) -> AgentManagerPanel {
    AgentManagerPanel::new(agents)
}

fn sample_job_registry(workflow: &workflow::Workflow) -> JobRegistry {
    let mut registry = JobRegistry::new();
    registry
        .insert(Job::new("sample-run", workflow, r#"{"ticket":"VOC-1"}"#).unwrap())
        .unwrap();
    registry
}

fn sample_script_registry() -> ScriptRegistry {
    let mut registry = ScriptRegistry::new();
    registry
        .insert(
            ScriptDefinition::new(
                "echo-json",
                "fn main(input_json: &str) -> String { input_json.to_string() }",
            )
            .unwrap()
            .with_dependency(ScriptDependency::version("serde_json", "1")),
        )
        .unwrap();
    registry
}

fn sample_sandbox_registry() -> SandboxRegistry {
    let mut registry = SandboxRegistry::new();
    registry
        .insert(
            SandboxDefinition::new("read-only", sandbox::SandboxPolicy::read_only(".")).unwrap(),
        )
        .unwrap();
    registry
        .insert(
            SandboxDefinition::new(
                "workspace-write",
                sandbox::SandboxPolicy::workspace_write("."),
            )
            .unwrap(),
        )
        .unwrap();
    registry
}

fn sample_agent_panel() -> AgentManagerPanel {
    AgentManagerPanel::new(vec![
        AgentProfile::new("codex", "Codex", AgentRuntime::Codex)
            .unwrap()
            .with_model("gpt-5.5")
            .with_workdir(".")
            .with_execution_policy("workspace-write")
            .with_description("Primary implementation agent."),
        AgentProfile::new("claude", "Claude", AgentRuntime::Claude)
            .unwrap()
            .with_model("sonnet")
            .with_workdir(".")
            .with_execution_policy("read-only")
            .with_description("Review and documentation agent."),
    ])
}
