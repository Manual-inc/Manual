use std::fmt;
use std::path::PathBuf;

use eframe::egui::{self, Color32, Response, RichText, Ui};
use sandbox::{FilesystemAccess, FilesystemEntry, NetworkMode, SandboxPolicy, SandboxPreset};
use sandbox_registry::{SandboxDefinition, SandboxId, SandboxRegistry, SandboxRegistryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxDraft {
    pub id: String,
    pub preset: SandboxPreset,
    pub workspace_root: PathBuf,
    pub network_enabled: bool,
}

impl SandboxDraft {
    pub fn registration() -> Self {
        Self {
            id: String::new(),
            preset: SandboxPreset::ReadOnly,
            workspace_root: PathBuf::from("."),
            network_enabled: false,
        }
    }

    pub fn from_definition(definition: &SandboxDefinition) -> Self {
        let policy = definition.policy();

        Self {
            id: definition.id().as_str().to_string(),
            preset: policy.preset,
            workspace_root: workspace_root_for_policy(policy),
            network_enabled: policy.network.mode == NetworkMode::Enabled,
        }
    }

    pub fn to_definition(&self) -> Result<SandboxDefinition, SandboxRegistryError> {
        SandboxDefinition::new(self.id.trim(), self.to_policy())
    }

    pub fn to_policy(&self) -> SandboxPolicy {
        let workspace_root = self.workspace_root.clone();
        let mut policy = match self.preset {
            SandboxPreset::ReadOnly => SandboxPolicy::read_only(workspace_root),
            SandboxPreset::WorkspaceWrite => SandboxPolicy::workspace_write(workspace_root),
            SandboxPreset::DangerFullAccess => return SandboxPolicy::danger_full_access(),
        };

        policy.network.mode = if self.network_enabled {
            NetworkMode::Enabled
        } else {
            NetworkMode::Restricted
        };
        policy
    }
}

impl Default for SandboxDraft {
    fn default() -> Self {
        Self::registration()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxGuiMode {
    Detail,
    Register,
    Edit { original_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxGuiError {
    NoSelection,
    NotEditing,
    Registry(SandboxRegistryError),
}

impl fmt::Display for SandboxGuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSelection => write!(f, "no sandbox is selected"),
            Self::NotEditing => write!(f, "sandbox editor is not in an editable mode"),
            Self::Registry(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SandboxGuiError {}

impl From<SandboxRegistryError> for SandboxGuiError {
    fn from(value: SandboxRegistryError) -> Self {
        Self::Registry(value)
    }
}

#[derive(Debug, Clone)]
pub struct SandboxGui {
    registry: SandboxRegistry,
    selected_id: Option<SandboxId>,
    mode: SandboxGuiMode,
    draft: SandboxDraft,
    status: Option<StatusMessage>,
}

impl SandboxGui {
    pub fn new(registry: SandboxRegistry) -> Self {
        let selected_id = registry
            .iter()
            .next()
            .map(|definition| definition.id().clone());

        Self {
            registry,
            selected_id,
            mode: SandboxGuiMode::Detail,
            draft: SandboxDraft::registration(),
            status: None,
        }
    }

    pub fn registry(&self) -> &SandboxRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut SandboxRegistry {
        &mut self.registry
    }

    pub fn into_registry(self) -> SandboxRegistry {
        self.registry
    }

    pub fn definition_ids(&self) -> Vec<String> {
        self.registry
            .iter()
            .map(|definition| definition.id().as_str().to_string())
            .collect()
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_ref().map(SandboxId::as_str)
    }

    pub fn selected_definition(&self) -> Option<&SandboxDefinition> {
        self.selected_id
            .as_ref()
            .and_then(|id| self.registry.get(id))
    }

    pub fn mode(&self) -> SandboxGuiMode {
        self.mode.clone()
    }

    pub fn draft(&self) -> &SandboxDraft {
        &self.draft
    }

    pub fn draft_mut(&mut self) -> &mut SandboxDraft {
        &mut self.draft
    }

    pub fn select(&mut self, id: impl Into<String>) -> Result<(), SandboxGuiError> {
        let id = SandboxId::new(id.into()).map_err(SandboxRegistryError::InvalidId)?;

        if self.registry.get(&id).is_none() {
            return Err(SandboxRegistryError::UnknownId(id).into());
        }

        self.selected_id = Some(id);
        self.mode = SandboxGuiMode::Detail;
        self.status = None;
        Ok(())
    }

    pub fn start_registration(&mut self) {
        self.mode = SandboxGuiMode::Register;
        self.draft = SandboxDraft::registration();
        self.status = None;
    }

    pub fn start_editing_selected(&mut self) -> Result<(), SandboxGuiError> {
        let definition = self
            .selected_definition()
            .ok_or(SandboxGuiError::NoSelection)?
            .clone();

        self.draft = SandboxDraft::from_definition(&definition);
        self.mode = SandboxGuiMode::Edit {
            original_id: definition.id().as_str().to_string(),
        };
        self.status = None;
        Ok(())
    }

    pub fn cancel_editing(&mut self) {
        self.mode = SandboxGuiMode::Detail;
        self.draft = SandboxDraft::registration();
        self.status = None;
    }

    pub fn save_draft(&mut self) -> Result<(), SandboxGuiError> {
        let definition = self.draft.to_definition()?;
        let saved_id = definition.id().clone();

        match &self.mode {
            SandboxGuiMode::Register => {
                self.registry.insert(definition)?;
                self.status = Some(StatusMessage::success(format!(
                    "Registered sandbox '{}'",
                    saved_id.as_str()
                )));
            }
            SandboxGuiMode::Edit { original_id } => {
                self.registry.update(original_id.clone(), definition)?;
                self.status = Some(StatusMessage::success(format!(
                    "Updated sandbox '{}'",
                    saved_id.as_str()
                )));
            }
            SandboxGuiMode::Detail => return Err(SandboxGuiError::NotEditing),
        }

        self.selected_id = Some(saved_id);
        self.mode = SandboxGuiMode::Detail;
        self.draft = SandboxDraft::registration();
        Ok(())
    }

    pub fn delete_selected(&mut self) -> Result<(), SandboxGuiError> {
        let selected_id = self
            .selected_id
            .as_ref()
            .ok_or(SandboxGuiError::NoSelection)?
            .clone();
        let removed = self.registry.remove(selected_id.as_str())?;

        self.selected_id = self
            .registry
            .iter()
            .next()
            .map(|definition| definition.id().clone());
        self.mode = SandboxGuiMode::Detail;
        self.draft = SandboxDraft::registration();
        self.status = Some(StatusMessage::warning(format!(
            "Deleted sandbox '{}'",
            removed.id().as_str()
        )));
        Ok(())
    }

    pub fn ui(&mut self, ui: &mut Ui) -> SandboxGuiResponse {
        let mut changed = false;
        let response = ui
            .scope(|ui| {
                ui.set_min_size(ui.available_size_before_wrap());
                changed = self.draw(ui);
            })
            .response;

        SandboxGuiResponse {
            response,
            changed,
            selected_id: self.selected_id().map(ToString::to_string),
        }
    }

    fn draw(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        egui::Frame::NONE
            .fill(Color32::from_rgb(246, 247, 244))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 10.0);
                draw_header(ui);

                if let Some(status) = &self.status {
                    draw_status(ui, status);
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.set_min_height((ui.available_height() - 8.0).max(360.0));

                    ui.vertical(|ui| {
                        ui.set_width(260.0);
                        changed |= self.draw_list(ui);
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 8.0).max(420.0));
                        changed |= self.draw_main(ui);
                    });
                });
            });

        changed
    }

    fn draw_list(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.heading("Sandboxes");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new("+").strong())
                    .on_hover_text("Register a sandbox")
                    .clicked()
                {
                    self.start_registration();
                }
            });
        });

        ui.add_space(4.0);
        let ids = self.definition_ids();

        if ids.is_empty() {
            ui.colored_label(Color32::from_rgb(96, 101, 110), "No sandboxes registered");
            return changed;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for id in ids {
                    let selected = self.selected_id() == Some(id.as_str())
                        && matches!(self.mode, SandboxGuiMode::Detail);
                    let label = egui::Button::selectable(selected, id.as_str());

                    if ui.add_sized([240.0, 30.0], label).clicked() {
                        if self.select(id).is_ok() {
                            changed = true;
                        }
                    }
                }
            });

        changed
    }

    fn draw_main(&mut self, ui: &mut Ui) -> bool {
        match self.mode.clone() {
            SandboxGuiMode::Detail => self.draw_detail(ui),
            SandboxGuiMode::Register => self.draw_form(ui, "Register Sandbox"),
            SandboxGuiMode::Edit { .. } => self.draw_form(ui, "Edit Sandbox"),
        }
    }

    fn draw_detail(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        let Some(definition) = self.selected_definition().cloned() else {
            ui.heading("Sandbox Detail");
            ui.colored_label(
                Color32::from_rgb(96, 101, 110),
                "Select or register a sandbox.",
            );

            if ui.button("Register Sandbox").clicked() {
                self.start_registration();
                changed = true;
            }

            return changed;
        };

        ui.horizontal(|ui| {
            ui.heading("Sandbox Detail");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Delete")
                    .on_hover_text("Delete this sandbox")
                    .clicked()
                {
                    if let Err(error) = self.delete_selected() {
                        self.status = Some(StatusMessage::error(error.to_string()));
                    } else {
                        changed = true;
                    }
                }

                if ui
                    .button("Edit")
                    .on_hover_text("Edit this sandbox")
                    .clicked()
                {
                    if let Err(error) = self.start_editing_selected() {
                        self.status = Some(StatusMessage::error(error.to_string()));
                    }
                }
            });
        });

        ui.separator();
        draw_detail_row(ui, "ID", definition.id().as_str());
        draw_detail_row(ui, "Preset", preset_label(definition.policy().preset));
        draw_detail_row(
            ui,
            "Network",
            network_label(definition.policy().network.mode),
        );
        draw_detail_row(
            ui,
            "Filesystem",
            filesystem_summary(definition.policy()).as_str(),
        );

        ui.add_space(8.0);
        ui.label(RichText::new("Filesystem Entries").strong());
        draw_filesystem_entries(ui, &definition.policy().filesystem.entries);

        changed
    }

    fn draw_form(&mut self, ui: &mut Ui, title: &str) -> bool {
        let mut changed = false;

        ui.heading(title);
        ui.separator();

        egui::Grid::new("sandbox_gui_form")
            .num_columns(2)
            .spacing([18.0, 12.0])
            .striped(false)
            .show(ui, |ui| {
                ui.label("ID");
                ui.text_edit_singleline(&mut self.draft.id);
                ui.end_row();

                ui.label("Preset");
                egui::ComboBox::from_id_salt("sandbox_gui_preset")
                    .selected_text(preset_label(self.draft.preset))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.draft.preset,
                            SandboxPreset::ReadOnly,
                            preset_label(SandboxPreset::ReadOnly),
                        );
                        ui.selectable_value(
                            &mut self.draft.preset,
                            SandboxPreset::WorkspaceWrite,
                            preset_label(SandboxPreset::WorkspaceWrite),
                        );
                        ui.selectable_value(
                            &mut self.draft.preset,
                            SandboxPreset::DangerFullAccess,
                            preset_label(SandboxPreset::DangerFullAccess),
                        );
                    });
                ui.end_row();

                ui.label("Workspace Root");
                let mut workspace_root = self.draft.workspace_root.display().to_string();
                let enabled = self.draft.preset != SandboxPreset::DangerFullAccess;
                ui.add_enabled_ui(enabled, |ui| {
                    if ui.text_edit_singleline(&mut workspace_root).changed() {
                        self.draft.workspace_root = PathBuf::from(workspace_root);
                    }
                });
                ui.end_row();

                ui.label("Network");
                if self.draft.preset == SandboxPreset::DangerFullAccess {
                    self.draft.network_enabled = true;
                }
                ui.add_enabled_ui(self.draft.preset != SandboxPreset::DangerFullAccess, |ui| {
                    ui.checkbox(&mut self.draft.network_enabled, "Enabled");
                });
                ui.end_row();
            });

        ui.add_space(8.0);
        let preview = self.draft.to_policy();
        ui.label(RichText::new("Policy Preview").strong());
        draw_detail_row(ui, "Preset", preset_label(preview.preset));
        draw_detail_row(ui, "Network", network_label(preview.network.mode));
        draw_filesystem_entries(ui, &preview.filesystem.entries);

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                match self.save_draft() {
                    Ok(()) => changed = true,
                    Err(error) => self.status = Some(StatusMessage::error(error.to_string())),
                }
            }

            if ui.button("Cancel").clicked() {
                self.cancel_editing();
            }
        });

        changed
    }
}

