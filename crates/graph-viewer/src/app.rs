use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui::{self, Color32, Ui};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{circular_layout, load_graph_file, Graph, GraphLayout, GraphView};

pub struct GraphViewerApp {
    graph_path: PathBuf,
    graph: Option<Graph>,
    layout: GraphLayout,
    status: String,
    last_error: Option<String>,
    watcher_problem: Option<String>,
    change_rx: Receiver<()>,
    view: GraphView,
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
            view: GraphView::default(),
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

            self.view.zoom_controls(ui);
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

    fn draw_graph(&mut self, ui: &mut Ui) {
        let Some(graph) = &self.graph else {
            self.view.empty_ui(ui, "No graph loaded");
            return;
        };

        self.view.ui(ui, graph, &self.layout);
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

fn abbreviate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{shortened}...")
    } else {
        shortened
    }
}
