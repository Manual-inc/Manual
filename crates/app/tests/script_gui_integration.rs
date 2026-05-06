use app::script_gui_component;

#[test]
fn app_exposes_script_gui_component_for_embedding() {
    let panel = script_gui_component();

    assert!(panel.status_message().is_none());
}
