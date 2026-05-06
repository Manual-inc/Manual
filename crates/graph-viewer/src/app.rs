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
    let watch_dir = watch_dir_for_graph_path(graph_path);

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        handle_watch_event(event, &watched_file, &change_tx, &egui_ctx);
    })?;

    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

fn watch_dir_for_graph_path(graph_path: &Path) -> PathBuf {
    graph_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn handle_watch_event(
    event: notify::Result<Event>,
    watched_file: &Path,
    change_tx: &Sender<()>,
    egui_ctx: &egui::Context,
) {
    match event {
        Ok(event) if event_touches_path(&event, watched_file) => {
            let _ = change_tx.send(());
            egui_ctx.request_repaint();
        }
        _ => {}
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_graph_path(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "manual-graph-viewer-app-{test_name}-{}.json",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ))
    }

    fn write_graph(path: &Path, node_id: &str) {
        fs::write(
            path,
            format!(
                r#"{{
                    "nodes": [{{ "id": "{node_id}" }}],
                    "edges": []
                }}"#
            ),
        )
        .expect("test graph should be written");
    }

    fn app_without_watcher(graph_path: PathBuf, change_rx: Receiver<()>) -> GraphViewerApp {
        GraphViewerApp {
            graph_path,
            graph: None,
            layout: GraphLayout::empty(),
            status: String::new(),
            last_error: None,
            watcher_problem: None,
            change_rx,
            view: GraphView::default(),
            _watcher: None,
        }
    }

    fn run_app_update_frame(app: &mut GraphViewerApp) -> Vec<egui::Shape> {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(640.0, 480.0),
            )),
            ..Default::default()
        });

        let mut frame = eframe::Frame::_new_kittest();
        eframe::App::update(app, &ctx, &mut frame);

        ctx.end_pass()
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect()
    }

    fn assert_text_shape(shapes: &[egui::Shape], expected: &str) {
        assert!(
            shapes.iter().any(|shape| {
                matches!(shape, egui::Shape::Text(text) if text.galley.text() == expected)
            }),
            "expected text shape containing {expected:?}"
        );
    }

    #[test]
    fn reload_graph_loads_graph_layout_and_success_status() {
        let path = unique_graph_path("reload-success");
        fs::write(
            &path,
            r#"
            {
              "nodes": [
                { "id": "source" },
                { "id": "target" }
              ],
              "edges": [
                { "source": "source", "target": "target" }
              ]
            }
            "#,
        )
        .expect("test graph should be written");
        let (_change_tx, change_rx) = mpsc::channel();
        let mut app = app_without_watcher(path.clone(), change_rx);

        app.reload_graph();

        let graph = app.graph.as_ref().expect("graph should load");
        assert_eq!(graph.nodes().len(), 2);
        assert_eq!(graph.edges().len(), 1);
        assert!(app.layout.position("source").is_some());
        assert!(app.layout.position("target").is_some());
        assert_eq!(app.status, "2 nodes, 1 edges loaded");
        assert_eq!(app.last_error, None);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn reload_graph_records_error_and_keeps_previous_graph() {
        let path = unique_graph_path("reload-error");
        write_graph(&path, "stable");
        let (_change_tx, change_rx) = mpsc::channel();
        let mut app = app_without_watcher(path.clone(), change_rx);
        app.reload_graph();

        fs::write(&path, "{ not valid json").expect("invalid graph should be written");
        app.reload_graph();

        assert_eq!(
            app.graph
                .as_ref()
                .expect("previous graph should remain available")
                .nodes()[0]
                .id,
            "stable"
        );
        assert!(app.status.starts_with("Load error: invalid graph JSON:"));
        assert!(matches!(&app.last_error, Some(error) if error.starts_with("invalid graph JSON:")));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn apply_pending_file_changes_reloads_latest_json_once_events_arrive() {
        let path = unique_graph_path("pending-changes");
        write_graph(&path, "before");
        let (change_tx, change_rx) = mpsc::channel();
        let mut app = app_without_watcher(path.clone(), change_rx);
        app.reload_graph();

        write_graph(&path, "after");
        app.apply_pending_file_changes();
        assert_eq!(
            app.graph
                .as_ref()
                .expect("initial graph should remain loaded")
                .nodes()[0]
                .id,
            "before"
        );

        change_tx.send(()).expect("change event should be sent");
        change_tx
            .send(())
            .expect("second change event should be sent");
        app.apply_pending_file_changes();

        assert_eq!(
            app.graph
                .as_ref()
                .expect("updated graph should be loaded")
                .nodes()[0]
                .id,
            "after"
        );
        assert_eq!(app.status, "1 nodes, 0 edges loaded");
        assert_eq!(app.last_error, None);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn update_paints_toolbar_and_graph_canvas() {
        let path = unique_graph_path("update-frame");
        write_graph(&path, "visible");
        let (_change_tx, change_rx) = mpsc::channel();
        let mut app = app_without_watcher(path.clone(), change_rx);
        app.reload_graph();

        let shapes = run_app_update_frame(&mut app);

        assert_text_shape(&shapes, "Manual Graph Viewer");
        assert!(
            shapes.iter().any(
                |shape| matches!(shape, egui::Shape::Circle(circle) if circle.fill == Color32::from_rgb(70, 124, 208))
            ),
            "graph node should be painted by the central graph view"
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn event_touches_path_matches_any_exact_event_path() {
        let graph_path = PathBuf::from("/tmp/manual-graph.json");
        let other_path = PathBuf::from("/tmp/other.json");

        let matching_event = Event::new(notify::EventKind::Any)
            .add_path(other_path.clone())
            .add_path(graph_path.clone());
        assert!(event_touches_path(&matching_event, &graph_path));

        let unrelated_event = Event::new(notify::EventKind::Any).add_path(other_path);
        assert!(!event_touches_path(&unrelated_event, &graph_path));
    }

    #[test]
    fn handle_watch_event_sends_change_only_for_matching_successful_events() {
        let watched_file = PathBuf::from("/tmp/manual-graph.json");
        let other_file = PathBuf::from("/tmp/other.json");
        let (change_tx, change_rx) = mpsc::channel();
        let ctx = egui::Context::default();

        handle_watch_event(
            Ok(Event::new(notify::EventKind::Any).add_path(other_file)),
            &watched_file,
            &change_tx,
            &ctx,
        );
        assert!(change_rx.try_recv().is_err());

        handle_watch_event(
            Ok(Event::new(notify::EventKind::Any).add_path(watched_file.clone())),
            &watched_file,
            &change_tx,
            &ctx,
        );
        assert_eq!(change_rx.try_recv(), Ok(()));

        handle_watch_event(
            Err(notify::Error::generic("watch failed")),
            &watched_file,
            &change_tx,
            &ctx,
        );
        assert!(change_rx.try_recv().is_err());
    }

    #[test]
    fn watch_dir_for_graph_path_uses_parent_or_current_directory() {
        assert_eq!(
            watch_dir_for_graph_path(Path::new("/tmp/manual-graph.json")),
            PathBuf::from("/tmp")
        );
        assert_eq!(
            watch_dir_for_graph_path(Path::new("graphs/manual-graph.json")),
            PathBuf::from("graphs")
        );
        assert_eq!(
            watch_dir_for_graph_path(Path::new("manual-graph.json")),
            PathBuf::from(".")
        );
    }

    #[test]
    fn absolute_path_preserves_absolute_paths_and_resolves_relative_paths() {
        let absolute = PathBuf::from("/tmp/manual-graph.json");
        assert_eq!(absolute_path(absolute.clone()), absolute);

        let relative = PathBuf::from("graph.json");
        assert_eq!(
            absolute_path(relative.clone()),
            std::env::current_dir()
                .expect("current dir should be available")
                .join(relative)
        );
    }

    #[test]
    fn abbreviate_preserves_short_paths_and_truncates_long_paths() {
        assert_eq!(abbreviate("short", 8), "short");
        assert_eq!(abbreviate("abcdefghijklmnop", 8), "abcdefgh...");
        assert_eq!(abbreviate("가나다라마바사", 3), "가나다...");
    }
}
