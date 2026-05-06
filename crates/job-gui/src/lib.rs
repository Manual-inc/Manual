use eframe::egui::{self, Color32, RichText, Ui};
use job::{Job, JobId, JobStatus, NodeRunStatus};
use job_registry::JobRegistry;

const STATUS_CREATED: Color32 = Color32::from_rgb(116, 125, 136);
const STATUS_RUNNING: Color32 = Color32::from_rgb(54, 126, 188);
const STATUS_SUCCEEDED: Color32 = Color32::from_rgb(70, 142, 88);
const STATUS_FAILED: Color32 = Color32::from_rgb(194, 82, 72);
const STATUS_CANCELED: Color32 = Color32::from_rgb(160, 111, 70);
const TEXT_MUTED: Color32 = Color32::from_rgb(132, 144, 160);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSummaryRow {
    pub id: String,
    pub workflow_id: String,
    pub status: JobStatus,
    pub completed_nodes: usize,
    pub total_nodes: usize,
    pub running_nodes: usize,
    pub failed_nodes: usize,
}

pub fn job_summary_rows(registry: &JobRegistry) -> Vec<JobSummaryRow> {
    registry.iter().map(JobSummaryRow::from_job).collect()
}

impl JobSummaryRow {
    fn from_job(job: &Job) -> Self {
        Self {
            id: job.id().as_str().to_string(),
            workflow_id: job.workflow_id().to_string(),
            status: job.status(),
            completed_nodes: job
                .nodes()
                .iter()
                .filter(|node| {
                    matches!(
                        node.status(),
                        NodeRunStatus::Succeeded | NodeRunStatus::Skipped
                    )
                })
                .count(),
            total_nodes: job.nodes().len(),
            running_nodes: job
                .nodes()
                .iter()
                .filter(|node| node.status() == NodeRunStatus::Running)
                .count(),
            failed_nodes: job
                .nodes()
                .iter()
                .filter(|node| node.status() == NodeRunStatus::Failed)
                .count(),
        }
    }

