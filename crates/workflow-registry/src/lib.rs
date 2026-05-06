use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use node::{Node, NodeContract, NodeIdError, NodeKind, NodeKindParseError};
use serde::Deserialize;
use serde::Serialize;
use workflow::{
    Workflow, WorkflowEdge, WorkflowEdgeKind, WorkflowEdgeKindParseError, WorkflowError,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkflowId(String);

impl WorkflowId {
    pub fn new(value: impl Into<String>) -> Result<Self, WorkflowIdError> {
        let value = value.into();

        if value.trim().is_empty() {
            return Err(WorkflowIdError::Empty);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(WorkflowIdError::ContainsWhitespace(value));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowIdError {
    Empty,
    ContainsWhitespace(String),
}

impl fmt::Display for WorkflowIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "workflow id cannot be empty"),
            Self::ContainsWhitespace(value) => {
                write!(f, "workflow id cannot contain whitespace: {value}")
            }
        }
    }
}

impl std::error::Error for WorkflowIdError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowRegistry {
    workflows: BTreeMap<WorkflowId, Workflow>,
}

impl WorkflowRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, workflow: Workflow) -> Result<(), WorkflowRegistryError> {
        let id = WorkflowId::new(workflow.id()).map_err(WorkflowRegistryError::InvalidId)?;

        if self.workflows.contains_key(&id) {
            return Err(WorkflowRegistryError::DuplicateId(id));
        }

        self.workflows.insert(id, workflow);
        Ok(())
    }

    pub fn resolve(&self, id: impl Into<String>) -> Result<&Workflow, WorkflowRegistryError> {
        let id = WorkflowId::new(id).map_err(WorkflowRegistryError::InvalidId)?;

        self.workflows
            .get(&id)
            .ok_or(WorkflowRegistryError::UnknownId(id))
    }

    pub fn get(&self, id: &WorkflowId) -> Option<&Workflow> {
        self.workflows.get(id)
    }

    pub fn get_mut(&mut self, id: &WorkflowId) -> Option<&mut Workflow> {
        self.workflows.get_mut(id)
    }

    pub fn remove(&mut self, id: &WorkflowId) -> Option<Workflow> {
        self.workflows.remove(id)
    }

    pub fn len(&self) -> usize {
        self.workflows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.workflows.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Workflow> {
        self.workflows.values()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowRegistryError {
    InvalidId(WorkflowIdError),
    DuplicateId(WorkflowId),
    UnknownId(WorkflowId),
}

impl fmt::Display for WorkflowRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(error) => write!(f, "{error}"),
            Self::DuplicateId(id) => write!(f, "duplicate workflow id: {id}"),
            Self::UnknownId(id) => write!(f, "unknown workflow id: {id}"),
        }
    }
}

impl std::error::Error for WorkflowRegistryError {}

pub trait WorkflowStore {
    fn load(&self) -> Result<WorkflowRegistry, WorkflowStoreError>;

    fn save(&self, registry: &WorkflowRegistry) -> Result<(), WorkflowStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWorkflowStore {
    path: PathBuf,
}

impl FileWorkflowStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn for_repository(repository_root: impl AsRef<Path>) -> Self {
        Self::new(
            repository_root
                .as_ref()
                .join(".manual")
                .join("workflows.toml"),
        )
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl WorkflowStore for FileWorkflowStore {
    fn load(&self) -> Result<WorkflowRegistry, WorkflowStoreError> {
        if !self.path.exists() {
            return Ok(WorkflowRegistry::new());
        }

        let contents = fs::read_to_string(&self.path)
            .map_err(|error| WorkflowStoreError::io(&self.path, error))?;
        let document: StoredWorkflowDocument =
            toml::from_str(&contents).map_err(|error| WorkflowStoreError::Decode {
                path: self.path.clone(),
                message: error.to_string(),
            })?;

        document.into_registry()
    }

    fn save(&self, registry: &WorkflowRegistry) -> Result<(), WorkflowStoreError> {
        let document = StoredWorkflowDocument::from_registry(registry);
        let contents =
            toml::to_string_pretty(&document).map_err(|error| WorkflowStoreError::Encode {
                message: error.to_string(),
            })?;

        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| WorkflowStoreError::io(parent, error))?;
            }
        }

