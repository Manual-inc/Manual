use eframe::egui::{
    self, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
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
