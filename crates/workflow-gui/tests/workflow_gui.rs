use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};
use workflow_gui::{
    EdgeDraft, NodeDraft, WorkflowDraft, WorkflowGui, WorkflowGuiEvent, WorkflowPage,
    color_for_node_kind, sample_agent_handoff_workflow, workflow_to_graph,
};
use workflow_registry::WorkflowRegistry;

#[test]
fn converts_workflow_into_graph_for_embedding_in_detail_pages() {
    let workflow = sample_workflow("debug-voc", "Debug VOC");

    let graph = workflow_to_graph(&workflow);

    assert_eq!(graph.nodes().len(), 3);
    assert_eq!(graph.nodes()[0].id, "trigger");
    assert_eq!(graph.nodes()[0].label, "trigger [entry, trigger]");
    assert_eq!(
        graph.nodes()[0].color.as_deref(),
        Some(color_for_node_kind(NodeKind::Trigger))
    );
    assert_eq!(graph.edges().len(), 2);
    assert_eq!(
        graph.edges()[1].label.as_deref(),
        Some("summarize (sequence)")
    );
}

#[test]
fn workflow_gui_state_lists_selects_edits_creates_and_deletes_workflows() {
    let mut gui = WorkflowGui::new(registry_with(sample_workflow("debug-voc", "Debug VOC")));

    assert_eq!(gui.page(), WorkflowPage::List);
    assert_eq!(gui.workflow_summaries().len(), 1);
    assert_eq!(gui.workflow_summaries()[0].id, "debug-voc");
    assert_eq!(gui.workflow_summaries()[0].node_count, 3);

    assert_eq!(
        gui.open_detail("debug-voc").expect("detail should open"),
        WorkflowGuiEvent::OpenedDetail("debug-voc".to_string())
    );
    assert_eq!(gui.page(), WorkflowPage::Detail);
    assert_eq!(
        gui.selected_workflow()
            .expect("selected workflow should exist")
            .name(),
        "Debug VOC"
    );

    assert_eq!(
        gui.start_edit_selected()
            .expect("selected workflow should be editable"),
        WorkflowGuiEvent::StartedEdit("debug-voc".to_string())
    );
    assert_eq!(gui.page(), WorkflowPage::Edit);
    gui.draft_mut().expect("edit draft should exist").name = "Debug VOC Updated".to_string();
    assert_eq!(
        gui.save_draft().expect("edit should save"),
        WorkflowGuiEvent::Updated("debug-voc".to_string())
    );
    assert_eq!(gui.page(), WorkflowPage::Detail);
    assert_eq!(
        gui.registry()
            .resolve("debug-voc")
            .expect("workflow should remain registered")
            .name(),
        "Debug VOC Updated"
    );

    assert_eq!(gui.start_create(), WorkflowGuiEvent::StartedCreate);
    let draft = gui.draft_mut().expect("create draft should exist");
    *draft = workflow_draft("triage-flow", "Triage Flow");
    assert_eq!(
        gui.save_draft().expect("create should save"),
        WorkflowGuiEvent::Created("triage-flow".to_string())
    );
    assert_eq!(gui.page(), WorkflowPage::Detail);
    assert_eq!(
        gui.selected_workflow()
            .expect("created workflow should be selected")
            .id(),
        "triage-flow"
    );

    assert_eq!(
        gui.delete_selected()
            .expect("selected workflow should delete"),
        WorkflowGuiEvent::Deleted("triage-flow".to_string())
    );
    assert_eq!(gui.page(), WorkflowPage::List);
    assert!(gui.registry().resolve("triage-flow").is_err());
    assert_eq!(gui.workflow_summaries().len(), 1);
}

#[test]
fn workflow_draft_round_trips_full_workflow_fields() {
    let workflow = sample_agent_handoff_workflow();

    let draft = WorkflowDraft::from_workflow(&workflow);
    let rebuilt = draft
        .build_workflow()
        .expect("draft should rebuild the original workflow shape");

    assert_eq!(rebuilt.id(), workflow.id());
    assert_eq!(rebuilt.name(), workflow.name());
    assert_eq!(rebuilt.goal(), workflow.goal());
    assert_eq!(
        rebuilt.entry_node().as_str(),
        workflow.entry_node().as_str()
    );
    assert_eq!(rebuilt.nodes(), workflow.nodes());
    assert_eq!(rebuilt.edges(), workflow.edges());
}

