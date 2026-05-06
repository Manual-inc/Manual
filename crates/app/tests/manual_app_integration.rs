use app::{AppSection, manual_app};

#[test]
fn manual_app_composes_every_gui_crate_into_one_shell() {
    let app = manual_app();

    assert_eq!(
        app.sections(),
        &[
            AppSection::Workflows,
            AppSection::Jobs,
            AppSection::Nodes,
            AppSection::Scripts,
            AppSection::Sandboxes,
            AppSection::Agents,
        ]
    );
    assert_eq!(app.active_section(), AppSection::Workflows);
    assert_eq!(app.workflow_gui().workflow_summaries().len(), 1);
    assert_eq!(app.job_registry().len(), 1);
    assert_eq!(app.node_detail_node().id.as_str(), "routing_agent");
    assert_eq!(app.script_registry().len(), 1);
    assert_eq!(
        app.sandbox_gui().definition_ids(),
        vec!["read-only", "workspace-write"]
    );
    assert_eq!(app.agent_panel().agents().len(), 2);
}
