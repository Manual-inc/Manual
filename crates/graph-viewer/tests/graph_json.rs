use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use manual_graph_viewer::{
    Edge, Graph, GraphLayout, GraphLoadError, GraphView, Node, PanOffset, SmoothZoom, ZoomLevel,
    circular_layout, load_graph_file,
};

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {actual} to be approximately {expected}"
    );
}

#[test]
fn parses_nodes_and_edges_with_aliases() {
    let graph = Graph::from_json_str(
        r##"
        {
          "nodes": [
            { "id": "root", "label": "Root", "color": "#4f8cff" },
            { "id": "leaf" }
          ],
          "edges": [
            { "from": "root", "to": "leaf", "label": "feeds" }
          ]
        }
        "##,
    )
    .expect("graph should parse");

    assert_eq!(graph.nodes().len(), 2);
    assert_eq!(graph.nodes()[0].id, "root");
    assert_eq!(graph.nodes()[0].label, "Root");
    assert_eq!(graph.nodes()[0].color.as_deref(), Some("#4f8cff"));
    assert_eq!(graph.nodes()[1].label, "leaf");

    assert_eq!(graph.edges().len(), 1);
    assert_eq!(graph.edges()[0].source, "root");
    assert_eq!(graph.edges()[0].target, "leaf");
    assert_eq!(graph.edges()[0].label.as_deref(), Some("feeds"));
}

#[test]
fn parses_graph_when_edges_are_omitted() {
    let graph = Graph::from_json_str(
        r#"
        {
          "nodes": [
            { "id": "only", "label": "Only Node" }
          ]
        }
        "#,
    )
    .expect("edges should default to an empty list");

    assert_eq!(graph.nodes().len(), 1);
    assert_eq!(graph.edges().len(), 0);
    assert_eq!(graph.nodes()[0].id, "only");
    assert_eq!(graph.nodes()[0].label, "Only Node");
}

#[test]
fn constructs_graph_from_nodes_and_edges_directly() {
    let graph = Graph::new(
        vec![
            Node {
                id: "root".to_string(),
                label: "Root".to_string(),
                color: Some("#4f8cff".to_string()),
            },
            Node {
                id: "leaf".to_string(),
                label: "Leaf".to_string(),
                color: None,
            },
        ],
        vec![Edge {
            source: "root".to_string(),
            target: "leaf".to_string(),
            label: Some("feeds".to_string()),
        }],
    )
    .expect("direct graph construction should validate and preserve graph data");

    assert_eq!(graph.nodes().len(), 2);
    assert_eq!(graph.nodes()[0].id, "root");
    assert_eq!(graph.nodes()[0].label, "Root");
    assert_eq!(graph.nodes()[0].color.as_deref(), Some("#4f8cff"));
    assert_eq!(graph.edges().len(), 1);
    assert_eq!(graph.edges()[0].source, "root");
    assert_eq!(graph.edges()[0].target, "leaf");
    assert_eq!(graph.edges()[0].label.as_deref(), Some("feeds"));
}

#[test]
fn direct_graph_construction_reuses_endpoint_validation() {
    let err = Graph::new(
        vec![Node {
            id: "known".to_string(),
            label: "Known".to_string(),
            color: None,
        }],
        vec![Edge {
            source: "known".to_string(),
            target: "missing".to_string(),
            label: None,
        }],
    )
    .expect_err("missing direct graph endpoints should be rejected");

    assert_eq!(
        err,
        GraphLoadError::MissingEndpoint {
            edge_index: 0,
            endpoint: "target",
            node_id: "missing".to_string()
        }
    );
}

#[test]
fn invalid_json_is_reported_as_a_graph_load_error() {
    let err = Graph::from_json_str("{ definitely not valid json")
        .expect_err("invalid JSON should be rejected");

    match err {
        GraphLoadError::InvalidJson(message) => {
            assert!(!message.is_empty());
        }
        other => panic!("expected InvalidJson error, got {other:?}"),
    }
}

