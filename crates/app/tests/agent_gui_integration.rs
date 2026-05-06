use app::{AgentProfile, AgentRuntime, agent_gui_component};

#[test]
fn app_exposes_agent_gui_component_for_embedding() {
    let panel = agent_gui_component(vec![
        AgentProfile::new("codex", "Codex", AgentRuntime::Codex)
            .expect("agent profile should be valid"),
    ]);

    assert_eq!(panel.agents().len(), 1);
    assert!(panel.agent("codex").is_some());
}
