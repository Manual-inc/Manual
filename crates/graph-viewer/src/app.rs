use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2, vec2,
};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{Edge, Graph, GraphLayout, Node, circular_layout, load_graph_file};

const NODE_RADIUS: f32 = 18.0;

pub struct GraphViewerApp {
    graph_path: PathBuf,
    graph: Option<Graph>,
    layout: GraphLayout,
    status: String,
    last_error: Option<String>,
    watcher_problem: Option<String>,
    change_rx: Receiver<()>,
    _watcher: Option<RecommendedWatcher>,
}

impl GraphViewerApp {
    pub fn new(graph_path: impl Into<PathBuf>, egui_ctx: egui::Context) -> Self {
        let graph_path = absolute_path(graph_path.into());
        let (change_tx, change_rx) = mpsc::channel();
        let watcher = watch_graph_file(&graph_path, change_tx, egui_ctx);
        let watcher_problem = watcher.as_ref().err().map(ToString::to_string);

        let mut app = Self {
            graph_path,
            graph: None,
            layout: GraphLayout::empty(),
            status: String::new(),
            last_error: None,
            watcher_problem,
            change_rx,
            _watcher: watcher.ok(),
        };

        app.reload_graph();
        app
    }

    fn reload_graph(&mut self) {
        match load_graph_file(&self.graph_path) {
            Ok(graph) => {
                self.layout = circular_layout(&graph);
                self.status = format!(
                    "{} nodes, {} edges loaded",
                    graph.nodes().len(),
                    graph.edges().len()
                );
                self.graph = Some(graph);
                self.last_error = None;
            }
            Err(error) => {
                self.status = format!("Load error: {error}");
                self.last_error = Some(error.to_string());
            }
        }
    }

    fn apply_pending_file_changes(&mut self) {
        let mut changed = false;
        while self.change_rx.try_recv().is_ok() {
            changed = true;
        }

        if changed {
            self.reload_graph();
        }
    }

    fn draw_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Manual Graph Viewer");
            ui.separator();
            ui.monospace(abbreviate(&self.graph_path.display().to_string(), 96))
                .on_hover_text(self.graph_path.display().to_string());
            ui.separator();

            if ui.button("Reload").clicked() {
                self.reload_graph();
            }

            let status_color = if self.last_error.is_some() {
                Color32::from_rgb(220, 82, 72)
            } else {
                Color32::from_rgb(90, 145, 94)
            };
            ui.colored_label(status_color, &self.status);

            if let Some(problem) = &self.watcher_problem {
                ui.colored_label(
                    Color32::from_rgb(218, 154, 74),
                    format!("Watcher unavailable: {problem}"),
                );
            }
        });
    }

    fn draw_graph(&self, ui: &mut Ui) {
        let size = ui.available_size_before_wrap();
        let (rect, _response) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 0.0, Color32::from_rgb(15, 18, 22));
        draw_canvas_grid(&painter, rect);

        let Some(graph) = &self.graph else {
            draw_center_message(&painter, rect, "No graph loaded");
            return;
        };

        if graph.nodes().is_empty() {
            draw_center_message(&painter, rect, "Empty graph");
            return;
        }

        let graph_rect = rect.shrink2(Vec2::new(64.0, 64.0));
        let center = graph_rect.center();
        let scale = graph_rect.width().min(graph_rect.height()) * 0.42;

        for edge in graph.edges() {
            self.draw_edge(&painter, center, scale, edge);
        }

        for node in graph.nodes() {
            self.draw_node(&painter, center, scale, node);
        }
    }

    fn screen_position(&self, center: Pos2, scale: f32, node_id: &str) -> Option<Pos2> {
        self.layout
            .position(node_id)
            .map(|point| center + vec2(point.x * scale, point.y * scale))
    }

    fn draw_edge(&self, painter: &Painter, center: Pos2, scale: f32, edge: &Edge) {
        let Some(source) = self.screen_position(center, scale, &edge.source) else {
            return;
        };
        let Some(target) = self.screen_position(center, scale, &edge.target) else {
            return;
        };

        let stroke = Stroke::new(1.5, Color32::from_rgb(116, 125, 136));
        painter.line_segment([source, target], stroke);
        draw_arrow_head(painter, source, target, stroke.color);

        if let Some(label) = &edge.label {
            let midpoint = source.lerp(target, 0.5);
            painter.text(
                midpoint + vec2(0.0, -10.0),
                Align2::CENTER_BOTTOM,
                abbreviate(label, 28),
                FontId::proportional(12.0),
                Color32::from_rgb(196, 203, 214),
            );
        }
    }

    fn draw_node(&self, painter: &Painter, center: Pos2, scale: f32, node: &Node) {
        let Some(position) = self.screen_position(center, scale, &node.id) else {
            return;
        };

        let fill = node
            .color
            .as_deref()
            .and_then(parse_hex_color)
            .unwrap_or_else(|| Color32::from_rgb(70, 124, 208));

        painter.circle_filled(position, NODE_RADIUS, fill);
        painter.circle_stroke(
            position,
            NODE_RADIUS,
            Stroke::new(2.0, Color32::from_rgb(234, 238, 244)),
        );
        painter.text(
            position + vec2(0.0, NODE_RADIUS + 8.0),
            Align2::CENTER_TOP,
            abbreviate(&node.label, 24),
            FontId::proportional(13.0),
            Color32::from_rgb(234, 238, 244),
        );
    }
}

