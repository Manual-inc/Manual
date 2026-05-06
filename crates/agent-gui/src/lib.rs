use std::fmt;

use eframe::egui::{self, Color32, RichText, Ui};

pub const AGENT_GUI_PAGE_COUNT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRuntime {
    Codex,
    Claude,
    Custom(String),
}

impl AgentRuntime {
    pub fn label(&self) -> String {
        match self {
            Self::Codex => "Codex".to_string(),
            Self::Claude => "Claude".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    fn validate(&self) -> Result<(), AgentGuiError> {
        match self {
            Self::Custom(name) if name.trim().is_empty() => Err(AgentGuiError::EmptyRuntimeName),
            _ => Ok(()),
        }
    }
}

impl fmt::Display for AgentRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProfile {
    id: String,
    display_name: String,
    runtime: AgentRuntime,
    model: String,
    workdir: String,
    execution_policy: String,
    description: String,
    enabled: bool,
}

impl AgentProfile {
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        runtime: AgentRuntime,
    ) -> Result<Self, AgentGuiError> {
        AgentDraft::new()
            .with_id(id)
            .with_display_name(display_name)
            .with_runtime(runtime)
            .into_profile()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn runtime(&self) -> &AgentRuntime {
        &self.runtime
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn workdir(&self) -> &str {
        &self.workdir
    }

    pub fn execution_policy(&self) -> &str {
        &self.execution_policy
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_workdir(mut self, workdir: impl Into<String>) -> Self {
        self.workdir = workdir.into();
        self
    }

    pub fn with_execution_policy(mut self, execution_policy: impl Into<String>) -> Self {
        self.execution_policy = execution_policy.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDraft {
    id: String,
    display_name: String,
    runtime: AgentRuntime,
    model: String,
    workdir: String,
    execution_policy: String,
    description: String,
    enabled: bool,
}

impl AgentDraft {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_profile(profile: &AgentProfile) -> Self {
        Self {
            id: profile.id.clone(),
            display_name: profile.display_name.clone(),
            runtime: profile.runtime.clone(),
            model: profile.model.clone(),
            workdir: profile.workdir.clone(),
            execution_policy: profile.execution_policy.clone(),
            description: profile.description.clone(),
            enabled: profile.enabled,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn runtime(&self) -> &AgentRuntime {
        &self.runtime
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn workdir(&self) -> &str {
        &self.workdir
    }

    pub fn execution_policy(&self) -> &str {
        &self.execution_policy
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = id.into();
    }

    pub fn set_display_name(&mut self, display_name: impl Into<String>) {
        self.display_name = display_name.into();
    }

    pub fn set_runtime(&mut self, runtime: AgentRuntime) {
        self.runtime = runtime;
    }

    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
    }

    pub fn set_workdir(&mut self, workdir: impl Into<String>) {
        self.workdir = workdir.into();
    }

    pub fn set_execution_policy(&mut self, execution_policy: impl Into<String>) {
        self.execution_policy = execution_policy.into();
    }

    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = description.into();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.set_id(id);
        self
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.set_display_name(display_name);
        self
    }

    pub fn with_runtime(mut self, runtime: AgentRuntime) -> Self {
        self.set_runtime(runtime);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.set_model(model);
        self
    }

    pub fn with_workdir(mut self, workdir: impl Into<String>) -> Self {
        self.set_workdir(workdir);
        self
    }

    pub fn with_execution_policy(mut self, execution_policy: impl Into<String>) -> Self {
        self.set_execution_policy(execution_policy);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.set_description(description);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.set_enabled(enabled);
        self
    }

    pub fn validate(&self) -> Result<(), AgentGuiError> {
        validate_id(&self.id)?;

        if self.display_name.trim().is_empty() {
            return Err(AgentGuiError::EmptyDisplayName);
        }

        self.runtime.validate()
    }

    pub fn into_profile(self) -> Result<AgentProfile, AgentGuiError> {
        self.validate()?;

        Ok(AgentProfile {
            id: self.id.trim().to_string(),
            display_name: self.display_name.trim().to_string(),
            runtime: normalize_runtime(self.runtime),
            model: self.model,
            workdir: self.workdir,
            execution_policy: self.execution_policy,
            description: self.description,
            enabled: self.enabled,
        })
    }
}

impl Default for AgentDraft {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            runtime: AgentRuntime::Codex,
            model: String::new(),
            workdir: String::new(),
            execution_policy: String::new(),
            description: String::new(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentPage {
    List,
    Detail { id: String },
    New,
    Edit { id: String },
    DeleteConfirm { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    Created(String),
    Updated(String),
    Deleted(String),
    Selected(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentGuiResponse {
    events: Vec<AgentEvent>,
}

impl AgentGuiResponse {
    pub fn events(&self) -> &[AgentEvent] {
        &self.events
    }

    fn push(&mut self, event: AgentEvent) {
        self.events.push(event);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentGuiError {
    EmptyId,
    IdContainsWhitespace(String),
    EmptyDisplayName,
    EmptyRuntimeName,
    DuplicateId(String),
    UnknownId(String),
    NoDraftTarget,
    NoDeleteTarget,
}

impl fmt::Display for AgentGuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "agent id cannot be empty"),
            Self::IdContainsWhitespace(id) => write!(f, "agent id cannot contain whitespace: {id}"),
            Self::EmptyDisplayName => write!(f, "agent display name cannot be empty"),
            Self::EmptyRuntimeName => write!(f, "custom agent runtime name cannot be empty"),
            Self::DuplicateId(id) => write!(f, "duplicate agent id: {id}"),
            Self::UnknownId(id) => write!(f, "unknown agent id: {id}"),
            Self::NoDraftTarget => write!(f, "agent draft can only be submitted from a form page"),
            Self::NoDeleteTarget => {
                write!(f, "agent delete can only be confirmed from delete page")
            }
        }
    }
}

impl std::error::Error for AgentGuiError {}

#[derive(Debug, Clone)]
pub struct AgentManagerPanel {
    agents: Vec<AgentProfile>,
    page: AgentPage,
    draft: AgentDraft,
    status: Option<String>,
}

impl AgentManagerPanel {
    pub fn new(agents: Vec<AgentProfile>) -> Self {
        Self {
            agents,
            page: AgentPage::List,
            draft: AgentDraft::new(),
            status: None,
        }
    }

    pub fn agents(&self) -> &[AgentProfile] {
        &self.agents
    }

    pub fn agent(&self, id: &str) -> Option<&AgentProfile> {
        self.agents.iter().find(|agent| agent.id() == id)
    }

    pub fn page(&self) -> &AgentPage {
        &self.page
    }

    pub fn draft(&self) -> &AgentDraft {
        &self.draft
    }

    pub fn draft_mut(&mut self) -> &mut AgentDraft {
        &mut self.draft
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn selected_agent(&self) -> Option<&AgentProfile> {
        match &self.page {
            AgentPage::Detail { id } | AgentPage::Edit { id } | AgentPage::DeleteConfirm { id } => {
                self.agent(id)
            }
            AgentPage::List | AgentPage::New => None,
        }
    }

    pub fn select_agent(&mut self, id: impl AsRef<str>) -> Result<AgentEvent, AgentGuiError> {
        let id = id.as_ref();
        self.require_agent(id)?;

        let id = id.to_string();
        self.page = AgentPage::Detail { id: id.clone() };
        self.status = Some(format!("Viewing {id}"));
        Ok(AgentEvent::Selected(id))
    }

    pub fn start_registration(&mut self) {
        self.draft = AgentDraft::new();
        self.page = AgentPage::New;
        self.status = None;
    }

    pub fn start_editing(&mut self, id: impl AsRef<str>) -> Result<(), AgentGuiError> {
        let id = id.as_ref();
        let agent = self.require_agent(id)?.clone();

        self.draft = AgentDraft::from_profile(&agent);
        self.page = AgentPage::Edit { id: id.to_string() };
        self.status = None;
        Ok(())
    }

    pub fn request_delete(&mut self, id: impl AsRef<str>) -> Result<(), AgentGuiError> {
        let id = id.as_ref();
        self.require_agent(id)?;

        self.page = AgentPage::DeleteConfirm { id: id.to_string() };
        self.status = None;
        Ok(())
    }

    pub fn cancel_form(&mut self) {
        match &self.page {
            AgentPage::Edit { id } => self.page = AgentPage::Detail { id: id.clone() },
            AgentPage::New => self.page = AgentPage::List,
            AgentPage::List | AgentPage::Detail { .. } | AgentPage::DeleteConfirm { .. } => {}
        }
        self.status = None;
    }

    pub fn cancel_delete(&mut self) {
        match &self.page {
            AgentPage::DeleteConfirm { id } => self.page = AgentPage::Detail { id: id.clone() },
            AgentPage::List
            | AgentPage::Detail { .. }
            | AgentPage::New
            | AgentPage::Edit { .. } => {}
        }
        self.status = None;
    }

    pub fn submit_draft(&mut self) -> Result<AgentEvent, AgentGuiError> {
        let profile = self.draft.clone().into_profile()?;

        match self.page.clone() {
            AgentPage::New => self.create_profile(profile),
            AgentPage::Edit { id } => self.update_profile(&id, profile),
            AgentPage::List | AgentPage::Detail { .. } | AgentPage::DeleteConfirm { .. } => {
                Err(AgentGuiError::NoDraftTarget)
            }
        }
    }

    pub fn confirm_delete(&mut self) -> Result<AgentEvent, AgentGuiError> {
        let AgentPage::DeleteConfirm { id } = self.page.clone() else {
            return Err(AgentGuiError::NoDeleteTarget);
        };

        let Some(index) = self.agents.iter().position(|agent| agent.id() == id) else {
            return Err(AgentGuiError::UnknownId(id));
        };

        self.agents.remove(index);
        self.page = AgentPage::List;
        self.status = Some(format!("Deleted {id}"));
        Ok(AgentEvent::Deleted(id))
    }

    pub fn ui(&mut self, ui: &mut Ui) -> AgentGuiResponse {
        let mut response = AgentGuiResponse::default();

        self.draw_header(ui);
        ui.add_space(8.0);

        if let Some(status) = &self.status {
            ui.colored_label(Color32::from_rgb(69, 129, 92), status);
            ui.add_space(8.0);
        }

        match self.page.clone() {
            AgentPage::List => self.draw_list_page(ui, &mut response),
            AgentPage::Detail { id } => self.draw_detail_page(ui, &id, &mut response),
            AgentPage::New => self.draw_form_page(ui, "Register Agent", &mut response),
            AgentPage::Edit { .. } => self.draw_form_page(ui, "Edit Agent", &mut response),
            AgentPage::DeleteConfirm { id } => {
                self.draw_delete_page(ui, &id, &mut response);
            }
        }

        response
    }

    fn require_agent(&self, id: &str) -> Result<&AgentProfile, AgentGuiError> {
        self.agent(id)
            .ok_or_else(|| AgentGuiError::UnknownId(id.to_string()))
    }

    fn create_profile(&mut self, profile: AgentProfile) -> Result<AgentEvent, AgentGuiError> {
        if self.agent(profile.id()).is_some() {
            return Err(AgentGuiError::DuplicateId(profile.id().to_string()));
        }

        let id = profile.id().to_string();
        self.agents.push(profile);
        self.page = AgentPage::Detail { id: id.clone() };
        self.status = Some(format!("Registered {id}"));
        Ok(AgentEvent::Created(id))
    }

    fn update_profile(
        &mut self,
        original_id: &str,
        profile: AgentProfile,
    ) -> Result<AgentEvent, AgentGuiError> {
        if profile.id() != original_id && self.agent(profile.id()).is_some() {
            return Err(AgentGuiError::DuplicateId(profile.id().to_string()));
        }

        let Some(index) = self
            .agents
            .iter()
            .position(|agent| agent.id() == original_id)
        else {
            return Err(AgentGuiError::UnknownId(original_id.to_string()));
        };

        let id = profile.id().to_string();
        self.agents[index] = profile;
        self.page = AgentPage::Detail { id: id.clone() };
        self.status = Some(format!("Updated {id}"));
        Ok(AgentEvent::Updated(id))
    }

    fn draw_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Agents");
            ui.add_space(8.0);
            if ui
                .button("Register")
                .on_hover_text("Register agent")
                .clicked()
            {
                self.start_registration();
            }
        });
    }

    fn draw_list_page(&mut self, ui: &mut Ui, response: &mut AgentGuiResponse) {
        if self.agents.is_empty() {
            ui.label("No agents registered.");
            return;
        }

        ui.horizontal(|ui| {
            ui.add_sized(
                [160.0, 20.0],
                egui::Label::new(RichText::new("Name").strong()),
            );
            ui.add_sized(
                [112.0, 20.0],
                egui::Label::new(RichText::new("Provider").strong()),
            );
            ui.add_sized(
                [136.0, 20.0],
                egui::Label::new(RichText::new("Model").strong()),
            );
            ui.add_sized(
                [136.0, 20.0],
                egui::Label::new(RichText::new("Policy").strong()),
            );
            ui.add_sized(
                [88.0, 20.0],
                egui::Label::new(RichText::new("State").strong()),
            );
            ui.strong("Actions");
        });
        ui.separator();

        for agent in self.agents.clone() {
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        [160.0, 24.0],
                        egui::Button::new(agent.display_name()).selected(false),
                    )
                    .on_hover_text("Open details")
                    .clicked()
                {
                    record_result(response, self.select_agent(agent.id()));
                }
                ui.add_sized([112.0, 24.0], egui::Label::new(agent.runtime().label()));
                ui.add_sized(
                    [136.0, 24.0],
                    egui::Label::new(value_or_dash(agent.model())),
                );
                ui.add_sized(
                    [136.0, 24.0],
                    egui::Label::new(value_or_dash(agent.execution_policy())),
                );
                ui.add_sized(
                    [88.0, 24.0],
                    egui::Label::new(if agent.enabled() {
                        "Enabled"
                    } else {
                        "Disabled"
                    }),
                );
                if ui.button("Edit").clicked() {
                    if let Err(error) = self.start_editing(agent.id()) {
                        self.status = Some(error.to_string());
                    }
                }
                if ui.button("Delete").clicked() {
                    if let Err(error) = self.request_delete(agent.id()) {
                        self.status = Some(error.to_string());
                    }
                }
            });
        }
    }

    fn draw_detail_page(&mut self, ui: &mut Ui, id: &str, response: &mut AgentGuiResponse) {
        let Some(agent) = self.agent(id).cloned() else {
            ui.colored_label(
                Color32::from_rgb(180, 67, 57),
                format!("Unknown agent: {id}"),
            );
            return;
        };

        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                self.page = AgentPage::List;
            }
            if ui.button("Edit").clicked() {
                if let Err(error) = self.start_editing(agent.id()) {
                    self.status = Some(error.to_string());
                }
            }
            if ui.button("Delete").clicked() {
                if let Err(error) = self.request_delete(agent.id()) {
                    self.status = Some(error.to_string());
                }
            }
        });
        ui.add_space(8.0);

        ui.heading(agent.display_name());
        ui.label(
            RichText::new(agent.id())
                .monospace()
                .color(Color32::from_rgb(84, 112, 151)),
        );
        ui.separator();

        detail_row(ui, "Provider", &agent.runtime().label());
        detail_row(ui, "Model", value_or_dash(agent.model()));
        detail_row(ui, "Workdir", value_or_dash(agent.workdir()));
        detail_row(ui, "Policy", value_or_dash(agent.execution_policy()));
        detail_row(
            ui,
            "State",
            if agent.enabled() {
                "Enabled"
            } else {
                "Disabled"
            },
        );
        ui.separator();
        ui.label(value_or_dash(agent.description()));

        let _ = response;
    }

    fn draw_form_page(&mut self, ui: &mut Ui, title: &str, response: &mut AgentGuiResponse) {
        ui.heading(title);
        ui.add_space(8.0);

        form_text_row(ui, "ID", &mut self.draft.id);
        form_text_row(ui, "Name", &mut self.draft.display_name);
        ui.horizontal(|ui| {
            ui.add_sized([112.0, 20.0], egui::Label::new("Provider"));
            egui::ComboBox::from_id_salt("agent_gui_runtime")
                .selected_text(self.draft.runtime.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draft.runtime, AgentRuntime::Codex, "Codex");
                    ui.selectable_value(&mut self.draft.runtime, AgentRuntime::Claude, "Claude");
                    if !matches!(self.draft.runtime, AgentRuntime::Custom(_)) {
                        ui.selectable_value(
                            &mut self.draft.runtime,
                            AgentRuntime::Custom("custom".to_string()),
                            "Custom",
                        );
                    }
                });
        });

        if let AgentRuntime::Custom(runtime_name) = &mut self.draft.runtime {
            form_text_row(ui, "Runtime", runtime_name);
        }

        form_text_row(ui, "Model", &mut self.draft.model);
        form_text_row(ui, "Workdir", &mut self.draft.workdir);
        form_text_row(ui, "Policy", &mut self.draft.execution_policy);
        ui.horizontal(|ui| {
            ui.add_sized([112.0, 20.0], egui::Label::new("Enabled"));
            ui.checkbox(&mut self.draft.enabled, "");
        });

        ui.add_space(8.0);
        ui.label("Description");
        ui.add(egui::TextEdit::multiline(&mut self.draft.description).desired_rows(4));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                match self.submit_draft() {
                    Ok(event) => response.push(event),
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
            if ui.button("Cancel").clicked() {
                self.cancel_form();
            }
        });
    }

    fn draw_delete_page(&mut self, ui: &mut Ui, id: &str, response: &mut AgentGuiResponse) {
        let display_name = self
            .agent(id)
            .map(|agent| agent.display_name().to_string())
            .unwrap_or_else(|| id.to_string());

        ui.heading("Delete Agent");
        ui.label(format!("Delete {display_name}?"));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                self.cancel_delete();
            }
            if ui
                .button(RichText::new("Delete").color(Color32::from_rgb(180, 67, 57)))
                .clicked()
            {
                match self.confirm_delete() {
                    Ok(event) => response.push(event),
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
        });
    }
}

