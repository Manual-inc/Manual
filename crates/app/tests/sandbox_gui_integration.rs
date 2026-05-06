use app::sandbox_gui_component;
use sandbox::SandboxPolicy;
use sandbox_registry::{SandboxDefinition, SandboxRegistry};

#[test]
fn app_exposes_sandbox_gui_component_for_embedding() {
    let mut registry = SandboxRegistry::new();
    registry
        .insert(
            SandboxDefinition::new("read-only", SandboxPolicy::read_only("/workspace")).unwrap(),
        )
        .unwrap();

    let gui = sandbox_gui_component(registry);

    assert_eq!(gui.selected_id(), Some("read-only"));
    assert_eq!(gui.definition_ids(), vec!["read-only"]);
}
