use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit, Ui};
use script::{ScriptDependency, ScriptDependencySource};
use script_registry::{
    FileScriptStore, ScriptDefinition, ScriptId, ScriptRegistry, ScriptRegistryError, ScriptStore,
    ScriptStoreError,
};

pub type GuiResult = eframe::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    Detail,
    Register,
    Edit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptGuiAction {
    Created(String),
    Updated(String),
    Deleted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptDetail {
    pub id: String,
    pub rust_source: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptGuiError {
    Registry(ScriptRegistryError),
    Dependency(String),
    MissingSelection,
    UnsupportedMode(FormMode),
    Store(ScriptStoreError),
}

impl fmt::Display for ScriptGuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => write!(f, "{error}"),
            Self::Dependency(message) => write!(f, "{message}"),
            Self::MissingSelection => write!(f, "no script is selected"),
            Self::UnsupportedMode(mode) => write!(f, "cannot submit a {mode:?} page"),
            Self::Store(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ScriptGuiError {}

impl From<ScriptRegistryError> for ScriptGuiError {
    fn from(error: ScriptRegistryError) -> Self {
        Self::Registry(error)
    }
}

impl From<ScriptStoreError> for ScriptGuiError {
    fn from(error: ScriptStoreError) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScriptForm {
    id: String,
    rust_source: String,
    dependencies_text: String,
}

impl ScriptForm {
    fn empty() -> Self {
        Self {
            id: String::new(),
            rust_source: default_script_source(),
            dependencies_text: String::new(),
        }
    }

    fn from_definition(script: &ScriptDefinition) -> Self {
        Self {
            id: script.id().as_str().to_string(),
            rust_source: script.rust_source().to_string(),
            dependencies_text: format_dependencies(script.dependencies()),
        }
    }
}

impl Default for ScriptForm {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRegistryPanel {
    selected_id: Option<String>,
    mode: FormMode,
    form: ScriptForm,
    status_message: Option<String>,
}

impl ScriptRegistryPanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(&self) -> FormMode {
        self.mode
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn visible_script_ids(&self, registry: &ScriptRegistry) -> Vec<String> {
        registry
            .iter()
            .map(|script| script.id().as_str().to_string())
            .collect()
    }

    pub fn select_script(
        &mut self,
        id: impl Into<String>,
        registry: &ScriptRegistry,
    ) -> Result<(), ScriptGuiError> {
        let id = id.into();
        registry.resolve(id.clone())?;
        self.selected_id = Some(id);
        self.mode = FormMode::Detail;
        self.status_message = None;
        Ok(())
    }

    pub fn selected_detail(&self, registry: &ScriptRegistry) -> Option<ScriptDetail> {
        let id = self.selected_id.as_ref()?;
        let script = registry.resolve(id.clone()).ok()?;

        Some(ScriptDetail {
            id: script.id().as_str().to_string(),
            rust_source: script.rust_source().to_string(),
            dependencies: script
                .dependencies()
                .iter()
                .map(format_dependency)
                .collect(),
        })
    }

    pub fn start_register(&mut self) {
        self.selected_id = None;
        self.mode = FormMode::Register;
        self.form = ScriptForm::empty();
        self.status_message = None;
    }

    pub fn start_edit(&mut self, registry: &ScriptRegistry) -> Result<(), ScriptGuiError> {
        let id = self
            .selected_id
            .as_ref()
            .ok_or(ScriptGuiError::MissingSelection)?;
        let script = registry.resolve(id.clone())?;

        self.form = ScriptForm::from_definition(script);
        self.mode = FormMode::Edit;
        self.status_message = None;
        Ok(())
    }

    pub fn set_form_id(&mut self, id: impl Into<String>) {
        self.form.id = id.into();
    }

    pub fn set_form_rust_source(&mut self, rust_source: impl Into<String>) {
        self.form.rust_source = rust_source.into();
    }

    pub fn set_form_dependencies(&mut self, dependencies: impl Into<String>) {
        self.form.dependencies_text = dependencies.into();
    }

    pub fn submit_form(
        &mut self,
        registry: &mut ScriptRegistry,
    ) -> Result<ScriptGuiAction, ScriptGuiError> {
        let result = match self.mode {
            FormMode::Register => self.register_script(registry),
            FormMode::Edit => self.update_script(registry),
            FormMode::Detail => Err(ScriptGuiError::UnsupportedMode(FormMode::Detail)),
        };

        self.record_result(result)
    }

    pub fn delete_selected(
        &mut self,
        registry: &mut ScriptRegistry,
    ) -> Result<ScriptGuiAction, ScriptGuiError> {
        let id = self
            .selected_id
            .as_ref()
            .ok_or(ScriptGuiError::MissingSelection)?
            .clone();
        let script_id = ScriptId::new(id.clone()).map_err(ScriptRegistryError::InvalidId)?;

        registry
            .remove(&script_id)
            .ok_or_else(|| ScriptRegistryError::UnknownId(script_id.clone()))?;

        self.selected_id = None;
        self.mode = FormMode::Detail;
        self.form = ScriptForm::empty();
        self.record_result(Ok(ScriptGuiAction::Deleted(id)))
    }

    pub fn ui(&mut self, ui: &mut Ui, registry: &mut ScriptRegistry) -> Option<ScriptGuiAction> {
        self.drop_missing_selection(registry);

        let mut action = None;
        egui::Frame::new()
            .fill(Color32::from_rgb(248, 249, 250))
            .inner_margin(egui::Margin::same(18))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_height(640.0);
                    self.draw_script_list(ui, registry);
                    ui.separator();
                    action = self.draw_page(ui, registry);
                });
            });

        action
    }

    fn register_script(
        &mut self,
        registry: &mut ScriptRegistry,
    ) -> Result<ScriptGuiAction, ScriptGuiError> {
        let dependencies = parse_dependencies(&self.form.dependencies_text)?;
        let script = ScriptDefinition::new(self.form.id.trim(), self.form.rust_source.clone())?
            .with_dependencies(dependencies);
        let id = script.id().as_str().to_string();

        registry.insert(script)?;
        self.selected_id = Some(id.clone());
        self.mode = FormMode::Detail;
        Ok(ScriptGuiAction::Created(id))
    }

    fn update_script(
        &mut self,
        registry: &mut ScriptRegistry,
    ) -> Result<ScriptGuiAction, ScriptGuiError> {
        let selected = self
            .selected_id
            .as_ref()
            .ok_or(ScriptGuiError::MissingSelection)?
            .clone();
        let script_id = ScriptId::new(selected.clone()).map_err(ScriptRegistryError::InvalidId)?;
        let dependencies = parse_dependencies(&self.form.dependencies_text)?;
        let script = registry
            .get_mut(&script_id)
            .ok_or_else(|| ScriptRegistryError::UnknownId(script_id.clone()))?;

        script.set_rust_source(self.form.rust_source.clone());
        script.set_dependencies(dependencies);
        self.mode = FormMode::Detail;

        Ok(ScriptGuiAction::Updated(selected))
    }

    fn record_result(
        &mut self,
        result: Result<ScriptGuiAction, ScriptGuiError>,
    ) -> Result<ScriptGuiAction, ScriptGuiError> {
        match result {
            Ok(action) => {
                self.status_message = Some(status_for_action(&action));
                Ok(action)
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn drop_missing_selection(&mut self, registry: &ScriptRegistry) {
        let Some(id) = &self.selected_id else {
            return;
        };

        if registry.resolve(id.clone()).is_err() {
            self.selected_id = None;
            if self.mode == FormMode::Edit {
                self.mode = FormMode::Detail;
            }
        }
    }

    fn draw_script_list(&mut self, ui: &mut Ui, registry: &ScriptRegistry) {
        ui.vertical(|ui| {
            ui.set_width(260.0);
            ui.horizontal(|ui| {
                ui.heading("Scripts");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Register").clicked() {
                        self.start_register();
                    }
                });
            });
            ui.add_space(8.0);

            ScrollArea::vertical().show(ui, |ui| {
                for id in self.visible_script_ids(registry) {
                    let selected = self.selected_id.as_deref() == Some(id.as_str());
                    if ui.selectable_label(selected, &id).clicked() {
                        let _ = self.select_script(id, registry);
                    }
                }
            });
        });
    }

    fn draw_page(&mut self, ui: &mut Ui, registry: &mut ScriptRegistry) -> Option<ScriptGuiAction> {
        ui.vertical(|ui| {
            ui.set_min_width(620.0);
            match self.mode {
                FormMode::Detail => self.draw_detail_page(ui, registry),
                FormMode::Register => self.draw_form_page(ui, registry, "Register Script"),
                FormMode::Edit => self.draw_form_page(ui, registry, "Edit Script"),
            }
        })
        .inner
    }

    fn draw_detail_page(
        &mut self,
        ui: &mut Ui,
        registry: &mut ScriptRegistry,
    ) -> Option<ScriptGuiAction> {
        let Some(detail) = self.selected_detail(registry) else {
            ui.heading("Details");
            ui.add_space(12.0);
            ui.label("Select a script or register a new one.");
            self.draw_status(ui);
            return None;
        };

        let mut action = None;
        ui.heading("Details");
        ui.add_space(8.0);
        ui.label(RichText::new(&detail.id).strong().size(18.0));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Edit").clicked() {
                if let Err(error) = self.start_edit(registry) {
                    self.status_message = Some(error.to_string());
                }
            }

            if ui.button("Delete").clicked() {
                match self.delete_selected(registry) {
                    Ok(gui_action) => action = Some(gui_action),
                    Err(error) => self.status_message = Some(error.to_string()),
                }
            }
        });

        ui.add_space(14.0);
        ui.label(RichText::new("Dependencies").strong());
        if detail.dependencies.is_empty() {
            ui.label("No dependencies");
        } else {
            for dependency in &detail.dependencies {
                ui.monospace(dependency);
            }
        }

        ui.add_space(14.0);
        ui.label(RichText::new("Rust Source").strong());
        let mut source = detail.rust_source;
        ui.add(
            TextEdit::multiline(&mut source)
                .code_editor()
                .desired_rows(18)
                .interactive(false),
        );
        self.draw_status(ui);

        action
    }

    fn draw_form_page(
        &mut self,
        ui: &mut Ui,
        registry: &mut ScriptRegistry,
        title: &str,
    ) -> Option<ScriptGuiAction> {
        let mut action = None;
        ui.heading(title);
        ui.add_space(8.0);

        ui.label("Script ID");
        ui.add_enabled(
            self.mode == FormMode::Register,
            TextEdit::singleline(&mut self.form.id).desired_width(f32::INFINITY),
        );

        ui.add_space(10.0);
        ui.label("Rust Source");
        ui.add(
            TextEdit::multiline(&mut self.form.rust_source)
                .code_editor()
                .desired_rows(16)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(10.0);
        ui.label("Dependencies");
        ui.add(
            TextEdit::multiline(&mut self.form.dependencies_text)
                .desired_rows(5)
                .desired_width(f32::INFINITY),
        )
        .on_hover_text(
            "Use one dependency per line: serde_json = \"1\", helper = path:/workspace/helper, or regex = manifest:{ version = \"1\" }",
        );

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                match self.submit_form(registry) {
                    Ok(gui_action) => action = Some(gui_action),
                    Err(error) => self.status_message = Some(error.to_string()),
                }
            }

            if ui.button("Cancel").clicked() {
                self.mode = FormMode::Detail;
                self.status_message = None;
            }
        });

        self.draw_status(ui);
        action
    }

    fn draw_status(&self, ui: &mut Ui) {
        if let Some(message) = &self.status_message {
            ui.add_space(10.0);
            ui.label(RichText::new(message).color(Color32::from_rgb(78, 93, 112)));
        }
    }
}