#[test]
fn graph_load_errors_have_actionable_display_messages() {
    let duplicate = GraphLoadError::DuplicateNode("same".to_string());
    assert_eq!(duplicate.to_string(), "duplicate node id: same");

    let missing = GraphLoadError::MissingEndpoint {
        edge_index: 3,
        endpoint: "source",
        node_id: "ghost".to_string(),
    };
    assert_eq!(
        missing.to_string(),
        "edge 3 references missing source node: ghost"
    );

    let invalid = GraphLoadError::InvalidJson("expected value".to_string());
    assert_eq!(invalid.to_string(), "invalid graph JSON: expected value");

    let io = GraphLoadError::Io("permission denied".to_string());
    assert_eq!(
        io.to_string(),
        "could not read graph JSON: permission denied"
    );
}

#[test]
fn rejects_duplicate_node_ids() {
    let err = Graph::from_json_str(
        r#"
        {
          "nodes": [
            { "id": "same" },
            { "id": "same" }
          ],
          "edges": []
        }
        "#,
    )
    .expect_err("duplicate IDs should be rejected");

    assert_eq!(err, GraphLoadError::DuplicateNode("same".to_string()));
}

#[test]
fn rejects_edges_that_point_at_missing_source_nodes() {
    let err = Graph::from_json_str(
        r#"
        {
          "nodes": [{ "id": "known" }],
          "edges": [{ "source": "missing", "target": "known" }]
        }
        "#,
    )
    .expect_err("missing source endpoint should be rejected");

    assert_eq!(
        err,
        GraphLoadError::MissingEndpoint {
            edge_index: 0,
            endpoint: "source",
            node_id: "missing".to_string()
        }
    );
}

#[test]
fn rejects_edges_that_point_at_missing_nodes() {
    let err = Graph::from_json_str(
        r#"
        {
          "nodes": [{ "id": "known" }],
          "edges": [{ "source": "known", "target": "missing" }]
        }
        "#,
    )
    .expect_err("missing edge endpoint should be rejected");

    assert_eq!(
        err,
        GraphLoadError::MissingEndpoint {
            edge_index: 0,
            endpoint: "target",
            node_id: "missing".to_string()
        }
    );
}

