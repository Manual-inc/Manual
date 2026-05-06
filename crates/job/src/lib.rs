use std::fmt;

use node::{NodeId, NodeIdError};
use workflow::Workflow;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(String);

impl JobId {
    pub fn new(value: impl Into<String>) -> Result<Self, JobIdError> {
        let value = value.into();

        if value.trim().is_empty() {
            return Err(JobIdError::Empty);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(JobIdError::ContainsWhitespace(value));
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

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobIdError {
    Empty,
    ContainsWhitespace(String),
}

impl fmt::Display for JobIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "job id cannot be empty"),
            Self::ContainsWhitespace(value) => {
                write!(f, "job id cannot contain whitespace: {value}")
            }
        }
    }
}

impl std::error::Error for JobIdError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JobStatus {
    Created,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeRunStatus {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
    Skipped,
    WaitingForApproval,
}

impl NodeRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::WaitingForApproval => "waiting_for_approval",
        }
    }

    fn is_success_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Skipped)
    }
}

impl fmt::Display for NodeRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobNode {
    node_id: NodeId,
    status: NodeRunStatus,
    attempts: u32,
}

impl JobNode {
    fn new(node_id: NodeId, status: NodeRunStatus) -> Self {
        Self {
            node_id,
            status,
            attempts: 0,
        }
    }

    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub fn status(&self) -> NodeRunStatus {
        self.status
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    id: JobId,
    workflow_id: String,
    input_json: String,
    status: JobStatus,
    nodes: Vec<JobNode>,
}

impl Job {
    pub fn new(
        id: impl Into<String>,
        workflow: &Workflow,
        input_json: impl Into<String>,
    ) -> Result<Self, JobError> {
        let id = JobId::new(id).map_err(JobError::InvalidJobId)?;
        let nodes = workflow
            .nodes()
            .iter()
            .map(|node| {
                let status = if &node.id == workflow.entry_node() {
                    NodeRunStatus::Ready
                } else {
                    NodeRunStatus::Pending
                };

                JobNode::new(node.id.clone(), status)
            })
            .collect();

        Ok(Self {
            id,
            workflow_id: workflow.id().to_string(),
            input_json: input_json.into(),
            status: JobStatus::Created,
            nodes,
        })
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }

    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    pub fn input_json(&self) -> &str {
        &self.input_json
    }

    pub fn status(&self) -> JobStatus {
        self.status
    }

    pub fn nodes(&self) -> &[JobNode] {
        &self.nodes
    }

    pub fn node_status(&self, node_id: &str) -> Option<NodeRunStatus> {
        self.nodes
            .iter()
            .find(|node| node.node_id.as_str() == node_id)
            .map(JobNode::status)
    }

    pub fn mark_node_ready(&mut self, node_id: impl Into<String>) -> Result<(), JobError> {
        self.transition_node(node_id, NodeRunStatus::Ready)
    }

    pub fn start_node(&mut self, node_id: impl Into<String>) -> Result<(), JobError> {
        self.transition_node(node_id, NodeRunStatus::Running)
    }

    pub fn succeed_node(&mut self, node_id: impl Into<String>) -> Result<(), JobError> {
        self.transition_node(node_id, NodeRunStatus::Succeeded)
    }

    pub fn fail_node(&mut self, node_id: impl Into<String>) -> Result<(), JobError> {
        self.transition_node(node_id, NodeRunStatus::Failed)
    }

    fn transition_node(
        &mut self,
        node_id: impl Into<String>,
        to: NodeRunStatus,
    ) -> Result<(), JobError> {
        let node_id = NodeId::new(node_id).map_err(JobError::InvalidNodeId)?;
        let node = self
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or_else(|| JobError::UnknownNode(node_id.to_string()))?;
        let from = node.status;

        if !valid_node_transition(from, to) {
            return Err(JobError::InvalidNodeTransition {
                node_id: node_id.to_string(),
                from,
                to,
            });
        }

        node.status = to;
        if to == NodeRunStatus::Running {
            node.attempts += 1;
        }
        self.refresh_status();

        Ok(())
    }

    fn refresh_status(&mut self) {
        if self
            .nodes
            .iter()
            .any(|node| node.status == NodeRunStatus::Failed)
        {
            self.status = JobStatus::Failed;
            return;
        }

        if self
            .nodes
            .iter()
            .all(|node| node.status.is_success_terminal())
        {
            self.status = JobStatus::Succeeded;
            return;
        }

        if self.nodes.iter().any(|node| {
            matches!(
                node.status,
                NodeRunStatus::Running | NodeRunStatus::Succeeded | NodeRunStatus::Skipped
            )
        }) {
            self.status = JobStatus::Running;
        }
    }
}

fn valid_node_transition(from: NodeRunStatus, to: NodeRunStatus) -> bool {
    matches!(
        (from, to),
        (NodeRunStatus::Pending, NodeRunStatus::Ready)
            | (NodeRunStatus::Ready, NodeRunStatus::Running)
            | (NodeRunStatus::Running, NodeRunStatus::Succeeded)
            | (NodeRunStatus::Running, NodeRunStatus::Failed)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    InvalidJobId(JobIdError),
    InvalidNodeId(NodeIdError),
    UnknownNode(String),
    InvalidNodeTransition {
        node_id: String,
        from: NodeRunStatus,
        to: NodeRunStatus,
    },
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJobId(error) => write!(f, "invalid job id: {error}"),
            Self::InvalidNodeId(error) => write!(f, "invalid job node id: {error}"),
            Self::UnknownNode(node_id) => write!(f, "job references unknown node: {node_id}"),
            Self::InvalidNodeTransition { node_id, from, to } => {
                write!(f, "node {node_id} cannot transition from {from} to {to}")
            }
        }
    }
}

impl std::error::Error for JobError {}