impl Default for ScriptRegistryPanel {
    fn default() -> Self {
        Self {
            selected_id: None,
            mode: FormMode::Detail,
            form: ScriptForm::empty(),
            status_message: None,
        }
    }
}

pub struct ScriptManagerApp {
    store: FileScriptStore,
    registry: ScriptRegistry,
    panel: ScriptRegistryPanel,
    store_status: String,
}

impl ScriptManagerApp {
    pub fn new(store: FileScriptStore) -> Self {
        let (registry, store_status) = match store.load() {
            Ok(registry) => {
                let count = registry.len();
                (registry, format!("{count} scripts loaded"))
            }
            Err(error) => (ScriptRegistry::new(), format!("Load error: {error}")),
        };

        Self {
            store,
            registry,
            panel: ScriptRegistryPanel::new(),
            store_status,
        }
    }

    pub fn from_registry(store: FileScriptStore, registry: ScriptRegistry) -> Self {
        let count = registry.len();
        Self {
            store,
            registry,
            panel: ScriptRegistryPanel::new(),
            store_status: format!("{count} scripts loaded"),
        }
    }

    pub fn registry(&self) -> &ScriptRegistry {
        &self.registry
    }

    pub fn panel(&self) -> &ScriptRegistryPanel {
        &self.panel
    }

