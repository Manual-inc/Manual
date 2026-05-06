use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use manual_graph_viewer::{
    circular_layout, load_graph_file, Graph, GraphLayout, GraphLoadError, GraphView, PanOffset,
    SmoothZoom, ZoomLevel,
};

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
fn smooth_zoom_uses_continuous_scroll_targets() {
    let mut zoom = SmoothZoom::default();

    zoom.zoom_by_scroll(12.0);

    assert!(zoom.target().value() > 1.0);
    assert!(zoom.target().value() < 1.25);
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
