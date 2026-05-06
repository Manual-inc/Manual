use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::f32::consts::TAU;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;

pub mod app;
pub mod view;

pub use view::GraphView;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphLoadError {
    InvalidJson(String),
    DuplicateNode(String),
    MissingEndpoint {
        edge_index: usize,
        endpoint: &'static str,
        node_id: String,
    },
    Io(String),
}

impl fmt::Display for GraphLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(f, "invalid graph JSON: {message}"),
            Self::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
            Self::MissingEndpoint {
                edge_index,
                endpoint,
                node_id,
            } => write!(
                f,
                "edge {edge_index} references missing {endpoint} node: {node_id}"
            ),
            Self::Io(message) => write!(f, "could not read graph JSON: {message}"),
        }
    }
}

impl std::error::Error for GraphLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Graph {
    pub fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Result<Self, GraphLoadError> {
        validate_graph(&nodes, &edges)?;

        Ok(Self { nodes, edges })
    }

    pub fn from_json_str(source: &str) -> Result<Self, GraphLoadError> {
        let document: GraphDocument = serde_json::from_str(source)
            .map_err(|error| GraphLoadError::InvalidJson(error.to_string()))?;

        let mut nodes = Vec::with_capacity(document.nodes.len());

        for node in document.nodes {
            nodes.push(Node {
                label: node.label.unwrap_or_else(|| node.id.clone()),
                id: node.id,
                color: node.color,
            });
        }

        let mut edges = Vec::with_capacity(document.edges.len());

        for edge in document.edges {
            edges.push(Edge {
                source: edge.source,
                target: edge.target,
                label: edge.label,
            });
        }

        Self::new(nodes, edges)
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}

fn validate_graph(nodes: &[Node], edges: &[Edge]) -> Result<(), GraphLoadError> {
    let mut ids = BTreeSet::new();

    for node in nodes {
        if !ids.insert(node.id.clone()) {
            return Err(GraphLoadError::DuplicateNode(node.id.clone()));
        }
    }

    for (edge_index, edge) in edges.iter().enumerate() {
        if !ids.contains(&edge.source) {
            return Err(GraphLoadError::MissingEndpoint {
                edge_index,
                endpoint: "source",
                node_id: edge.source.clone(),
            });
        }

        if !ids.contains(&edge.target) {
            return Err(GraphLoadError::MissingEndpoint {
                edge_index,
                endpoint: "target",
                node_id: edge.target.clone(),
            });
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphLayout {
    positions: BTreeMap<String, Point>,
}

impl GraphLayout {
    pub fn empty() -> Self {
        Self {
            positions: BTreeMap::new(),
        }
    }

    pub fn position(&self, node_id: &str) -> Option<Point> {
        self.positions.get(node_id).copied()
    }
}

pub fn circular_layout(graph: &Graph) -> GraphLayout {
    let mut positions = BTreeMap::new();
    let node_count = graph.nodes.len();

    if node_count == 0 {
        return GraphLayout { positions };
    }

    if node_count == 1 {
        positions.insert(graph.nodes[0].id.clone(), Point { x: 0.0, y: 0.0 });
        return GraphLayout { positions };
    }

    for (index, node) in graph.nodes.iter().enumerate() {
        let angle = TAU * index as f32 / node_count as f32;
        positions.insert(
            node.id.clone(),
            Point {
                x: angle.cos(),
                y: angle.sin(),
            },
        );
    }

    GraphLayout { positions }
}

pub fn load_graph_file(path: impl AsRef<Path>) -> Result<Graph, GraphLoadError> {
    let source =
        fs::read_to_string(path.as_ref()).map_err(|error| GraphLoadError::Io(error.to_string()))?;
    Graph::from_json_str(&source)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomLevel {
    value: f32,
}

impl ZoomLevel {
    pub const MIN: f32 = 0.25;
    pub const MAX: f32 = 4.0;
    const STEP: f32 = 1.25;

    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(Self::MIN, Self::MAX),
        }
    }

    pub fn value(self) -> f32 {
        self.value
    }

    pub fn percent(self) -> u32 {
        (self.value * 100.0).round() as u32
    }

    pub fn zoom_in(&mut self) {
        *self = Self::new(self.value * Self::STEP);
    }

    pub fn zoom_out(&mut self) {
        *self = Self::new(self.value / Self::STEP);
    }

    pub fn zoom_by_scroll(&mut self, scroll_delta_y: f32) {
        if scroll_delta_y > 0.0 {
            self.zoom_in();
        } else if scroll_delta_y < 0.0 {
            self.zoom_out();
        }
    }

    pub fn scaled_by(self, factor: f32) -> Self {
        Self::new(self.value * factor)
    }

    pub fn lerp_toward(self, target: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self::new(self.value + (target.value - self.value) * amount)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self { value: 1.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothZoom {
    displayed: ZoomLevel,
    target: ZoomLevel,
}

impl SmoothZoom {
    pub const ANIMATION_EPSILON: f32 = 0.002;

    pub fn new(zoom: ZoomLevel) -> Self {
        Self {
            displayed: zoom,
            target: zoom,
        }
    }

    pub fn displayed(self) -> ZoomLevel {
        self.displayed
    }

    pub fn target(self) -> ZoomLevel {
        self.target
    }

    pub fn zoom_in(&mut self) {
        self.target.zoom_in();
    }

    pub fn zoom_out(&mut self) {
        self.target.zoom_out();
    }

    pub fn zoom_by_scroll(&mut self, scroll_delta_y: f32) {
        if scroll_delta_y == 0.0 {
            return;
        }

        let factor = (scroll_delta_y * 0.0025).exp();
        self.target = self.target.scaled_by(factor);
    }

    pub fn reset(&mut self) {
        self.target = ZoomLevel::default();
    }

    pub fn jump_to_target(&mut self) {
        self.displayed = self.target;
    }

    pub fn advance(&mut self, amount: f32) -> bool {
        let next = self.displayed.lerp_toward(self.target, amount);

        if (next.value() - self.target.value()).abs() <= Self::ANIMATION_EPSILON {
            self.displayed = self.target;
            false
        } else {
            self.displayed = next;
            true
        }
    }
}

impl Default for SmoothZoom {
    fn default() -> Self {
        Self::new(ZoomLevel::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanOffset {
    x: f32,
    y: f32,
}

impl PanOffset {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn x(self) -> f32 {
        self.x
    }

    pub fn y(self) -> f32 {
        self.y
    }

    pub fn pan_by(&mut self, delta_x: f32, delta_y: f32) {
        self.x += delta_x;
        self.y += delta_y;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl Default for PanOffset {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Deserialize)]
struct GraphDocument {
    nodes: Vec<NodeDocument>,
    #[serde(default)]
    edges: Vec<EdgeDocument>,
}

#[derive(Debug, Deserialize)]
struct NodeDocument {
    id: String,
    label: Option<String>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EdgeDocument {
    #[serde(alias = "from")]
    source: String,
    #[serde(alias = "to")]
    target: String,
    label: Option<String>,
}