#[derive(Debug, Clone)]
pub struct SandboxGuiResponse {
    pub response: Response,
    pub changed: bool,
    pub selected_id: Option<String>,
}

pub struct SandboxGuiApp {
    gui: SandboxGui,
}

impl SandboxGuiApp {
    pub fn new(registry: SandboxRegistry) -> Self {
        Self {
            gui: SandboxGui::new(registry),
        }
    }

    pub fn gui(&self) -> &SandboxGui {
        &self.gui
    }

    pub fn gui_mut(&mut self) -> &mut SandboxGui {
        &mut self.gui
    }
}

impl eframe::App for SandboxGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.gui.ui(ui);
            });
    }
}

pub fn run_native(registry: SandboxRegistry) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1040.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Manual Sandbox GUI",
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(SandboxGuiApp::new(registry.clone())))),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusMessage {
    text: String,
    kind: StatusKind,
}

impl StatusMessage {
    fn success(text: String) -> Self {
        Self {
            text,
            kind: StatusKind::Success,
        }
    }

    fn warning(text: String) -> Self {
        Self {
            text,
            kind: StatusKind::Warning,
        }
    }

    fn error(text: String) -> Self {
        Self {
            text,
            kind: StatusKind::Error,
        }
    }

    fn color(&self) -> Color32 {
        match self.kind {
            StatusKind::Success => Color32::from_rgb(68, 126, 92),
            StatusKind::Warning => Color32::from_rgb(172, 110, 47),
            StatusKind::Error => Color32::from_rgb(188, 70, 66),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusKind {
    Success,
    Warning,
    Error,
}

fn draw_header(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Manual Sandbox Console")
                .strong()
                .color(Color32::from_rgb(30, 35, 43))
                .size(18.0),
        );
        ui.separator();
        ui.colored_label(
            Color32::from_rgb(93, 101, 116),
            "Manage local sandbox policies",
        );
    });
}

