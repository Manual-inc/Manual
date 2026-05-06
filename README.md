# Manual

Manual is a Rust-based, fast, lightweight runtime for turning repeatable work into agent-executable workflows.

## Core Features

- **Workflow**: describes repeatable work as a graph of nodes, edges, branches, loops, integrations, acceptance criteria, and artifacts.
- **Agent**: treats Codex, Claude Code, scripts, Python programs, and future adapters as executable units that can inspect, reason, calculate, and produce results.
- **Sandbox**: wraps each agent or workflow node in an OS-native execution boundary so file, network, process, and workspace permissions stay explicit.

## What This Repository Contains

This repository currently starts as a compact Rust workspace:

- `core`: shared Manual domain language and workspace metadata.
- `cli`: a thin command entrypoint for inspecting and validating Manual assets.
- `skill`: a bundled agent skill template plus validation entrypoints.
- `app`: an early application shell that depends on the shared core.

The product direction is broader than the first implementation: Manual is intended to become a local-first automation control plane for workflow graphs, agent routing, sandbox policies, run history, cost tracking, artifacts, and a localhost visualization surface.

## Documentation

Detailed English documentation lives in [docs](docs/README.md). The same Markdown source can be converted into a static GitHub Pages site:

```bash
node docs/build.mjs
```

After running the build, publish the `docs/` directory with GitHub Pages.

## Graph Viewer

`manual-graph-viewer` is a native desktop graph viewer built with `egui/eframe`.
It reads graph data from JSON, draws nodes and directed edges, and reloads the
visualization automatically whenever the JSON file is saved.

Use the `-` and `+` toolbar buttons to zoom out and in. Click the percentage
button to reset to `100%`, scroll over the graph canvas to adjust zoom, and drag
the canvas to pan around the graph.

The graph canvas is also reusable as an `egui` component:

```rust
use manual_graph_viewer::{GraphView, circular_layout};

let graph = manual_graph_viewer::Graph::from_json_str(json_source)?;
let layout = circular_layout(&graph);
let mut view = GraphView::default();

egui::CentralPanel::default().show(ctx, |ui| {
    view.zoom_controls(ui);
    view.ui(ui, &graph, &layout);
});
```

Run it with the sample graph:

```sh
cargo run -p manual-graph-viewer -- crates/graph-viewer/examples/sample_graph.json
```

Or pass your own JSON file:

```sh
cargo run -p manual-graph-viewer -- /path/to/graph.json
```

Expected JSON shape:

```json
{
  "nodes": [
    { "id": "a", "label": "Alpha", "color": "#4f8cff" },
    { "id": "b" }
  ],
  "edges": [
    { "source": "a", "target": "b", "label": "links to" }
  ]
}
```

Edges also accept `from` and `to` aliases:

```json
{ "from": "a", "to": "b" }
```
