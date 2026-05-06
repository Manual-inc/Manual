# Manual

Manual is a Rust-based, fast, lightweight runtime for turning repeatable work into agent-executable workflows.

## Core Features

- **Workflow**: describes repeatable work as a graph of nodes, edges, branches, loops, integrations, acceptance criteria, and artifacts.
- **Agent**: treats Codex, Claude Code, scripts, Python programs, and future adapters as executable units that can inspect, reason, calculate, and produce results.
- **Sandbox**: wraps each agent or workflow node in an OS-native execution boundary so file, network, process, and workspace permissions stay explicit.

## What This Repository Contains

This repository currently starts as a compact Rust workspace:

- `crates/core`: shared Manual domain language and workspace metadata.
- `crates/node`: workflow node types and node contract metadata.
- `crates/workflow`: workflow graph types that combine nodes with directed edges and validation.
- `crates/workflow-registry`: workflow template registration, lookup, and file-backed storage for the `workflow` graph model.
- `crates/cli`: a thin command entrypoint for inspecting and validating Manual assets.
- `crates/skill`: a bundled agent skill template plus validation entrypoints.
- `crates/agent`: adapters for controlling external agent CLIs as JSONL streams.
- `crates/agent-gui`: reusable native agent profile components for managing agent runtime metadata.
- `crates/script`: a Rust-script runner that passes input JSON into a user-defined Rust `main` function, supports Cargo dependencies, and captures output.
- `crates/script-gui`: reusable `egui` components and a native manager app for listing, viewing, registering, editing, and deleting scripts.
- `crates/sandbox`: cross-platform policy models and OS-specific sandbox execution plans.
- `crates/sandbox-registry`: named sandbox definitions and lookup logic backed by the `sandbox` policy model.
- `crates/runtime`: the execution layer that turns input, sandbox policy, and a script or agent target into a captured run.
- `crates/app`: an early application shell that depends on the shared core.
- `crates/graph-viewer`: reusable native graph visualization primitives and a JSON graph viewer.
- `crates/workflow-gui`: reusable native workflow management components for listing, viewing, creating, editing, deleting, and graphing workflows.
- `crates/node-gui`: a reusable `egui` node details component for inspecting node identity, description, and contract metadata.

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

## Workflow GUI

`workflow-gui` adapts validated `workflow::Workflow` values into the
`manual-graph-viewer` graph model and provides reusable `egui` components for
workflow registry list, detail, create, edit, and delete pages.

Run the bundled sample workflow GUI through the app:

```sh
cargo run -p app -- --workflow-gui
```

## Script GUI

`script-gui` provides a reusable `egui` component for managing
`script-registry` entries. Embed `ScriptRegistryPanel` in another app by passing
it a mutable `ScriptRegistry`, or run the file-backed manager through the app
shell:

```sh
cargo run -p app -- --script-gui
```

## Agent GUI

`agent-gui` provides a reusable `AgentManagerPanel` for agent profile list,
detail, register, edit, and delete pages. It owns only GUI state, so app shells
can embed it with their own persistence layer.

Open the app directly on the agent manager:

```sh
cargo run -p app -- --agent-gui
```

## Node GUI

`node-gui` provides a portable `egui` component for showing detailed
`node::Node` metadata without owning application state or file loading.

Run the sample node details window:

```sh
cargo run -p node-gui --example node_details
```

```rust
use node_gui::NodeDetailsView;

let details = NodeDetailsView::default();

egui::SidePanel::right("node_details").show(ctx, |ui| {
    details.ui(ui, selected_node);
});
```