fn draw_status(ui: &mut Ui, status: &StatusMessage) {
    ui.colored_label(status.color(), &status.text);
}

fn draw_detail_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.set_min_width(420.0);
        ui.label(
            RichText::new(label)
                .strong()
                .color(Color32::from_rgb(69, 76, 88)),
        );
        ui.add_space(12.0);
        ui.monospace(value);
    });
}

fn draw_filesystem_entries(ui: &mut Ui, entries: &[FilesystemEntry]) {
    if entries.is_empty() {
        ui.colored_label(Color32::from_rgb(96, 101, 110), "No filesystem entries");
        return;
    }

    egui::Grid::new("sandbox_gui_filesystem_entries")
        .num_columns(2)
        .spacing([18.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label(RichText::new("Access").strong());
            ui.label(RichText::new("Path").strong());
            ui.end_row();

            for entry in entries {
                ui.monospace(access_label(entry.access));
                ui.monospace(entry.path.display().to_string());
                ui.end_row();
            }
        });
}

fn workspace_root_for_policy(policy: &SandboxPolicy) -> PathBuf {
    match policy.preset {
        SandboxPreset::ReadOnly => policy_root_with_access(policy, FilesystemAccess::Read),
        SandboxPreset::WorkspaceWrite => policy_root_with_access(policy, FilesystemAccess::Write),
        SandboxPreset::DangerFullAccess => PathBuf::from("."),
    }
}

fn policy_root_with_access(policy: &SandboxPolicy, access: FilesystemAccess) -> PathBuf {
    policy
        .filesystem
        .entries
        .iter()
        .find(|entry| entry.access == access)
        .map(|entry| entry.path.clone())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn preset_label(preset: SandboxPreset) -> &'static str {
    match preset {
        SandboxPreset::ReadOnly => "Read Only",
        SandboxPreset::WorkspaceWrite => "Workspace Write",
        SandboxPreset::DangerFullAccess => "Danger Full Access",
    }
}

fn network_label(mode: NetworkMode) -> &'static str {
    match mode {
        NetworkMode::Restricted => "Restricted",
        NetworkMode::Enabled => "Enabled",
    }
}

fn access_label(access: FilesystemAccess) -> &'static str {
    match access {
        FilesystemAccess::Read => "Read",
        FilesystemAccess::Write => "Write",
        FilesystemAccess::None => "None",
    }
}

fn filesystem_summary(policy: &SandboxPolicy) -> String {
    let entries = policy.filesystem.entries.len();
    match entries {
        0 => "no entries".to_string(),
        1 => "1 entry".to_string(),
        count => format!("{count} entries"),
    }
}
