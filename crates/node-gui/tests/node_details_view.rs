use eframe::egui::{CentralPanel, Context, RawInput, Rect, Shape, pos2, vec2};
use node::{Node, NodeKind};
use node_gui::NodeDetailsView;

fn sample_node() -> Node {
    Node::new(
        "inspect",
        NodeKind::LlmTask,
        "Inspect symptoms, logs, and likely code paths.",
    )
    .expect("node should be valid")
    .with_input("voc_ticket")
    .with_input("runtime_logs")
    .with_output("root_cause_hypothesis")
    .with_sandbox("read-only")
    .with_runtime("codex")
    .with_artifact("inspection-notes.md")
    .with_acceptance("Likely code paths are identified.")
}

fn render_node_details(node: &Node) -> Vec<String> {
    let ctx = Context::default();
    ctx.begin_pass(RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(480.0, 360.0))),
        ..Default::default()
    });

    CentralPanel::default().show(&ctx, |ui| {
        NodeDetailsView::default().ui(ui, node);
    });

    ctx.end_pass()
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            Shape::Text(text) => Some(text.galley.text().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn node_details_view_renders_node_identity_and_description() {
    let texts = render_node_details(&sample_node());

    assert!(texts.contains(&"inspect".to_string()));
    assert!(texts.contains(&"llm_task".to_string()));
    assert!(texts.contains(&"Inspect symptoms, logs, and likely code paths.".to_string()));
}

#[test]
fn node_details_view_renders_contract_metadata() {
    let texts = render_node_details(&sample_node());

    assert!(texts.contains(&"Inputs".to_string()));
    assert!(texts.contains(&"voc_ticket, runtime_logs".to_string()));
    assert!(texts.contains(&"Outputs".to_string()));
    assert!(texts.contains(&"root_cause_hypothesis".to_string()));
    assert!(texts.contains(&"Sandbox".to_string()));
    assert!(texts.contains(&"read-only".to_string()));
    assert!(texts.contains(&"Runtime".to_string()));
    assert!(texts.contains(&"codex".to_string()));
    assert!(texts.contains(&"Artifacts".to_string()));
    assert!(texts.contains(&"inspection-notes.md".to_string()));
    assert!(texts.contains(&"Acceptance".to_string()));
    assert!(texts.contains(&"Likely code paths are identified.".to_string()));
}

#[test]
fn node_details_view_marks_empty_contract_fields_as_none() {
    let node =
        Node::new("report", NodeKind::Artifact, "Write a report.").expect("node should be valid");

    let texts = render_node_details(&node);

    assert_eq!(
        texts.iter().filter(|text| text.as_str() == "None").count(),
        6
    );
}
