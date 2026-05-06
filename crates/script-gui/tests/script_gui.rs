use eframe::egui::{self, CentralPanel, Color32, Context, RawInput, Shape};
use script::{ScriptDependency, ScriptDependencySource};
use script_gui::{FormMode, ScriptGuiAction, ScriptRegistryPanel};
use script_registry::{ScriptDefinition, ScriptRegistry};

fn registry_with_scripts() -> ScriptRegistry {
    let mut registry = ScriptRegistry::new();
    registry
        .insert(
            ScriptDefinition::new(
                "normalize",
                "fn main(input: &str) -> String { input.trim().into() }",
            )
            .unwrap()
            .with_dependency(ScriptDependency::version("serde_json", "1")),
        )
        .unwrap();
    registry
        .insert(
            ScriptDefinition::new("summarize", "fn main(_: &str) -> String { String::new() }")
                .unwrap(),
        )
        .unwrap();
    registry
}

#[test]
fn panel_lists_scripts_selects_details_and_summarizes_dependencies() {
    let registry = registry_with_scripts();
    let mut panel = ScriptRegistryPanel::new();

    assert_eq!(
        panel.visible_script_ids(&registry),
        ["normalize".to_string(), "summarize".to_string()]
    );

    panel.select_script("normalize", &registry).unwrap();
    let detail = panel
        .selected_detail(&registry)
        .expect("selected script should have detail");

    assert_eq!(detail.id, "normalize");
    assert!(detail.rust_source.contains("trim"));
    assert_eq!(detail.dependencies, ["serde_json = \"1\"".to_string()]);
    assert_eq!(panel.mode(), FormMode::Detail);
}

#[test]
fn panel_registers_updates_and_deletes_scripts() {
    let mut registry = ScriptRegistry::new();
    let mut panel = ScriptRegistryPanel::new();

    panel.start_register();
    panel.set_form_id("echo-json");
    panel.set_form_rust_source("fn main(input: &str) -> String { input.to_string() }");
    panel.set_form_dependencies("serde_json = \"1\"\nhelper = path:/workspace/helper");

    assert_eq!(
        panel.submit_form(&mut registry).unwrap(),
        ScriptGuiAction::Created("echo-json".to_string())
    );
    let created = registry.resolve("echo-json").unwrap();
    assert_eq!(created.dependencies().len(), 2);
    assert!(matches!(
        &created.dependencies()[0].source,
        ScriptDependencySource::Version(version) if version == "1"
    ));
    assert!(matches!(
        &created.dependencies()[1].source,
        ScriptDependencySource::Path(path) if path == std::path::Path::new("/workspace/helper")
    ));

    panel.start_edit(&registry).unwrap();
    panel.set_form_rust_source("fn main(_: &str) -> String { \"updated\".into() }");

    assert_eq!(
        panel.submit_form(&mut registry).unwrap(),
        ScriptGuiAction::Updated("echo-json".to_string())
    );
    assert!(
        registry
            .resolve("echo-json")
            .unwrap()
            .rust_source()
            .contains("updated")
    );

    assert_eq!(
        panel.delete_selected(&mut registry).unwrap(),
        ScriptGuiAction::Deleted("echo-json".to_string())
    );
    assert!(registry.is_empty());
    assert!(panel.selected_detail(&registry).is_none());
}

#[test]
fn panel_reports_validation_errors_without_mutating_registry() {
    let mut registry = ScriptRegistry::new();
    let mut panel = ScriptRegistryPanel::new();

    panel.start_register();
    panel.set_form_id("has whitespace");
    panel.set_form_rust_source("fn main(_: &str) -> String { String::new() }");

    let error = panel
        .submit_form(&mut registry)
        .expect_err("invalid id should be rejected");

    assert_eq!(
        error.to_string(),
        "script id cannot contain whitespace: has whitespace"
    );
    assert!(registry.is_empty());
    assert!(
        panel
            .status_message()
            .unwrap()
            .contains("cannot contain whitespace")
    );
}

#[test]
fn panel_ui_paints_list_detail_and_component_actions() {
    let mut registry = registry_with_scripts();
    let mut panel = ScriptRegistryPanel::new();
    panel.select_script("normalize", &registry).unwrap();

    let shapes = render_panel_frame(&mut panel, &mut registry);

    assert_text_shape(&shapes, "Scripts");
    assert_text_shape(&shapes, "Register");
    assert_text_shape(&shapes, "normalize");
    assert_text_shape(&shapes, "Details");
    assert_text_shape(&shapes, "Edit");
    assert!(
        shapes.iter().any(
            |shape| matches!(shape, Shape::Rect(rect) if rect.fill == Color32::from_rgb(248, 249, 250))
        ),
        "component should paint a light work surface"
    );
}

fn render_panel_frame(
    panel: &mut ScriptRegistryPanel,
    registry: &mut ScriptRegistry,
) -> Vec<Shape> {
    let ctx = Context::default();
    ctx.begin_pass(RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(1024.0, 720.0),
        )),
        ..Default::default()
    });

    CentralPanel::default().show(&ctx, |ui| {
        panel.ui(ui, registry);
    });

    ctx.end_pass()
        .shapes
        .into_iter()
        .map(|clipped| clipped.shape)
        .collect()
}

fn assert_text_shape(shapes: &[Shape], expected: &str) {
    assert!(
        shapes
            .iter()
            .any(|shape| { matches!(shape, Shape::Text(text) if text.galley.text() == expected) }),
        "expected text shape containing {expected:?}"
    );
}
