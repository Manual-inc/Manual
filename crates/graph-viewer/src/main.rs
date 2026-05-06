use std::path::PathBuf;

use eframe::egui;
use manual_graph_viewer::app::GraphViewerApp;

fn main() -> eframe::Result {
    let graph_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/sample_graph.json")
        });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 760.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Manual Graph Viewer",
        native_options,
        Box::new(move |creation_context| {
            Ok(Box::new(GraphViewerApp::new(
                graph_path.clone(),
                creation_context.egui_ctx.clone(),
            )))
        }),
    )
}