    fn persist_after(&mut self, action: ScriptGuiAction) {
        match self.store.save(&self.registry) {
            Ok(()) => {
                self.store_status = format!(
                    "{}; saved to {}",
                    status_for_action(&action),
                    self.store.path().display()
                );
            }
            Err(error) => {
                self.store_status = format!("Save error after {}: {error}", action_verb(&action));
            }
        }
    }
}

impl eframe::App for ScriptManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("script_gui_toolbar")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Manual Script GUI");
                    ui.separator();
                    ui.monospace(abbreviate(&self.store.path().display().to_string(), 88))
                        .on_hover_text(self.store.path().display().to_string());
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(90, 145, 94), &self.store_status);
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                if let Some(action) = self.panel.ui(ui, &mut self.registry) {
                    self.persist_after(action);
                }
            });
    }
}

pub fn run_native(store: FileScriptStore) -> GuiResult {
    let title = format!("Manual Script GUI - {}", store.path().display());
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 760.0]),
        ..Default::default()
    };

    eframe::run_native(
        &title,
        native_options,
        Box::new(move |_creation_context| Ok(Box::new(ScriptManagerApp::new(store.clone())))),
    )
}

pub fn run_repository_script_gui(repository_root: impl AsRef<Path>) -> GuiResult {
    run_native(FileScriptStore::for_repository(repository_root))
}

