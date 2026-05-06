use std::collections::BTreeMap;
use std::fmt;

use workflow::Workflow;

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
