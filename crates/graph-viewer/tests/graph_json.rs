use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use manual_graph_viewer::{Graph, GraphLoadError, circular_layout, load_graph_file};

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
