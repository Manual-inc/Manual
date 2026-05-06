use eframe::egui::{self, Color32, Response, RichText, Ui};
use node::Node;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NodeDetailsView;

impl NodeDetailsView {
    pub fn new() -> Self {
        Self
    }

    pub fn ui(&self, ui: &mut Ui, node: &Node) -> Response {
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.monospace(
                            RichText::new(node.id.as_str())
                                .strong()
                                .color(Color32::from_rgb(234, 238, 244)),
                        );
                        ui.label(
                            RichText::new(node.kind.as_str())
                                .color(Color32::from_rgb(127, 180, 255)),
                        );
                    });

                    ui.add_space(6.0);
                    ui.label(RichText::new(&node.description).color(Color32::from_rgb(45, 52, 64)));
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    draw_contract_row(ui, "Inputs", list_text(&node.contract.inputs));
                    draw_contract_row(ui, "Outputs", list_text(&node.contract.outputs));
                    draw_contract_row(
                        ui,
                        "Sandbox",
                        optional_text(node.contract.sandbox.as_deref()),
                    );
                    draw_contract_row(
                        ui,
                        "Runtime",
                        optional_text(node.contract.runtime.as_deref()),
                    );
                    draw_contract_row(ui, "Artifacts", list_text(&node.contract.artifacts));
                    draw_contract_row(
                        ui,
                        "Acceptance",
                        optional_text(node.contract.acceptance.as_deref()),
                    );
                });
            })
            .response
    }
}

fn draw_contract_row(ui: &mut Ui, label: &'static str, value: String) {
    ui.horizontal_wrapped(|ui| {
        ui.add_sized(
            [92.0, 18.0],
            egui::Label::new(
                RichText::new(label)
                    .strong()
                    .color(Color32::from_rgb(66, 76, 90)),
            ),
        );
        ui.label(value);
    });
}

fn list_text(values: &[String]) -> String {
    if values.is_empty() {
        "None".to_string()
    } else {
        values.join(", ")
    }
}

fn optional_text(value: Option<&str>) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("None")
        .to_string()
}
