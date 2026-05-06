use eframe::egui;
use sandbox::{NetworkMode, SandboxPolicy, SandboxPreset};
use sandbox_gui::{SandboxDraft, SandboxGui, SandboxGuiMode};
use sandbox_registry::{SandboxDefinition, SandboxRegistry};

fn registry_with_two_sandboxes() -> SandboxRegistry {
    let mut registry = SandboxRegistry::new();
    registry
        .insert(
            SandboxDefinition::new("read-only", SandboxPolicy::read_only("/workspace")).unwrap(),
        )
        .unwrap();
    registry
        .insert(
            SandboxDefinition::new(
                "workspace-write",
                SandboxPolicy::workspace_write("/workspace"),
            )
            .unwrap(),
        )
        .unwrap();
    registry
}

#[test]
fn sandbox_gui_lists_and_selects_registry_definitions() {
    let mut gui = SandboxGui::new(registry_with_two_sandboxes());

    assert_eq!(gui.definition_ids(), vec!["read-only", "workspace-write"]);
    assert_eq!(gui.selected_id(), Some("read-only"));

    gui.select("workspace-write").unwrap();

    assert_eq!(gui.selected_id(), Some("workspace-write"));
    assert_eq!(
        gui.selected_definition().unwrap().policy().preset,
        SandboxPreset::WorkspaceWrite
    );
}

#[test]
fn sandbox_gui_registers_updates_and_deletes_sandboxes() {
    let mut gui = SandboxGui::new(SandboxRegistry::new());

    gui.start_registration();
    *gui.draft_mut() = SandboxDraft {
        id: "networked-readonly".to_string(),
        preset: SandboxPreset::ReadOnly,
        workspace_root: "/workspace".into(),
        network_enabled: true,
    };
    gui.save_draft().unwrap();

    let created = gui
        .registry()
        .resolve("networked-readonly")
        .expect("created sandbox should be present");
    assert_eq!(created.policy().preset, SandboxPreset::ReadOnly);
    assert_eq!(created.policy().network.mode, NetworkMode::Enabled);

    gui.start_editing_selected().unwrap();
    gui.draft_mut().id = "safe-write".to_string();
    gui.draft_mut().preset = SandboxPreset::WorkspaceWrite;
    gui.draft_mut().network_enabled = false;
    gui.save_draft().unwrap();

    assert!(gui.registry().resolve("networked-readonly").is_err());
    assert_eq!(
        gui.registry()
            .resolve("safe-write")
            .unwrap()
            .policy()
            .preset,
        SandboxPreset::WorkspaceWrite
    );

    gui.delete_selected().unwrap();

    assert!(gui.registry().is_empty());
    assert_eq!(gui.selected_id(), None);
}

#[test]
fn sandbox_gui_reports_validation_errors_without_changing_registry() {
    let mut gui = SandboxGui::new(registry_with_two_sandboxes());

    gui.start_registration();
    gui.draft_mut().id = "read-only".to_string();
    let error = gui.save_draft().unwrap_err();

    assert_eq!(error.to_string(), "duplicate sandbox id: read-only");
    assert_eq!(gui.definition_ids(), vec!["read-only", "workspace-write"]);
    assert!(matches!(gui.mode(), SandboxGuiMode::Register));
}

#[test]
fn danger_full_access_draft_keeps_the_full_access_policy_shape() {
    let draft = SandboxDraft {
        id: "danger".to_string(),
        preset: SandboxPreset::DangerFullAccess,
        workspace_root: "/workspace".into(),
        network_enabled: false,
    };

    let policy = draft.to_policy();

    assert_eq!(policy.preset, SandboxPreset::DangerFullAccess);
    assert_eq!(policy.network.mode, NetworkMode::Enabled);
    assert!(policy.filesystem.entries.is_empty());
}

#[test]
fn sandbox_gui_component_can_be_embedded_in_any_egui_ui() {
    fn accepts_component(gui: &mut SandboxGui, ui: &mut egui::Ui) -> egui::Response {
        gui.ui(ui).response
    }

    let mut gui = SandboxGui::new(registry_with_two_sandboxes());
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(920.0, 640.0),
        )),
        ..Default::default()
    });

    let mut response_rect = egui::Rect::NOTHING;
    egui::CentralPanel::default().show(&ctx, |ui| {
        response_rect = accepts_component(&mut gui, ui).rect;
    });

    let shapes: Vec<_> = ctx
        .end_pass()
        .shapes
        .into_iter()
        .map(|shape| shape.shape)
        .collect();

    assert!(response_rect.is_positive());
    assert!(shapes.iter().any(|shape| {
        matches!(shape, egui::Shape::Text(text) if text.galley.text() == "Sandboxes")
    }));
    assert!(shapes.iter().any(|shape| {
        matches!(shape, egui::Shape::Text(text) if text.galley.text() == "read-only")
    }));
}
