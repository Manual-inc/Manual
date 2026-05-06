use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use node::{Node, NodeId, NodeIdError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workflow {
    id: String,
    name: String,
    goal: String,
    entry_node: NodeId,
    nodes: Vec<Node>,
    edges: Vec<WorkflowEdge>,
}

impl Workflow {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        goal: impl Into<String>,
        entry_node: impl Into<String>,
        nodes: Vec<Node>,
        edges: Vec<WorkflowEdge>,
    ) -> Result<Self, WorkflowError> {
        let entry_node = NodeId::new(entry_node).map_err(WorkflowError::InvalidEntryNode)?;
        validate_graph(&entry_node, &nodes, &edges)?;

        Ok(Self {
            id: id.into(),
            name: name.into(),
            goal: goal.into(),
            entry_node,
            nodes,
            edges,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn goal(&self) -> &str {
        &self.goal
    }

    pub fn entry_node(&self) -> &NodeId {
        &self.entry_node
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn edges(&self) -> &[WorkflowEdge] {
        &self.edges
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowEdge {
    source: NodeId,
    target: NodeId,
    kind: WorkflowEdgeKind,
    label: Option<String>,
}

impl WorkflowEdge {
    pub fn new(
        kind: WorkflowEdgeKind,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self::try_new(kind, source, target).expect("workflow edge ids should be valid")
    }

    pub fn try_new(
        kind: WorkflowEdgeKind,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, NodeIdError> {
        Ok(Self {
            source: NodeId::new(source)?,
            target: NodeId::new(target)?,
            kind,
            label: None,
        })
    }

    pub fn sequence(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(WorkflowEdgeKind::Sequence, source, target)
    }

    pub fn try_sequence(
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, NodeIdError> {
        Self::try_new(WorkflowEdgeKind::Sequence, source, target)
    }

    pub fn branch(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(WorkflowEdgeKind::Branch, source, target)
    }

    pub fn loop_back(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(WorkflowEdgeKind::LoopBack, source, target)
    }

    pub fn parallel(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(WorkflowEdgeKind::Parallel, source, target)
    }

    pub fn join(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(WorkflowEdgeKind::Join, source, target)
    }

    pub fn error(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(WorkflowEdgeKind::Error, source, target)
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn source(&self) -> &NodeId {
        &self.source
    }

    pub fn target(&self) -> &NodeId {
        &self.target
    }

    pub fn kind(&self) -> WorkflowEdgeKind {
        self.kind
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkflowEdgeKind {
    Sequence,
    Branch,
    LoopBack,
    Parallel,
    Join,
    Error,
}

impl WorkflowEdgeKind {
    pub const ALL: [Self; 6] = [
        Self::Sequence,
        Self::Branch,
        Self::LoopBack,
        Self::Parallel,
        Self::Join,
        Self::Error,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::Branch => "branch",
            Self::LoopBack => "loop_back",
            Self::Parallel => "parallel",
            Self::Join => "join",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for WorkflowEdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkflowEdgeKind {
    type Err = WorkflowEdgeKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "sequence" => Ok(Self::Sequence),
            "branch" => Ok(Self::Branch),
            "loop_back" => Ok(Self::LoopBack),
            "parallel" => Ok(Self::Parallel),
            "join" => Ok(Self::Join),
            "error" => Ok(Self::Error),
            other => Err(WorkflowEdgeKindParseError::Unknown(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowEdgeKindParseError {
    Unknown(String),
}

impl fmt::Display for WorkflowEdgeKindParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(value) => write!(f, "unknown workflow edge kind: {value}"),
        }
    }
}

impl std::error::Error for WorkflowEdgeKindParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    EmptyGraph,
    DuplicateNode(String),
    InvalidEntryNode(NodeIdError),
    MissingEntryNode(String),
    MissingEndpoint {
        edge_index: usize,
        endpoint: &'static str,
        node_id: String,
    },
    SelfLoop {
        edge_index: usize,
        node_id: String,
    },
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => write!(f, "workflow must contain at least one node"),
            Self::DuplicateNode(id) => write!(f, "duplicate node id: {id}"),
            Self::InvalidEntryNode(error) => write!(f, "invalid entry node id: {error}"),
            Self::MissingEntryNode(node_id) => {
                write!(f, "entry node does not exist: {node_id}")
            }
            Self::MissingEndpoint {
                edge_index,
                endpoint,
                node_id,
            } => write!(
                f,
                "edge {edge_index} references missing {endpoint} node: {node_id}"
            ),
            Self::SelfLoop {
                edge_index,
                node_id,
            } => write!(f, "edge {edge_index} cannot point {node_id} to itself"),
        }
    }
}

impl std::error::Error for WorkflowError {}

fn validate_graph(
    entry_node: &NodeId,
    nodes: &[Node],
    edges: &[WorkflowEdge],
) -> Result<(), WorkflowError> {
    if nodes.is_empty() {
        return Err(WorkflowError::EmptyGraph);
    }

    let mut node_ids = BTreeSet::new();

    for node in nodes {
        if !node_ids.insert(node.id.as_str().to_string()) {
            return Err(WorkflowError::DuplicateNode(node.id.to_string()));
        }
    }

    if !node_ids.contains(entry_node.as_str()) {
        return Err(WorkflowError::MissingEntryNode(entry_node.to_string()));
    }

    for (edge_index, edge) in edges.iter().enumerate() {
        if edge.source == edge.target {
            return Err(WorkflowError::SelfLoop {
                edge_index,
                node_id: edge.source.to_string(),
            });
        }

        if !node_ids.contains(edge.source.as_str()) {
            return Err(WorkflowError::MissingEndpoint {
                edge_index,
                endpoint: "source",
                node_id: edge.source.to_string(),
            });
        }

        if !node_ids.contains(edge.target.as_str()) {
            return Err(WorkflowError::MissingEndpoint {
                edge_index,
                endpoint: "target",
                node_id: edge.target.to_string(),
            });
        }
    }

    Ok(())
}