#[test]
fn empty_layout_has_no_positions() {
    let graph =
        Graph::from_json_str(r#"{ "nodes": [], "edges": [] }"#).expect("empty graph should parse");

    let layout = circular_layout(&graph);

    assert!(layout.position("missing").is_none());
}

#[test]
fn single_node_layout_is_centered() {
    let graph = Graph::from_json_str(r#"{ "nodes": [{ "id": "solo" }], "edges": [] }"#)
        .expect("single node graph should parse");

    let layout = circular_layout(&graph);
    let point = layout
        .position("solo")
        .expect("single node should have a position");

    assert_approx_eq(point.x, 0.0);
    assert_approx_eq(point.y, 0.0);
}

#[test]
fn circular_layout_places_every_node_at_a_finite_position() {
    let graph = Graph::from_json_str(
        r#"
        {
          "nodes": [
            { "id": "a" },
            { "id": "b" },
            { "id": "c" }
          ],
          "edges": []
        }
        "#,
    )
    .expect("graph should parse");

    let layout = circular_layout(&graph);

    assert!(layout.position("a").is_some());
    assert!(layout.position("b").is_some());
    assert!(layout.position("c").is_some());

    for node in graph.nodes() {
        let point = layout
            .position(&node.id)
            .expect("node should have a position");
        assert!(point.x.is_finite());
        assert!(point.y.is_finite());
    }
}

#[test]
fn circular_layout_places_nodes_on_unit_circle_in_input_order() {
    let graph = Graph::from_json_str(
        r#"
        {
          "nodes": [
            { "id": "east" },
            { "id": "north" },
            { "id": "west" },
            { "id": "south" }
          ],
          "edges": []
        }
        "#,
    )
    .expect("graph should parse");

    let layout = circular_layout(&graph);

    let east = layout.position("east").expect("east should be placed");
    assert_approx_eq(east.x, 1.0);
    assert_approx_eq(east.y, 0.0);

    let north = layout.position("north").expect("north should be placed");
    assert_approx_eq(north.x, 0.0);
    assert_approx_eq(north.y, 1.0);

    let west = layout.position("west").expect("west should be placed");
    assert_approx_eq(west.x, -1.0);
    assert_approx_eq(west.y, 0.0);

    let south = layout.position("south").expect("south should be placed");
    assert_approx_eq(south.x, 0.0);
    assert_approx_eq(south.y, -1.0);
}

#[test]
fn load_graph_file_reads_latest_json_from_disk() {
    let path = std::env::temp_dir().join(format!(
        "manual-graph-viewer-{}.json",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));

    fs::write(
        &path,
        r#"
        {
          "nodes": [{ "id": "before" }],
          "edges": []
        }
        "#,
    )
    .expect("test graph should be written");

    assert_eq!(
        load_graph_file(&path)
            .expect("initial graph should load")
            .nodes()[0]
            .id,
        "before"
    );

    fs::write(
        &path,
        r#"
        {
          "nodes": [{ "id": "after" }],
          "edges": []
        }
        "#,
    )
    .expect("test graph should be updated");

    assert_eq!(
        load_graph_file(&path)
            .expect("updated graph should load")
            .nodes()[0]
            .id,
        "after"
    );

    let _ = fs::remove_file(path);
}

#[test]
fn load_graph_file_reports_io_errors() {
    let missing = std::env::temp_dir().join(format!(
        "manual-graph-viewer-missing-{}.json",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));

    let err = load_graph_file(&missing).expect_err("missing file should return an IO error");

    match err {
        GraphLoadError::Io(message) => {
            assert!(!message.is_empty());
        }
        other => panic!("expected Io error, got {other:?}"),
    }
}

#[test]
fn zoom_level_steps_clamps_and_resets() {
    let mut zoom = ZoomLevel::default();

    assert_eq!(zoom.value(), 1.0);
    assert_eq!(zoom.percent(), 100);

    zoom.zoom_in();
    assert!(zoom.value() > 1.0);
    assert_eq!(zoom.percent(), 125);

    zoom.zoom_out();
    assert_eq!(zoom.value(), 1.0);

    for _ in 0..20 {
        zoom.zoom_out();
    }
    assert_eq!(zoom.value(), ZoomLevel::MIN);
    assert_eq!(zoom.percent(), 25);

    for _ in 0..40 {
        zoom.zoom_in();
    }
    assert_eq!(zoom.value(), ZoomLevel::MAX);
    assert_eq!(zoom.percent(), 400);

    zoom.reset();
    assert_eq!(zoom.value(), 1.0);
    assert_eq!(zoom.percent(), 100);
}

#[test]
fn zoom_level_scroll_direction_changes_discrete_zoom() {
    let mut zoom = ZoomLevel::default();

    zoom.zoom_by_scroll(1.0);
    assert_eq!(zoom.percent(), 125);

    zoom.zoom_by_scroll(-1.0);
    assert_eq!(zoom.percent(), 100);

    zoom.zoom_by_scroll(0.0);
    assert_eq!(zoom.percent(), 100);
}

#[test]
fn zoom_level_new_and_scaling_are_clamped() {
    assert_eq!(ZoomLevel::new(0.01).value(), ZoomLevel::MIN);
    assert_eq!(ZoomLevel::new(99.0).value(), ZoomLevel::MAX);
    assert_eq!(ZoomLevel::new(2.0).value(), 2.0);

    assert_eq!(ZoomLevel::new(1.0).scaled_by(0.01).value(), ZoomLevel::MIN);
    assert_eq!(ZoomLevel::new(2.0).scaled_by(99.0).value(), ZoomLevel::MAX);
    assert_eq!(ZoomLevel::new(2.0).scaled_by(0.5).value(), 1.0);
}

#[test]
fn zoom_level_lerp_toward_respects_interpolation_bounds() {
    let start = ZoomLevel::new(1.0);
    let target = ZoomLevel::new(3.0);

    assert_eq!(start.lerp_toward(target, -1.0).value(), 1.0);
    assert_eq!(start.lerp_toward(target, 0.0).value(), 1.0);
    assert_eq!(start.lerp_toward(target, 0.5).value(), 2.0);
    assert_eq!(start.lerp_toward(target, 1.0).value(), 3.0);
    assert_eq!(start.lerp_toward(target, 2.0).value(), 3.0);
}

#[test]
fn smooth_zoom_animates_displayed_zoom_toward_target() {
    let mut zoom = SmoothZoom::default();

    zoom.zoom_in();

    assert_eq!(zoom.displayed().percent(), 100);
    assert_eq!(zoom.target().percent(), 125);

    assert!(zoom.advance(0.5));
    assert!(zoom.displayed().value() > 1.0);
    assert!(zoom.displayed().value() < zoom.target().value());

    for _ in 0..24 {
        zoom.advance(0.5);
    }

    assert_eq!(zoom.displayed(), zoom.target());
}

#[test]
fn smooth_zoom_reset_animates_back_to_default_target() {
    let mut zoom = SmoothZoom::new(ZoomLevel::new(2.0));

    zoom.zoom_in();
    zoom.jump_to_target();
    assert!(zoom.displayed().value() > 2.0);

    zoom.reset();

    assert!(zoom.displayed().value() > 1.0);
    assert_eq!(zoom.target(), ZoomLevel::default());

    assert!(zoom.advance(0.5));
    assert!(zoom.displayed().value() < 2.5);
    assert!(zoom.displayed().value() > zoom.target().value());
}

#[test]
fn smooth_zoom_zoom_out_updates_target_without_jumping_displayed_zoom() {
    let mut zoom = SmoothZoom::new(ZoomLevel::new(2.0));

    zoom.zoom_out();

    assert_eq!(zoom.displayed().value(), 2.0);
    assert_eq!(zoom.target().value(), 1.6);
}

#[test]
fn smooth_zoom_snaps_to_target_when_advance_gets_close_enough() {
    let mut zoom = SmoothZoom::default();

    zoom.zoom_in();

    assert!(!zoom.advance(0.999));
    assert_eq!(zoom.displayed(), zoom.target());
}

#[test]
fn smooth_zoom_uses_continuous_scroll_targets() {
    let mut zoom = SmoothZoom::default();

    zoom.zoom_by_scroll(12.0);

    assert!(zoom.target().value() > 1.0);
    assert!(zoom.target().value() < 1.25);
}

#[test]
fn smooth_zoom_ignores_zero_scroll_and_clamps_large_scroll() {
    let mut zoom = SmoothZoom::default();

    zoom.zoom_by_scroll(0.0);
    assert_eq!(zoom.target(), ZoomLevel::default());

    zoom.zoom_by_scroll(10_000.0);
    assert_eq!(zoom.target().value(), ZoomLevel::MAX);

    zoom.zoom_by_scroll(-10_000.0);
    assert_eq!(zoom.target().value(), ZoomLevel::MIN);
}

#[test]
fn pan_offset_accumulates_drag_delta_and_resets() {
    let mut pan = PanOffset::default();

    assert_eq!(pan.x(), 0.0);
    assert_eq!(pan.y(), 0.0);

    pan.pan_by(24.0, -12.0);
    assert_eq!(pan.x(), 24.0);
    assert_eq!(pan.y(), -12.0);

    pan.pan_by(-4.0, 8.0);
    assert_eq!(pan.x(), 20.0);
    assert_eq!(pan.y(), -4.0);

    pan.reset();
    assert_eq!(pan, PanOffset::default());
}

#[test]
fn graph_view_new_and_with_zoom_initialize_state_predictably() {
    assert_eq!(GraphView::new(), GraphView::default());

    let view = GraphView::with_zoom(ZoomLevel::new(2.0));

    assert_eq!(view.zoom().value(), 2.0);
    assert_eq!(view.target_zoom().value(), 2.0);
    assert_eq!(view.pan(), PanOffset::default());
}

#[test]
fn graph_view_component_owns_zoom_without_file_watcher() {
    let mut view = GraphView::default();

    assert_eq!(view.zoom().percent(), 100);

    view.zoom_mut().zoom_in();
    assert_eq!(view.zoom().percent(), 100);
    assert_eq!(view.target_zoom().percent(), 125);

    view.reset_zoom();
    assert_eq!(view.target_zoom(), ZoomLevel::default());
}

#[test]
fn graph_view_component_owns_pan_without_file_watcher() {
    let mut view = GraphView::default();

    assert_eq!(view.pan(), PanOffset::default());

    view.pan_mut().pan_by(32.0, 18.0);
    assert_eq!(view.pan(), PanOffset::new(32.0, 18.0));

    view.reset_pan();
    assert_eq!(view.pan(), PanOffset::default());
}

#[allow(dead_code)]
fn graph_view_can_be_embedded_in_any_egui_ui(
    view: &mut GraphView,
    ui: &mut eframe::egui::Ui,
    graph: &Graph,
    layout: &GraphLayout,
) {
    let response: eframe::egui::Response = view.ui(ui, graph, layout);
    let _ = response;
}