impl Default for AgentManagerPanel {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

fn record_result(response: &mut AgentGuiResponse, result: Result<AgentEvent, AgentGuiError>) {
    if let Ok(event) = result {
        response.push(event);
    }
}

fn detail_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [112.0, 20.0],
            egui::Label::new(RichText::new(label).strong()),
        );
        ui.label(value);
    });
}

fn form_text_row(ui: &mut Ui, label: &str, value: &mut String) {
    ui.horizontal(|ui| {
        ui.add_sized([112.0, 20.0], egui::Label::new(label));
        ui.add_sized([360.0, 24.0], egui::TextEdit::singleline(value));
    });
}

fn value_or_dash(value: &str) -> &str {
    if value.trim().is_empty() { "-" } else { value }
}

fn normalize_runtime(runtime: AgentRuntime) -> AgentRuntime {
    match runtime {
        AgentRuntime::Custom(name) => AgentRuntime::Custom(name.trim().to_string()),
        runtime => runtime,
    }
}

fn validate_id(id: &str) -> Result<(), AgentGuiError> {
    let id = id.trim();

    if id.is_empty() {
        return Err(AgentGuiError::EmptyId);
    }

    if id.chars().any(char::is_whitespace) {
        return Err(AgentGuiError::IdContainsWhitespace(id.to_string()));
    }

    Ok(())
}
