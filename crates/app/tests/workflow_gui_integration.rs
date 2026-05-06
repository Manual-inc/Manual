use app::workflow_gui_component;
use workflow_gui::{WorkflowPage, sample_registry};

#[test]
fn app_exposes_workflow_gui_component_for_embedding() {
    let gui = workflow_gui_component(sample_registry());

    assert_eq!(gui.page(), WorkflowPage::List);
    assert_eq!(gui.workflow_summaries()[0].id, "agent-handoff-pipeline");
}