    pub fn progress_label(&self) -> String {
        format!("{}/{} complete", self.completed_nodes, self.total_nodes)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobGuiState {
    selected_job_id: Option<JobId>,
}

impl JobGuiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selected_job_id(&self) -> Option<&str> {
        self.selected_job_id.as_ref().map(JobId::as_str)
    }

    pub fn selected_job<'a>(&self, registry: &'a JobRegistry) -> Option<&'a Job> {
        self.selected_job_id
            .as_ref()
            .and_then(|id| registry.get(id))
    }

    pub fn select_job(&mut self, id: impl Into<String>, registry: &JobRegistry) {
        let Ok(id) = JobId::new(id) else {
            self.selected_job_id = None;
            return;
        };

        if registry.get(&id).is_some() {
            self.selected_job_id = Some(id);
        } else {
            self.selected_job_id = None;
        }
    }

    pub fn ensure_selection(&mut self, registry: &JobRegistry) {
        if self.selected_job(registry).is_some() {
            return;
        }

        self.selected_job_id = registry.iter().next().map(|job| job.id().clone());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobGuiResponse {
    pub selected_job_id: Option<String>,
    pub selected_job_changed: bool,
}

impl JobGuiResponse {
    fn from_state(before: Option<String>, state: &JobGuiState) -> Self {
        let selected_job_id = state.selected_job_id().map(str::to_string);

        Self {
            selected_job_changed: before != selected_job_id,
            selected_job_id,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct JobGui;

impl JobGui {
    pub fn new() -> Self {
        Self
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        state: &mut JobGuiState,
        registry: &JobRegistry,
    ) -> JobGuiResponse {
        state.ensure_selection(registry);
        let before = state.selected_job_id().map(str::to_string);
        let available_height = ui.available_height();

        ui.columns(2, |columns| {
            columns[0].set_min_width(360.0);
            columns[0].set_min_height(available_height);
            JobListPage::ui(&mut columns[0], state, registry);

            columns[1].set_min_width(420.0);
            columns[1].set_min_height(available_height);
            JobDetailPage::ui(&mut columns[1], state.selected_job(registry));
        });

        JobGuiResponse::from_state(before, state)
    }
}

pub struct JobListPage;

impl JobListPage {
    pub fn ui(ui: &mut Ui, state: &mut JobGuiState, registry: &JobRegistry) -> JobGuiResponse {
        state.ensure_selection(registry);
        let before = state.selected_job_id().map(str::to_string);

        ui.heading("Jobs");
        ui.add_space(6.0);

        if registry.is_empty() {
            ui.separator();
            ui.add_space(18.0);
            ui.colored_label(TEXT_MUTED, "No jobs registered");
            return JobGuiResponse::from_state(before, state);
        }

        ui.horizontal(|ui| {
            ui.add_sized(
                [96.0, 18.0],
                egui::Label::new(RichText::new("Job").strong()),
            );
            ui.add_sized(
                [116.0, 18.0],
                egui::Label::new(RichText::new("Workflow").strong()),
            );
            ui.add_sized(
                [72.0, 18.0],
                egui::Label::new(RichText::new("Status").strong()),
            );
            ui.add_sized(
                [104.0, 18.0],
                egui::Label::new(RichText::new("Progress").strong()),
            );
        });

        ui.separator();

        for row in job_summary_rows(registry) {
            ui.horizontal(|ui| {
                let selected = state.selected_job_id() == Some(row.id.as_str());
                if ui
                    .add_sized([96.0, 24.0], egui::Button::selectable(selected, &row.id))
                    .clicked()
                {
                    state.select_job(row.id.clone(), registry);
                }

                ui.add_sized([116.0, 24.0], egui::Label::new(&row.workflow_id));
                ui.add_sized(
                    [72.0, 24.0],
                    egui::Label::new(
                        RichText::new(row.status.as_str()).color(status_color(row.status)),
                    ),
                );
                ui.add_sized([104.0, 24.0], egui::Label::new(row.progress_label()));
            });
        }

        JobGuiResponse::from_state(before, state)
    }
}

pub struct JobDetailPage;

impl JobDetailPage {
    pub fn ui(ui: &mut Ui, job: Option<&Job>) {
        ui.heading("Job Detail");
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(8.0);

        let Some(job) = job else {
            ui.add_space(18.0);
            ui.colored_label(TEXT_MUTED, "Select a job to inspect details");
            return;
        };

        let summary = JobSummaryRow::from_job(job);

        detail_row(ui, "Job", |ui| {
            ui.monospace(job.id().as_str());
        });
        detail_row(ui, "Workflow", |ui| {
            ui.monospace(job.workflow_id());
        });
        detail_row(ui, "Status", |ui| {
            ui.colored_label(status_color(job.status()), job.status().as_str());
        });
        detail_row(ui, "Progress", |ui| {
            ui.label(summary.progress_label());
        });
        detail_row(ui, "Input", |ui| {
            ui.monospace(job.input_json());
        });

        ui.add_space(16.0);
        ui.heading("Nodes");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.add_sized(
                [120.0, 18.0],
                egui::Label::new(RichText::new("Node").strong()),
            );
            ui.add_sized(
                [132.0, 18.0],
                egui::Label::new(RichText::new("Status").strong()),
            );
            ui.add_sized(
                [72.0, 18.0],
                egui::Label::new(RichText::new("Attempts").strong()),
            );
        });

        ui.separator();

        for node in job.nodes() {
            ui.horizontal(|ui| {
                ui.add_sized([120.0, 24.0], egui::Label::new(node.node_id().as_str()));
                ui.add_sized(
                    [132.0, 24.0],
                    egui::Label::new(
                        RichText::new(node.status().as_str())
                            .color(node_status_color(node.status())),
                    ),
                );
                ui.add_sized([72.0, 24.0], egui::Label::new(node.attempts().to_string()));
            });
        }
    }
}

pub struct JobGuiApp {
    registry: JobRegistry,
    state: JobGuiState,
    gui: JobGui,
}

impl JobGuiApp {
    pub fn new(registry: JobRegistry) -> Self {
        Self {
            registry,
            state: JobGuiState::new(),
            gui: JobGui::new(),
        }
    }

    pub fn registry(&self) -> &JobRegistry {
        &self.registry
    }

    pub fn state(&self) -> &JobGuiState {
        &self.state
    }
}

impl eframe::App for JobGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("job_gui_toolbar")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Manual Job Viewer");
                    ui.separator();
                    ui.colored_label(TEXT_MUTED, format!("{} jobs", self.registry.len()));
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.gui.ui(ui, &mut self.state, &self.registry);
            });
    }
}

pub fn run_native(registry: JobRegistry) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 760.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Manual Job Viewer",
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(JobGuiApp::new(registry.clone())))),
    )
}

fn detail_row(ui: &mut Ui, label: &str, value: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [84.0, 24.0],
            egui::Label::new(RichText::new(label).color(TEXT_MUTED)),
        );
        value(ui);
    });
}

fn status_color(status: JobStatus) -> Color32 {
    match status {
        JobStatus::Created => STATUS_CREATED,
        JobStatus::Running => STATUS_RUNNING,
        JobStatus::Succeeded => STATUS_SUCCEEDED,
        JobStatus::Failed => STATUS_FAILED,
        JobStatus::Canceled => STATUS_CANCELED,
    }
}

fn node_status_color(status: NodeRunStatus) -> Color32 {
    match status {
        NodeRunStatus::Pending => STATUS_CREATED,
        NodeRunStatus::Ready => Color32::from_rgb(142, 128, 62),
        NodeRunStatus::Running => STATUS_RUNNING,
        NodeRunStatus::Succeeded => STATUS_SUCCEEDED,
        NodeRunStatus::Failed => STATUS_FAILED,
        NodeRunStatus::Skipped => STATUS_CANCELED,
        NodeRunStatus::WaitingForApproval => Color32::from_rgb(175, 96, 130),
    }
}
