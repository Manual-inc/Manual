use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, vec2,
};

use crate::{Edge, Graph, GraphLayout, Node, PanOffset, SmoothZoom, ZoomLevel};

const NODE_RADIUS: f32 = 18.0;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphView {
    zoom: SmoothZoom,
    pan: PanOffset,
}

impl GraphView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_zoom(zoom: ZoomLevel) -> Self {
        Self {
            zoom: SmoothZoom::new(zoom),
            pan: PanOffset::default(),
        }
    }

    pub fn zoom(&self) -> ZoomLevel {
        self.zoom.displayed()
    }

    pub fn target_zoom(&self) -> ZoomLevel {
        self.zoom.target()
    }

    pub fn zoom_mut(&mut self) -> &mut SmoothZoom {
        &mut self.zoom
    }

    pub fn reset_zoom(&mut self) {
        self.zoom.reset();
    }

    pub fn pan(&self) -> PanOffset {
        self.pan
    }

    pub fn pan_mut(&mut self) -> &mut PanOffset {
        &mut self.pan
    }

    pub fn reset_pan(&mut self) {
        self.pan.reset();
    }

    pub fn zoom_controls(&mut self, ui: &mut Ui) {
        let button_size = vec2(28.0, 24.0);

        if ui
            .add(egui::Button::new("-").min_size(button_size))
            .on_hover_text("Zoom out")
            .clicked()
        {
            self.zoom.zoom_out();
        }

        if ui
            .add(
                egui::Button::new(format!("{}%", self.zoom.displayed().percent()))
                    .min_size(vec2(56.0, 24.0)),
            )
            .on_hover_text("Reset zoom")
            .clicked()
        {
            self.reset_zoom();
        }

        if ui
            .add(egui::Button::new("+").min_size(button_size))
            .on_hover_text("Zoom in")
            .clicked()
        {
            self.zoom.zoom_in();
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, graph: &Graph, layout: &GraphLayout) -> Response {
        let size = ui.available_size_before_wrap();
        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());

        if response.hovered() {
            let scroll_delta_y = ui.ctx().input(|input| input.raw_scroll_delta.y);
            self.zoom.zoom_by_scroll(scroll_delta_y);
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            let drag_delta = ui.ctx().input(|input| input.pointer.delta());
            self.pan.pan_by(drag_delta.x, drag_delta.y);
            ui.ctx().request_repaint();
        }

        if self.zoom.advance(0.18) {
            ui.ctx().request_repaint();
        }

        let painter = ui.painter_at(rect);
        draw_canvas_background(&painter, rect);

        if graph.nodes().is_empty() {
            draw_center_message(&painter, rect, "Empty graph");
            return response;
        }

        let graph_rect = rect.shrink2(Vec2::new(64.0, 64.0));
        let center = graph_rect.center() + vec2(self.pan.x(), self.pan.y());
        let scale =
            graph_rect.width().min(graph_rect.height()) * 0.42 * self.zoom.displayed().value();

        for edge in graph.edges() {
            self.draw_edge(&painter, center, scale, edge, layout);
        }

        for node in graph.nodes() {
            self.draw_node(&painter, center, scale, node, layout);
        }

        response
    }

    pub fn empty_ui(&self, ui: &mut Ui, message: &str) -> Response {
        let size = ui.available_size_before_wrap();
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);

        draw_canvas_background(&painter, rect);
        draw_center_message(&painter, rect, message);

        response
    }

    fn screen_position(
        &self,
        center: Pos2,
        scale: f32,
        node_id: &str,
        layout: &GraphLayout,
    ) -> Option<Pos2> {
        layout
            .position(node_id)
            .map(|point| center + vec2(point.x * scale, point.y * scale))
    }

    fn draw_edge(
        &self,
        painter: &Painter,
        center: Pos2,
        scale: f32,
        edge: &Edge,
        layout: &GraphLayout,
    ) {
        let Some(source) = self.screen_position(center, scale, &edge.source, layout) else {
            return;
        };
        let Some(target) = self.screen_position(center, scale, &edge.target, layout) else {
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

    fn draw_node(
        &self,
        painter: &Painter,
        center: Pos2,
        scale: f32,
        node: &Node,
        layout: &GraphLayout,
    ) {
        let Some(position) = self.screen_position(center, scale, &node.id, layout) else {
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

impl Default for GraphView {
    fn default() -> Self {
        Self {
            zoom: SmoothZoom::default(),
            pan: PanOffset::default(),
        }
    }
}

fn draw_canvas_background(painter: &Painter, rect: Rect) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(15, 18, 22));
    draw_canvas_grid(painter, rect);
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

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{CentralPanel, Context, RawInput, Shape, pos2};

    fn sample_graph() -> Graph {
        Graph::from_json_str(
            r##"
            {
              "nodes": [
                { "id": "source", "label": "Source", "color": "#4f8cff" },
                { "id": "target", "label": "Target", "color": "#41b883" }
              ],
              "edges": [
                { "source": "source", "target": "target", "label": "connects" }
              ]
            }
            "##,
        )
        .expect("sample graph should parse")
    }

    fn run_view_frame(
        view: &mut GraphView,
        graph: &Graph,
        layout: &GraphLayout,
    ) -> (Rect, Vec<Shape>) {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(640.0, 480.0))),
            ..Default::default()
        });

        let mut response_rect = Rect::NOTHING;
        CentralPanel::default().show(&ctx, |ui| {
            response_rect = view.ui(ui, graph, layout).rect;
        });

        let output = ctx.end_pass();
        let shapes = output
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();

        (response_rect, shapes)
    }

    fn run_empty_frame(view: &GraphView) -> (Rect, Vec<Shape>) {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(320.0, 240.0))),
            ..Default::default()
        });

        let mut response_rect = Rect::NOTHING;
        CentralPanel::default().show(&ctx, |ui| {
            response_rect = view.empty_ui(ui, "No graph loaded").rect;
        });

        let output = ctx.end_pass();
        let shapes = output
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();

        (response_rect, shapes)
    }

    fn run_zoom_controls_frame(view: &mut GraphView) -> Vec<Shape> {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(240.0, 80.0))),
            ..Default::default()
        });

        CentralPanel::default().show(&ctx, |ui| {
            view.zoom_controls(ui);
        });

        ctx.end_pass()
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect()
    }

    fn run_arrow_head_frame(source: Pos2, target: Pos2) -> Vec<Shape> {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(160.0, 120.0))),
            ..Default::default()
        });

        CentralPanel::default().show(&ctx, |ui| {
            draw_arrow_head(
                ui.painter(),
                source,
                target,
                Color32::from_rgb(200, 120, 40),
            );
        });

        ctx.end_pass()
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect()
    }

    fn assert_pos_approx(actual: Pos2, expected: Pos2) {
        assert!(
            (actual.x - expected.x).abs() < 0.0001 && (actual.y - expected.y).abs() < 0.0001,
            "expected position {actual:?} to be approximately {expected:?}"
        );
    }

    fn line_length(points: [Pos2; 2]) -> f32 {
        (points[1] - points[0]).length()
    }

    fn find_circle_center(shapes: &[Shape], fill: Color32) -> Pos2 {
        shapes
            .iter()
            .find_map(|shape| match shape {
                Shape::Circle(circle) if circle.fill == fill => Some(circle.center),
                _ => None,
            })
            .expect("expected circle with requested fill color")
    }

    fn edge_lines(shapes: &[Shape]) -> Vec<[Pos2; 2]> {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::LineSegment { points, stroke }
                    if stroke.color == Color32::from_rgb(116, 125, 136) =>
                {
                    Some(*points)
                }
                _ => None,
            })
            .collect()
    }

    fn text_rect(shapes: &[Shape], value: &str) -> Rect {
        shapes
            .iter()
            .find_map(|shape| match shape {
                Shape::Text(text) if text.galley.text() == value => {
                    Some(Rect::from_min_size(text.pos, text.galley.size()))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected text shape containing {value:?}"))
    }

    fn assert_contains_text(shapes: &[Shape], value: &str) {
        let _ = text_rect(shapes, value);
    }

    fn colored_lines(shapes: &[Shape], color: Color32) -> Vec<[Pos2; 2]> {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::LineSegment { points, stroke } if stroke.color == color => Some(*points),
                _ => None,
            })
            .collect()
    }

    fn assert_line_segments_match(actual: &[[Pos2; 2]], expected: &[[Pos2; 2]]) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "unexpected number of line segments"
        );

        let mut matched = vec![false; expected.len()];
        for actual_points in actual {
            let Some((index, _)) = expected
                .iter()
                .enumerate()
                .find(|(index, expected_points)| {
                    !matched[*index]
                        && actual_points.iter().zip(expected_points.iter()).all(
                            |(actual, expected)| {
                                (actual.x - expected.x).abs() < 0.001
                                    && (actual.y - expected.y).abs() < 0.001
                            },
                        )
                })
            else {
                panic!("unexpected line segment: {actual_points:?}");
            };

            matched[index] = true;
        }
    }

    #[test]
    fn ui_allocates_canvas_and_paints_graph_shapes() {
        let graph = sample_graph();
        let layout = crate::circular_layout(&graph);
        let mut view = GraphView::default();

        let (response_rect, shapes) = run_view_frame(&mut view, &graph, &layout);

        assert!(response_rect.width() > 600.0);
        assert!(response_rect.height() > 400.0);
        assert!(
            shapes.iter().any(
                |shape| matches!(shape, Shape::Rect(rect) if rect.fill == Color32::from_rgb(15, 18, 22))
            ),
            "canvas background should be painted"
        );
        assert!(
            shapes
                .iter()
                .filter(|shape| matches!(shape, Shape::LineSegment { stroke, .. } if stroke.color == Color32::from_rgb(25, 30, 36)))
                .count()
                > 20,
            "canvas grid should paint many guide lines"
        );
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape, Shape::Circle(circle) if circle.fill == Color32::from_rgb(79, 140, 255)))
        );
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape, Shape::Circle(circle) if circle.fill == Color32::from_rgb(65, 184, 131)))
        );
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape, Shape::LineSegment { stroke, .. } if stroke.color == Color32::from_rgb(116, 125, 136)))
        );
        assert!(shapes.iter().any(|shape| matches!(shape, Shape::Text(_))));
    }

    #[test]
    fn ui_places_nodes_edges_and_arrow_head_at_expected_positions() {
        let graph = sample_graph();
        let layout = crate::circular_layout(&graph);
        let mut view = GraphView::default();
        view.pan_mut().pan_by(12.0, -8.0);

        let (response_rect, shapes) = run_view_frame(&mut view, &graph, &layout);
        let graph_rect = response_rect.shrink2(Vec2::new(64.0, 64.0));
        let center = graph_rect.center() + vec2(12.0, -8.0);
        let scale = graph_rect.width().min(graph_rect.height()) * 0.42;
        let source = center + vec2(scale, 0.0);
        let target = center + vec2(-scale, 0.0);

        assert_pos_approx(
            find_circle_center(&shapes, Color32::from_rgb(79, 140, 255)),
            source,
        );
        assert_pos_approx(
            find_circle_center(&shapes, Color32::from_rgb(65, 184, 131)),
            target,
        );

        let edge_lines = edge_lines(&shapes);
        assert_eq!(edge_lines.len(), 3, "edge plus two arrow-head strokes");

        let main_edge = edge_lines
            .iter()
            .copied()
            .max_by(|left, right| {
                line_length(*left)
                    .partial_cmp(&line_length(*right))
                    .expect("line lengths should be comparable")
            })
            .expect("main edge line should be present");
        assert_pos_approx(main_edge[0], source);
        assert_pos_approx(main_edge[1], target);

        let tip = target + vec2(18.0, 0.0);
        let arrow_lines = edge_lines
            .into_iter()
            .filter(|points| line_length(*points) < 20.0)
            .collect::<Vec<_>>();
        assert_eq!(arrow_lines.len(), 2);
        for points in arrow_lines {
            assert_pos_approx(points[1], tip);
            assert!((line_length(points) - 13.4164).abs() < 0.001);
        }
    }

    #[test]
    fn zoom_controls_render_expected_buttons_and_current_percent() {
        let mut view = GraphView::with_zoom(ZoomLevel::new(1.25));

        let shapes = run_zoom_controls_frame(&mut view);

        assert_contains_text(&shapes, "-");
        assert_contains_text(&shapes, "125%");
        assert_contains_text(&shapes, "+");
    }

    #[test]
    fn empty_ui_allocates_canvas_and_paints_message() {
        let view = GraphView::default();

        let (response_rect, shapes) = run_empty_frame(&view);

        assert!(response_rect.width() > 300.0);
        assert!(response_rect.height() > 200.0);
        assert!(shapes.iter().any(|shape| matches!(shape, Shape::Text(_))));
    }

    #[test]
    fn ui_scales_node_positions_by_displayed_zoom() {
        let graph = sample_graph();
        let layout = crate::circular_layout(&graph);
        let mut view = GraphView::with_zoom(ZoomLevel::new(2.0));

        let (response_rect, shapes) = run_view_frame(&mut view, &graph, &layout);
        let graph_rect = response_rect.shrink2(Vec2::new(64.0, 64.0));
        let center = graph_rect.center();
        let scale = graph_rect.width().min(graph_rect.height()) * 0.42 * 2.0;

        assert_pos_approx(
            find_circle_center(&shapes, Color32::from_rgb(79, 140, 255)),
            center + vec2(scale, 0.0),
        );
        assert_pos_approx(
            find_circle_center(&shapes, Color32::from_rgb(65, 184, 131)),
            center + vec2(-scale, 0.0),
        );
    }

    #[test]
    fn ui_places_node_and_edge_labels_at_expected_anchors() {
        let graph = sample_graph();
        let layout = crate::circular_layout(&graph);
        let mut view = GraphView::default();

        let (response_rect, shapes) = run_view_frame(&mut view, &graph, &layout);
        let graph_rect = response_rect.shrink2(Vec2::new(64.0, 64.0));
        let center = graph_rect.center();
        let scale = graph_rect.width().min(graph_rect.height()) * 0.42;
        let source = center + vec2(scale, 0.0);
        let target = center + vec2(-scale, 0.0);

        let source_label = text_rect(&shapes, "Source");
        assert_pos_approx(
            source_label.center_top(),
            source + vec2(0.0, NODE_RADIUS + 8.0),
        );

        let target_label = text_rect(&shapes, "Target");
        assert_pos_approx(
            target_label.center_top(),
            target + vec2(0.0, NODE_RADIUS + 8.0),
        );

        let edge_label = text_rect(&shapes, "connects");
        assert_pos_approx(
            edge_label.center_bottom(),
            source.lerp(target, 0.5) + vec2(0.0, -10.0),
        );
    }

    #[test]
    fn arrow_head_uses_perpendicular_offsets_for_diagonal_edges() {
        let source = pos2(10.0, 20.0);
        let target = pos2(110.0, 70.0);

        let shapes = run_arrow_head_frame(source, target);

        let direction = target - source;
        let unit = direction / direction.length();
        let normal = vec2(-unit.y, unit.x);
        let tip = target - unit * NODE_RADIUS;
        let expected = [
            [tip - unit * 12.0 + normal * 6.0, tip],
            [tip - unit * 12.0 - normal * 6.0, tip],
        ];
        assert_line_segments_match(
            &colored_lines(&shapes, Color32::from_rgb(200, 120, 40)),
            &expected,
        );
    }

    #[test]
    fn screen_position_applies_layout_scale_and_pan_center() {
        let graph = Graph::from_json_str(
            r#"
            {
              "nodes": [
                { "id": "east" },
                { "id": "north" },
                { "id": "west" },
                { "id": "south" }
              ],
              "edges": []
            }
            "#,
        )
        .expect("graph should parse");
        let layout = crate::circular_layout(&graph);
        let view = GraphView::default();

        let east = view
            .screen_position(pos2(10.0, 20.0), 5.0, "east", &layout)
            .expect("east should map to a screen position");
        assert_eq!(east, pos2(15.0, 20.0));

        let north = view
            .screen_position(pos2(10.0, 20.0), 5.0, "north", &layout)
            .expect("north should map to a screen position");
        assert!((north.x - 10.0).abs() < 0.0001);
        assert!((north.y - 25.0).abs() < 0.0001);

        assert!(
            view.screen_position(pos2(10.0, 20.0), 5.0, "missing", &layout)
                .is_none()
        );
    }

    #[test]
    fn parse_hex_color_accepts_hash_or_plain_rgb_and_rejects_bad_values() {
        assert_eq!(
            parse_hex_color("#4f8cff"),
            Some(Color32::from_rgb(79, 140, 255))
        );
        assert_eq!(
            parse_hex_color("41b883"),
            Some(Color32::from_rgb(65, 184, 131))
        );
        assert_eq!(parse_hex_color("#12345"), None);
        assert_eq!(parse_hex_color("#nothex"), None);
    }

    #[test]
    fn abbreviate_preserves_short_labels_and_shortens_long_labels() {
        assert_eq!(abbreviate("short", 12), "short");
        assert_eq!(abbreviate("abcdefghijklmnop", 8), "abcdefgh...");
        assert_eq!(abbreviate("가나다라마바사", 3), "가나다...");
    }
}
