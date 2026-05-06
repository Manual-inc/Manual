use eframe::egui;
use node::{Node, NodeKind};
use node_gui::NodeDetailsView;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([560.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Manual Node Details",
        native_options,
        Box::new(|_creation_context| Ok(Box::<NodeDetailsExample>::default())),
    )
}

struct NodeDetailsExample {
    node: Node,
    details: NodeDetailsView,
}

impl eframe::App for NodeDetailsExample {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Node Details");
            ui.add_space(12.0);
            self.details.ui(ui, &self.node);
        });
    }
}

impl Default for NodeDetailsExample {
    fn default() -> Self {
        Self {
            node: sample_node(),
            details: NodeDetailsView::default(),
        }
    }
}

fn sample_node() -> Node {
    Node::new(
        "inspect",
        NodeKind::LlmTask,
        "Inspect symptoms, logs, and likely code paths.",
    )
    .expect("sample node should be valid")
    .with_input("voc_ticket")
    .with_input("runtime_logs")
    .with_output("root_cause_hypothesis")
    .with_sandbox("read-only")
    .with_runtime("codex")
    .with_artifact("inspection-notes.md")
    .with_acceptance("Likely code paths are identified.")
}
