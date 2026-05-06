use agent_gui::{AgentDraft, AgentEvent, AgentManagerPanel, AgentPage, AgentProfile, AgentRuntime};
use eframe::egui::{CentralPanel, Context, RawInput, Rect, Shape, pos2, vec2};

fn sample_agent(id: &str, name: &str) -> AgentProfile {
    AgentProfile::new(id, name, AgentRuntime::Codex)
        .expect("sample agent should be valid")
        .with_model("gpt-5.5")
        .with_workdir("/tmp/manual")
        .with_execution_policy("read-only")
        .with_description("Repository inspection agent.")
}

fn run_panel_frame(panel: &mut AgentManagerPanel) -> Vec<Shape> {
    let ctx = Context::default();
    ctx.begin_pass(RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(960.0, 640.0))),
        ..Default::default()
    });

    CentralPanel::default().show(&ctx, |ui| {
        panel.ui(ui);
    });

    ctx.end_pass()
        .shapes
        .into_iter()
        .map(|clipped| clipped.shape)
        .collect()
}

fn assert_text_shape(shapes: &[Shape], expected: &str) {
    let mut texts = Vec::new();
    collect_text_shapes(shapes, &mut texts);

    assert!(
        texts.iter().any(|text| text.contains(expected)),
        "expected text shape containing {expected:?}, got {texts:?}"
    );
}

fn collect_text_shapes(shapes: &[Shape], texts: &mut Vec<String>) {
    for shape in shapes {
        match shape {
            Shape::Text(text) => texts.push(text.galley.text().to_string()),
            Shape::Vec(children) => collect_text_shapes(children, texts),
            _ => {}
        }
    }
}

#[test]
fn panel_starts_on_list_and_selects_detail_page() {
    let mut panel = AgentManagerPanel::new(vec![sample_agent("codex", "Codex")]);

    assert_eq!(panel.page(), &AgentPage::List);
    let event = panel
        .select_agent("codex")
        .expect("existing agent should be selectable");

    assert_eq!(event, AgentEvent::Selected("codex".to_string()));
    assert_eq!(
        panel.page(),
        &AgentPage::Detail {
            id: "codex".to_string()
        }
    );
    assert_eq!(
        panel.selected_agent().map(AgentProfile::display_name),
        Some("Codex")
    );
}

#[test]
fn registration_creates_profile_and_opens_detail_page() {
    let mut panel = AgentManagerPanel::new(Vec::new());
    panel.start_registration();
    panel.draft_mut().set_id("claude");
    panel.draft_mut().set_display_name("Claude");
    panel.draft_mut().set_runtime(AgentRuntime::Claude);
    panel.draft_mut().set_model("sonnet");
    panel.draft_mut().set_workdir("/workspace");
    panel.draft_mut().set_execution_policy("dontAsk");
    panel
        .draft_mut()
        .set_description("Implementation and review agent.");

    let event = panel
        .submit_draft()
        .expect("valid draft should create an agent");

    assert_eq!(event, AgentEvent::Created("claude".to_string()));
    assert_eq!(panel.agents().len(), 1);
    assert_eq!(panel.agents()[0].runtime(), &AgentRuntime::Claude);
    assert_eq!(
        panel.page(),
        &AgentPage::Detail {
            id: "claude".to_string()
        }
    );
}

#[test]
fn editing_updates_profile_and_keeps_component_embeddable_state() {
    let mut panel = AgentManagerPanel::new(vec![sample_agent("codex", "Codex")]);
    panel
        .start_editing("codex")
        .expect("existing agent should be editable");
    panel.draft_mut().set_display_name("Codex Planner");
    panel.draft_mut().set_model("gpt-5.5-high");
    panel.draft_mut().set_enabled(false);

    let event = panel
        .submit_draft()
        .expect("valid edit draft should update the agent");

    assert_eq!(event, AgentEvent::Updated("codex".to_string()));
    let agent = panel
        .agent("codex")
        .expect("updated agent should remain addressable by id");
    assert_eq!(agent.display_name(), "Codex Planner");
    assert_eq!(agent.model(), "gpt-5.5-high");
    assert!(!agent.enabled());
}

#[test]
fn delete_confirmation_removes_profile_and_returns_to_list() {
    let mut panel = AgentManagerPanel::new(vec![
        sample_agent("codex", "Codex"),
        sample_agent("claude", "Claude"),
    ]);

    panel
        .request_delete("codex")
        .expect("existing agent should move to delete confirmation");
    assert_eq!(
        panel.page(),
        &AgentPage::DeleteConfirm {
            id: "codex".to_string()
        }
    );

    let event = panel
        .confirm_delete()
        .expect("delete confirmation should remove the agent");

    assert_eq!(event, AgentEvent::Deleted("codex".to_string()));
    assert!(panel.agent("codex").is_none());
    assert!(panel.agent("claude").is_some());
    assert_eq!(panel.page(), &AgentPage::List);
}

#[test]
fn duplicate_or_invalid_drafts_are_rejected() {
    let mut panel = AgentManagerPanel::new(vec![sample_agent("codex", "Codex")]);
    panel.start_registration();
    panel.draft_mut().set_id("codex");
    panel.draft_mut().set_display_name("Duplicate Codex");

    assert!(
        panel
            .submit_draft()
            .expect_err("duplicate ids should be rejected")
            .to_string()
            .contains("duplicate agent id")
    );

    let empty_name = AgentDraft::new()
        .with_id("new-agent")
        .with_runtime(AgentRuntime::Custom("local".to_string()));
    assert!(
        empty_name
            .validate()
            .expect_err("empty display name should be rejected")
            .to_string()
            .contains("display name")
    );
}

#[test]
fn ui_renders_list_detail_and_form_pages_as_egui_components() {
    let mut panel = AgentManagerPanel::new(vec![sample_agent("codex", "Codex")]);
    let list_shapes = run_panel_frame(&mut panel);
    assert_text_shape(&list_shapes, "Agents");
    assert_text_shape(&list_shapes, "Register");
    assert_text_shape(&list_shapes, "Codex");

    panel
        .select_agent("codex")
        .expect("existing agent should be selectable");
    let detail_shapes = run_panel_frame(&mut panel);
    assert_text_shape(&detail_shapes, "Provider");
    assert_text_shape(&detail_shapes, "Repository inspection agent.");

    panel.start_registration();
    let form_shapes = run_panel_frame(&mut panel);
    assert_text_shape(&form_shapes, "Register Agent");
    assert_text_shape(&form_shapes, "Save");
}