        fs::write(&self.path, contents).map_err(|error| WorkflowStoreError::io(&self.path, error))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStoreError {
    Io { path: PathBuf, message: String },
    Decode { path: PathBuf, message: String },
    Encode { message: String },
    Registry(WorkflowRegistryError),
    Workflow(WorkflowError),
    NodeId(NodeIdError),
    NodeKind(NodeKindParseError),
    EdgeKind(WorkflowEdgeKindParseError),
}

impl WorkflowStoreError {
    fn io(path: &Path, error: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for WorkflowStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(
                    f,
                    "workflow store I/O failed at {}: {message}",
                    path.display()
                )
            }
            Self::Decode { path, message } => write!(
                f,
                "workflow store failed to decode {}: {message}",
                path.display()
            ),
            Self::Encode { message } => write!(f, "workflow store failed to encode: {message}"),
            Self::Registry(error) => write!(f, "{error}"),
            Self::Workflow(error) => write!(f, "{error}"),
            Self::NodeId(error) => write!(f, "{error}"),
            Self::NodeKind(error) => write!(f, "{error}"),
            Self::EdgeKind(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for WorkflowStoreError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredWorkflowDocument {
    workflows: Vec<StoredWorkflow>,
}

impl StoredWorkflowDocument {
    fn from_registry(registry: &WorkflowRegistry) -> Self {
        Self {
            workflows: registry.iter().map(StoredWorkflow::from_workflow).collect(),
        }
    }

    fn into_registry(self) -> Result<WorkflowRegistry, WorkflowStoreError> {
        let mut registry = WorkflowRegistry::new();

        for workflow in self.workflows {
            registry
                .insert(workflow.into_workflow()?)
                .map_err(WorkflowStoreError::Registry)?;
        }

        Ok(registry)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredWorkflow {
    id: String,
    name: String,
    goal: String,
    entry_node: String,
    nodes: Vec<StoredNode>,
    edges: Vec<StoredWorkflowEdge>,
}

impl StoredWorkflow {
    fn from_workflow(workflow: &Workflow) -> Self {
        Self {
            id: workflow.id().to_string(),
            name: workflow.name().to_string(),
            goal: workflow.goal().to_string(),
            entry_node: workflow.entry_node().as_str().to_string(),
            nodes: workflow.nodes().iter().map(StoredNode::from_node).collect(),
            edges: workflow
                .edges()
                .iter()
                .map(StoredWorkflowEdge::from_edge)
                .collect(),
        }
    }

    fn into_workflow(self) -> Result<Workflow, WorkflowStoreError> {
        let nodes = self
            .nodes
            .into_iter()
            .map(StoredNode::into_node)
            .collect::<Result<Vec<_>, _>>()?;
        let edges = self
            .edges
            .into_iter()
            .map(StoredWorkflowEdge::into_edge)
            .collect::<Result<Vec<_>, _>>()?;

        Workflow::new(self.id, self.name, self.goal, self.entry_node, nodes, edges)
            .map_err(WorkflowStoreError::Workflow)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredNode {
    id: String,
    kind: String,
    description: String,
    contract: StoredNodeContract,
}

impl StoredNode {
    fn from_node(node: &Node) -> Self {
        Self {
            id: node.id.as_str().to_string(),
            kind: node.kind.as_str().to_string(),
            description: node.description.clone(),
            contract: StoredNodeContract::from_contract(&node.contract),
        }
    }

    fn into_node(self) -> Result<Node, WorkflowStoreError> {
        let kind = self
            .kind
            .parse::<NodeKind>()
            .map_err(WorkflowStoreError::NodeKind)?;
        let mut node =
            Node::new(self.id, kind, self.description).map_err(WorkflowStoreError::NodeId)?;
        node.contract = self.contract.into_contract();

        Ok(node)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredNodeContract {
    inputs: Vec<String>,
    outputs: Vec<String>,
    sandbox: Option<String>,
    runtime: Option<String>,
    artifacts: Vec<String>,
    acceptance: Option<String>,
}

impl StoredNodeContract {
    fn from_contract(contract: &NodeContract) -> Self {
        Self {
            inputs: contract.inputs.clone(),
            outputs: contract.outputs.clone(),
            sandbox: contract.sandbox.clone(),
            runtime: contract.runtime.clone(),
            artifacts: contract.artifacts.clone(),
            acceptance: contract.acceptance.clone(),
        }
    }

    fn into_contract(self) -> NodeContract {
        NodeContract {
            inputs: self.inputs,
            outputs: self.outputs,
            sandbox: self.sandbox,
            runtime: self.runtime,
            artifacts: self.artifacts,
            acceptance: self.acceptance,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredWorkflowEdge {
    source: String,
    target: String,
    kind: String,
    label: Option<String>,
}

impl StoredWorkflowEdge {
    fn from_edge(edge: &WorkflowEdge) -> Self {
        Self {
            source: edge.source().as_str().to_string(),
            target: edge.target().as_str().to_string(),
            kind: edge.kind().as_str().to_string(),
            label: edge.label().map(str::to_string),
        }
    }

    fn into_edge(self) -> Result<WorkflowEdge, WorkflowStoreError> {
        let kind = self
            .kind
            .parse::<WorkflowEdgeKind>()
            .map_err(WorkflowStoreError::EdgeKind)?;
        let edge = WorkflowEdge::try_new(kind, self.source, self.target)
            .map_err(WorkflowStoreError::NodeId)?;

        Ok(match self.label {
            Some(label) => edge.with_label(label),
            None => edge,
        })
    }
}
