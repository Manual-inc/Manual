use job::{Job, JobStatus};
use job_gui::{JobGui, JobGuiState, JobSummaryRow, job_summary_rows};
use job_registry::JobRegistry;
use node::{Node, NodeKind};
use workflow::{Workflow, WorkflowEdge};

#[test]
fn summary_rows_present_jobs_for_the_list_page() {
    let registry = sample_registry();

    let rows = job_summary_rows(&registry);

    assert_eq!(
        rows,
        vec![
            JobSummaryRow {
                id: "job-001".to_string(),
                workflow_id: "debug-voc".to_string(),
                status: JobStatus::Running,
                completed_nodes: 1,
                total_nodes: 3,
                running_nodes: 1,
                failed_nodes: 0,
            },
            JobSummaryRow {
                id: "job-002".to_string(),
                workflow_id: "release-notes".to_string(),
                status: JobStatus::Created,
                completed_nodes: 0,
                total_nodes: 1,
                running_nodes: 0,
                failed_nodes: 0,
            },
        ]
    );
}

#[test]
fn state_selects_existing_jobs_and_clears_missing_selection() {
    let registry = sample_registry();
    let mut state = JobGuiState::new();

    state.select_job("job-001", &registry);

    assert_eq!(state.selected_job_id(), Some("job-001"));
    assert_eq!(
        state
            .selected_job(&registry)
            .expect("selected job should resolve")
            .status(),
        JobStatus::Running
    );

    state.select_job("missing", &registry);

    assert_eq!(state.selected_job_id(), None);
    assert!(state.selected_job(&registry).is_none());
}

#[test]
fn state_defaults_to_first_job_when_selection_is_empty_or_stale() {
    let registry = sample_registry();
    let mut state = JobGuiState::new();

    state.ensure_selection(&registry);

    assert_eq!(state.selected_job_id(), Some("job-001"));

    state.select_job("job-001", &registry);
    let empty_registry = JobRegistry::new();

    state.ensure_selection(&empty_registry);

    assert_eq!(state.selected_job_id(), None);
}

#[test]
fn job_gui_component_renders_list_and_selected_detail_page() {
    let registry = sample_registry();
    let mut state = JobGuiState::new();
    let mut gui = JobGui::new();

    let (selected_job_id, shapes) = run_job_gui_frame(&mut gui, &mut state, &registry);

    assert_eq!(selected_job_id.as_deref(), Some("job-001"));
    assert_contains_text(&shapes, "Jobs");
    assert_contains_text(&shapes, "Job Detail");
    assert_contains_text(&shapes, "job-001");
    assert_contains_text(&shapes, "debug-voc");
    assert_contains_text(&shapes, "running");
    assert_contains_text(&shapes, "1/3 complete");
    assert_contains_text(&shapes, "trigger");
    assert_contains_text(&shapes, "succeeded");
    assert_contains_text(&shapes, r#"{"ticket":"VOC-1"}"#);
}

#[test]
fn job_gui_component_renders_empty_list_and_detail_states() {
    let registry = JobRegistry::new();
    let mut state = JobGuiState::new();
    let mut gui = JobGui::new();

    let (selected_job_id, shapes) = run_job_gui_frame(&mut gui, &mut state, &registry);

    assert_eq!(selected_job_id, None);
    assert_contains_text(&shapes, "No jobs registered");
    assert_contains_text(&shapes, "Select a job to inspect details");
}

fn run_job_gui_frame(
    gui: &mut JobGui,
    state: &mut JobGuiState,
    registry: &JobRegistry,
) -> (Option<String>, Vec<eframe::egui::Shape>) {
    let ctx = eframe::egui::Context::default();
    ctx.begin_pass(eframe::egui::RawInput {
        screen_rect: Some(eframe::egui::Rect::from_min_size(
            eframe::egui::pos2(0.0, 0.0),
            eframe::egui::vec2(900.0, 620.0),
        )),
        ..Default::default()
    });

    let mut selected_job_id = None;
    eframe::egui::CentralPanel::default().show(&ctx, |ui| {
        let response = gui.ui(ui, state, registry);
        selected_job_id = response.selected_job_id;
    });

    let shapes = ctx
        .end_pass()
        .shapes
        .into_iter()
        .map(|clipped| clipped.shape)
        .collect();

    (selected_job_id, shapes)
}

fn assert_contains_text(shapes: &[eframe::egui::Shape], value: &str) {
    let texts = shapes
        .iter()
        .filter_map(|shape| match shape {
            eframe::egui::Shape::Text(text) => Some(text.galley.text().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        texts.iter().any(|text| text == value),
        "expected text shape containing {value:?}; rendered texts: {texts:?}"
    );
}

fn sample_registry() -> JobRegistry {
    let debug_workflow = Workflow::new(
        "debug-voc",
        "Debug VOC",
        "Find the root cause and produce a verified report.",
        "trigger",
        vec![
            Node::new("trigger", NodeKind::Trigger, "Receive the request.").unwrap(),
            Node::new("inspect", NodeKind::LlmTask, "Inspect the repository.").unwrap(),
            Node::new("report", NodeKind::Artifact, "Write the final report.").unwrap(),
        ],
        vec![
            WorkflowEdge::sequence("trigger", "inspect"),
            WorkflowEdge::sequence("inspect", "report"),
        ],
    )
    .unwrap();
    let release_workflow = Workflow::new(
        "release-notes",
        "Release Notes",
        "Summarize release changes.",
        "trigger",
        vec![Node::new("trigger", NodeKind::Trigger, "Receive release inputs.").unwrap()],
        Vec::new(),
    )
    .unwrap();
    let mut first = Job::new("job-001", &debug_workflow, r#"{"ticket":"VOC-1"}"#).unwrap();
    first.start_node("trigger").unwrap();
    first.succeed_node("trigger").unwrap();
    first.mark_node_ready("inspect").unwrap();
    first.start_node("inspect").unwrap();
    let second = Job::new("job-002", &release_workflow, r#"{"release":"1.0"}"#).unwrap();
    let mut registry = JobRegistry::new();

    registry.insert(first).unwrap();
    registry.insert(second).unwrap();

    registry
}