#[test]
fn workflow_gui_rejects_invalid_drafts_without_losing_current_state() {
    let mut gui = WorkflowGui::new(registry_with(sample_workflow("debug-voc", "Debug VOC")));
    gui.open_detail("debug-voc").expect("detail should open");
    gui.start_edit_selected()
        .expect("selected workflow should be editable");
    gui.draft_mut().expect("edit draft should exist").entry_node = "missing".to_string();

    let error = gui
        .save_draft()
        .expect_err("invalid workflow graph should be rejected");

    assert!(
        error
            .to_string()
            .contains("entry node does not exist: missing")
    );
    assert_eq!(gui.page(), WorkflowPage::Edit);
    assert_eq!(
        gui.registry()
            .resolve("debug-voc")
            .expect("original workflow should remain intact")
            .entry_node()
            .as_str(),
        "trigger"
    );
}

#[test]
fn workflow_gui_component_paints_list_and_detail_surfaces() {
    let mut gui = WorkflowGui::new(registry_with(sample_workflow("debug-voc", "Debug VOC")));

    let list_texts = run_gui_frame(&mut gui);
    assert!(list_texts.contains(&"Workflows".to_string()));
    assert!(list_texts.contains(&"Debug VOC".to_string()));
    assert!(list_texts.contains(&"New workflow".to_string()));

    gui.open_detail("debug-voc").expect("detail should open");
    let detail_texts = run_gui_frame(&mut gui);
    assert!(detail_texts.contains(&"Debug VOC".to_string()));
    assert!(detail_texts.contains(&"Edit".to_string()));
    assert!(detail_texts.contains(&"Delete".to_string()));
    assert!(detail_texts.contains(&"Nodes".to_string()));
    assert!(detail_texts.contains(&"Edges".to_string()));
}

fn registry_with(workflow: Workflow) -> WorkflowRegistry {
    let mut registry = WorkflowRegistry::new();
    registry
        .insert(workflow)
        .expect("sample workflow should register");
    registry
}

fn sample_workflow(id: &str, name: &str) -> Workflow {
    let trigger = Node::new("trigger", NodeKind::Trigger, "Receive the request.")
        .expect("trigger node should be valid");
    let inspect = Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.")
        .expect("inspect node should be valid")
        .with_runtime("agent")
        .with_input("request")
        .with_output("analysis");
    let report = Node::new("report", NodeKind::Artifact, "Write the final report.")
        .expect("report node should be valid")
        .with_input("analysis")
        .with_artifact("report.md");

    Workflow::new(
        id,
        name,
        "Find the root cause and produce a verified report.",
        "trigger",
        vec![trigger, inspect, report],
        vec![
            WorkflowEdge::sequence("trigger", "inspect"),
            WorkflowEdge::sequence("inspect", "report").with_label("summarize"),
        ],
    )
    .expect("workflow graph should be valid")
}

fn workflow_draft(id: &str, name: &str) -> WorkflowDraft {
    WorkflowDraft {
        id: id.to_string(),
        name: name.to_string(),
        goal: "Route a request into the right handling path.".to_string(),
        entry_node: "trigger".to_string(),
        nodes: vec![
            NodeDraft {
                id: "trigger".to_string(),
                kind: NodeKind::Trigger,
                description: "Receive the request.".to_string(),
                ..Default::default()
            },
            NodeDraft {
                id: "route".to_string(),
                kind: NodeKind::Condition,
                description: "Choose the next path.".to_string(),
                inputs: "request".to_string(),
                outputs: "decision".to_string(),
                ..Default::default()
            },
        ],
        edges: vec![EdgeDraft {
            source: "trigger".to_string(),
            target: "route".to_string(),
            kind: workflow::WorkflowEdgeKind::Sequence,
            label: "classify".to_string(),
        }],
    }
}

fn run_gui_frame(gui: &mut WorkflowGui) -> Vec<String> {
    let ctx = eframe::egui::Context::default();
    ctx.begin_pass(eframe::egui::RawInput {
        screen_rect: Some(eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(0.0, 0.0),
            eframe::egui::vec2(960.0, 720.0),
        )),
        ..Default::default()
    });

    eframe::egui::CentralPanel::default().show(&ctx, |ui| {
        gui.ui(ui);
    });

    ctx.end_pass()
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            eframe::egui::Shape::Text(text) => Some(text.galley.text().to_string()),
            _ => None,
        })
        .collect()
}