fn parse_dependencies(source: &str) -> Result<Vec<ScriptDependency>, ScriptGuiError> {
    let mut dependencies = Vec::new();

    for (line_index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        dependencies.push(parse_dependency(line_index + 1, line)?);
    }

    Ok(dependencies)
}

fn parse_dependency(line_number: usize, line: &str) -> Result<ScriptDependency, ScriptGuiError> {
    let (name, value) = line.split_once('=').ok_or_else(|| {
        ScriptGuiError::Dependency(format!(
            "invalid dependency line {line_number}: expected `name = \"version\"`, `name = path:/dir`, or `name = manifest:{{ ... }}`"
        ))
    })?;
    let name = name.trim();
    let value = value.trim();

    if name.is_empty() {
        return Err(ScriptGuiError::Dependency(format!(
            "invalid dependency line {line_number}: dependency name cannot be empty"
        )));
    }

    if let Some(path) = value.strip_prefix("path:") {
        return Ok(ScriptDependency::path(name, PathBuf::from(path.trim())));
    }

    if let Some(manifest_value) = value.strip_prefix("manifest:") {
        return Ok(ScriptDependency::manifest_value(
            name,
            manifest_value.trim().to_string(),
        ));
    }

    Ok(ScriptDependency::version(name, unquote(value)))
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

fn format_dependencies(dependencies: &[ScriptDependency]) -> String {
    dependencies
        .iter()
        .map(format_dependency)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_dependency(dependency: &ScriptDependency) -> String {
    match &dependency.source {
        ScriptDependencySource::Version(version) => {
            format!("{} = \"{}\"", dependency.name, version)
        }
        ScriptDependencySource::Path(path) => {
            format!("{} = path:{}", dependency.name, path.display())
        }
        ScriptDependencySource::ManifestValue(value) => {
            format!("{} = manifest:{}", dependency.name, value)
        }
    }
}

fn status_for_action(action: &ScriptGuiAction) -> String {
    match action {
        ScriptGuiAction::Created(id) => format!("Created script {id}"),
        ScriptGuiAction::Updated(id) => format!("Updated script {id}"),
        ScriptGuiAction::Deleted(id) => format!("Deleted script {id}"),
    }
}

fn action_verb(action: &ScriptGuiAction) -> &'static str {
    match action {
        ScriptGuiAction::Created(_) => "create",
        ScriptGuiAction::Updated(_) => "update",
        ScriptGuiAction::Deleted(_) => "delete",
    }
}

fn default_script_source() -> String {
    "fn main(input_json: &str) -> String {\n    input_json.to_string()\n}".to_string()
}

fn abbreviate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{shortened}...")
    } else {
        shortened
    }
}