impl eframe::App for GraphViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_pending_file_changes();

        egui::TopBottomPanel::top("graph_viewer_toolbar")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                self.draw_toolbar(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.draw_graph(ui);
            });
    }
}

fn watch_graph_file(
    graph_path: &Path,
    change_tx: Sender<()>,
    egui_ctx: egui::Context,
) -> notify::Result<RecommendedWatcher> {
    let watched_file = graph_path.to_path_buf();
    let watch_dir = graph_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut watcher =
        notify::recommended_watcher(move |event: notify::Result<Event>| match event {
            Ok(event) if event_touches_path(&event, &watched_file) => {
                let _ = change_tx.send(());
                egui_ctx.request_repaint();
            }
            _ => {}
        })?;

    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

fn event_touches_path(event: &Event, graph_path: &Path) -> bool {
    event.paths.iter().any(|path| path == graph_path)
}

fn absolute_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    std::env::current_dir()
        .map(|current_dir| current_dir.join(&path))
        .unwrap_or(path)
}

fn draw_canvas_grid(painter: &Painter, rect: Rect) {
    let stroke = Stroke::new(1.0, Color32::from_rgb(25, 30, 36));
    let step = 48.0;
    let mut x = rect.left();
    while x <= rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            stroke,
        );
        x += step;
    }

    let mut y = rect.top();
    while y <= rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            stroke,
        );
        y += step;
    }
}

fn draw_center_message(painter: &Painter, rect: Rect, message: &str) {
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        message,
        FontId::proportional(18.0),
        Color32::from_rgb(184, 193, 205),
    );
}

fn draw_arrow_head(painter: &Painter, source: Pos2, target: Pos2, color: Color32) {
    let direction = target - source;
    let length = direction.length();
    if length <= NODE_RADIUS {
        return;
    }

    let unit = direction / length;
    let normal = vec2(-unit.y, unit.x);
    let tip = target - unit * NODE_RADIUS;
    let left = tip - unit * 12.0 + normal * 6.0;
    let right = tip - unit * 12.0 - normal * 6.0;
    let stroke = Stroke::new(1.5, color);

    painter.line_segment([left, tip], stroke);
    painter.line_segment([right, tip], stroke);
}

fn parse_hex_color(value: &str) -> Option<Color32> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&value[0..2], 16).ok()?;
    let green = u8::from_str_radix(&value[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(Color32::from_rgb(red, green, blue))
}

fn abbreviate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{shortened}...")
    } else {
        shortened
    }
}
